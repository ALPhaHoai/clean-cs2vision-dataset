use eframe::egui;
use egui::ColorImage;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Instant;
use tracing::{debug, error, info, warn};

use crate::config::AppConfig;
use crate::core;
use crate::core::dataset::{parse_label_file, Dataset, DatasetSplit};
use crate::navigation::Navigator;
use crate::state::{
    BalanceAnalysisState, BatchProgressMessage, BatchState, FilterState, ImageState, Settings,
    UIState, UndoManager, UndoState,
};
use crate::ui;

#[derive(Default, Clone)]
pub struct BatchStats {
    pub total_scanned: usize,
    pub total_deleted: usize,
    pub current_progress: usize,
}

pub struct DatasetCleanerApp {
    // Core application state
    pub dataset: Dataset,
    pub current_index: usize,
    pub config: AppConfig,
    pub settings: Settings,
    pub undo_manager: UndoManager,

    // Organized state modules
    pub image: ImageState,
    pub ui: UIState,
    pub batch: BatchState,
    pub balance: BalanceAnalysisState,
    pub filter: FilterState,
}

impl Default for DatasetCleanerApp {
    fn default() -> Self {
        let config = AppConfig::default();
        let settings = Settings::load();
        let mut dataset = Dataset::new();

        // Prefer last dataset path from settings, fallback to config default
        let dataset_path = settings
            .last_dataset_path
            .clone()
            .or_else(|| Some(config.default_dataset_path.clone()))
            .unwrap();

        if dataset_path.exists() {
            info!("Loading dataset from: {:?}", dataset_path);
            dataset.load(dataset_path.clone());

            // Restore last split if available
            let split = match settings.last_split.as_str() {
                "val" => DatasetSplit::Val,
                "test" => DatasetSplit::Test,
                _ => DatasetSplit::Train,
            };
            if split != DatasetSplit::Train {
                dataset.change_split(split);
            }
        } else {
            warn!("Dataset path does not exist: {:?}", dataset_path);
        }

        // Restore last image index, clamped to valid range
        let num_images = dataset.get_image_files().len();
        let current_index = if num_images > 0 {
            settings.last_image_index.min(num_images - 1)
        } else {
            0
        };

        // Clone filter criteria before moving settings into app
        let filter_criteria = settings.filter_criteria.clone();

        let mut app = Self {
            dataset,
            current_index,
            config,
            settings,
            undo_manager: UndoManager::new(),
            image: ImageState::new(),
            ui: UIState::new(),
            batch: BatchState::new(),
            balance: BalanceAnalysisState::new(),
            filter: FilterState {
                criteria: filter_criteria,
                ..FilterState::new()
            },
        };

        // Parse label for the current image if dataset was loaded
        if !app.dataset.get_image_files().is_empty() {
            app.parse_label_file();

            // Apply filters if they were restored from settings
            if app.filter.is_active() {
                app.apply_filters();
            }
        }

        app
    }
}

impl DatasetCleanerApp {
    /// Helper method to reset image-related state
    fn reset_image_state(&mut self, reset_zoom: bool) {
        self.image.reset(reset_zoom);
    }

    pub fn load_dataset(&mut self, path: PathBuf) {
        info!("Loading dataset from: {:?}", path);
        self.dataset.load(path.clone());
        self.current_index = 0;
        self.reset_image_state(false);
        // Parse label file for the first image
        self.parse_label_file();
        info!(
            "Dataset loaded successfully, total images: {}",
            self.dataset.get_image_files().len()
        );

        // Save dataset path to settings
        self.settings.last_dataset_path = Some(path);
        self.settings.save();
    }

    pub fn change_split(&mut self, new_split: DatasetSplit) {
        info!("Changing dataset split to: {:?}", new_split);
        self.dataset.change_split(new_split);
        self.current_index = 0;
        self.reset_image_state(false);
        // Parse label file for the first image
        self.parse_label_file();
        debug!(
            "Split changed, current images count: {}",
            self.dataset.get_image_files().len()
        );

        // Save split to settings
        self.settings.last_split = new_split.as_str().to_string();
        self.settings.save();
    }

    pub fn load_current_image(&mut self, ctx: &egui::Context) {
        if self.dataset.get_image_files().is_empty() {
            return;
        }

        let img_path = &self.dataset.get_image_files()[self.current_index];
        info!("Attempting to load image: {:?}", img_path);

        // Clear any previous error
        self.image.load_error = None;

        match image::open(img_path) {
            Ok(img) => {
                info!("Successfully opened image, converting to RGBA8");
                let img_rgb = img.to_rgba8();
                let size = [img_rgb.width() as _, img_rgb.height() as _];
                let pixels = img_rgb.as_flat_samples();

                let color_image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

                let texture =
                    ctx.load_texture("current_image", color_image, egui::TextureOptions::LINEAR);

                // Calculate dominant color
                self.image.dominant_color = Self::calculate_dominant_color(&img);

                self.image.texture = Some(texture);
                info!("Image loaded successfully");
            }
            Err(e) => {
                let error_msg = format!("Failed to load image: {}", e);
                error!("{:?}: {}", img_path, error_msg);
                self.image.load_error = Some(error_msg);
            }
        }
    }

    fn calculate_dominant_color(img: &image::DynamicImage) -> Option<egui::Color32> {
        core::image::calculate_dominant_color(img).map(|(r, g, b)| egui::Color32::from_rgb(r, g, b))
    }

    pub fn parse_label_file(&mut self) {
        if self.dataset.get_image_files().is_empty() {
            self.image.label = None;
            return;
        }

        let img_path = &self.dataset.get_image_files()[self.current_index];

        // Get corresponding label file path using file_operations module
        let label_path = match core::operations::get_label_path_for_image(img_path) {
            Some(path) => path,
            None => {
                self.image.label = None;
                return;
            }
        };

        // Parse label file using the dedicated module
        self.image.label = parse_label_file(&label_path);
    }

    pub fn delete_current_image(&mut self) {
        info!("=== DELETE_CURRENT_IMAGE CALLED ===");

        if self.dataset.get_image_files().is_empty() {
            info!("ERROR: Dataset is empty, returning early");
            return;
        }
        info!(
            "Dataset has {} images",
            self.dataset.get_image_files().len()
        );
        info!("Current index: {}", self.current_index);

        let img_path = &self.dataset.get_image_files()[self.current_index].clone();
        info!("Image path to delete: {:?}", img_path);

        // Get image filename for display
        let image_filename = img_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        info!("Image filename: {}", image_filename);

        // Get corresponding label file path
        let label_path = core::operations::get_label_path_for_image(img_path);
        info!("Label path: {:?}", label_path);

        // Create temp directory in system temp
        let temp_dir = std::env::temp_dir().join("yolo_dataset_cleaner_undo");
        info!("Temp dir: {:?}", temp_dir);

        if let Err(e) = fs::create_dir_all(&temp_dir) {
            error!("ERROR creating temp directory: {}", e);
            return;
        }
        info!("Temp directory created successfully");

        // Generate unique temp paths using timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        // Delete image and label using file_operations module
        let (temp_image_path, temp_label_path) =
            match core::operations::delete_image_with_label(img_path, &temp_dir, timestamp) {
                Ok(paths) => paths,
                Err(e) => {
                    error!("Failed to delete image: {}", e);
                    return;
                }
            };

        // Create undo state and push to undo manager
        info!("Creating undo state and adding to undo manager");
        self.undo_manager.push_delete(UndoState {
            image_path: img_path.clone(),
            label_path,
            image_filename: image_filename.clone(),
            deleted_at: Instant::now(),
            temp_image_path,
            temp_label_path,
        });

        // Reload the current split to refresh the file list
        info!("Reloading current split");
        self.dataset.load_current_split();
        info!(
            "After reload, dataset has {} images",
            self.dataset.get_image_files().len()
        );

        // Adjust index if needed
        if self.current_index >= self.dataset.get_image_files().len() && self.current_index > 0 {
            self.current_index -= 1;
            info!("Adjusted index to {}", self.current_index);
        }

        // Clear current texture
        self.reset_image_state(false);
        info!("Cleared current state");

        // Parse the label for the new current image
        self.parse_label_file();
        info!("=== DELETE_CURRENT_IMAGE COMPLETED SUCCESSFULLY ===");
    }

    pub fn undo_delete(&mut self) {
        if let Some(undo_state) = self.undo_manager.undo() {
            info!(
                "Attempting to undo delete for: {}",
                undo_state.image_filename
            );

            // Restore image and label files using file_operations module
            if let Err(e) = core::operations::restore_image_with_label(
                &undo_state.temp_image_path,
                &undo_state.image_path,
                &undo_state.temp_label_path,
                &undo_state.label_path,
            ) {
                error!("Error restoring files: {}", e);
                return;
            }
            debug!("Files successfully restored");

            // Reload the dataset to refresh file list
            self.dataset.load_current_split();

            // Try to find the restored image and navigate to it
            if let Some(index) = self
                .dataset
                .get_image_files()
                .iter()
                .position(|p| p == &undo_state.image_path)
            {
                self.current_index = index;
            }

            // Clear current texture to force reload
            self.reset_image_state(false);

            // Parse the label for the current/restored image
            self.parse_label_file();
        }
    }

    pub fn redo_delete(&mut self) {
        if let Some(undo_state) = self.undo_manager.redo() {
            info!(
                "Attempting to redo delete for: {}",
                undo_state.image_filename
            );

            // Re-delete using file_operations module, but we need to manually handle it
            // since delete_image_with_label expects the original paths
            // Re-delete: move files back to temp location using move_file
            if let Err(e) =
                core::operations::move_file(&undo_state.image_path, &undo_state.temp_image_path)
            {
                error!("Error re-deleting image: {}", e);
                return;
            }

            // Re-delete label file if it exists
            if let (Some(orig_label), Some(temp_label)) =
                (&undo_state.label_path, &undo_state.temp_label_path)
            {
                if orig_label.exists() {
                    if let Err(e) = core::operations::move_file(orig_label, temp_label) {
                        error!("Error re-deleting label: {}", e);
                    }
                }
            }

            // Reload the dataset to refresh file list
            self.dataset.load_current_split();

            // Adjust index if needed
            if self.current_index >= self.dataset.get_image_files().len() && self.current_index > 0
            {
                self.current_index -= 1;
            }

            // Clear current texture
            self.reset_image_state(false);

            // Parse the label for the new current image
            self.parse_label_file();
        }
    }

    fn navigate_to(&mut self, new_index: usize) {
        if new_index != self.current_index {
            self.current_index = new_index;
            self.reset_image_state(true);
            self.parse_label_file();

            // Save image index to settings
            self.settings.last_image_index = self.current_index;
            self.settings.save();

            info!("Navigated to image index: {}", self.current_index);
        }
    }

    pub fn next_image(&mut self) {
        let nav = Navigator::new(self.dataset.get_image_files().len());
        if let Some(new_index) = nav.next(self.current_index, &self.filter) {
            self.navigate_to(new_index);
        }
    }

    pub fn prev_image(&mut self) {
        let nav = Navigator::new(self.dataset.get_image_files().len());
        if let Some(new_index) = nav.prev(self.current_index, &self.filter) {
            self.navigate_to(new_index);
        }
    }

    pub fn jump_to_first(&mut self) {
        let nav = Navigator::new(self.dataset.get_image_files().len());
        if let Some(new_index) = nav.first(&self.filter) {
            self.navigate_to(new_index);
        }
    }

    pub fn jump_to_last(&mut self) {
        let nav = Navigator::new(self.dataset.get_image_files().len());
        if let Some(new_index) = nav.last(&self.filter) {
            self.navigate_to(new_index);
        }
    }

    pub fn jump_by_offset(&mut self, offset: isize) {
        let nav = Navigator::new(self.dataset.get_image_files().len());
        if let Some(new_index) = nav.jump_by_offset(self.current_index, offset, &self.filter) {
            self.navigate_to(new_index);
        }
    }

    pub fn toggle_fullscreen(&mut self) {
        self.ui.fullscreen_mode = !self.ui.fullscreen_mode;
        info!("Fullscreen mode toggled: {}", self.ui.fullscreen_mode);
    }

    /// Apply current filter criteria and recompute filtered indices
    pub fn apply_filters(&mut self) {
        let image_files = self.dataset.get_image_files();
        self.filter.total_count = image_files.len();
        self.filter.filtered_indices =
            core::filter::apply_filters(image_files, &self.filter.criteria);

        info!(
            "Filters applied: {} / {} images match criteria",
            self.filter.filtered_indices.len(),
            self.filter.total_count
        );

        // If current index is not in filtered list, navigate to first filtered image
        if self.filter.is_active() && !self.filter.filtered_indices.is_empty() {
            if let Some(filtered_idx) = self.filter.get_filtered_index(self.current_index) {
                // Current image is in filtered list, navigate to it (updates display)
                if let Some(actual_index) = self.filter.get_actual_index(filtered_idx) {
                    self.navigate_to(actual_index);
                }
            } else {
                // Current image not in filtered list, go to first filtered image
                if let Some(actual_index) = self.filter.get_actual_index(0) {
                    self.navigate_to(actual_index);
                }
            }
        }

        // Save filter settings
        self.settings.filter_criteria = self.filter.criteria.clone();
        self.settings.save();
    }

    /// Clear all active filters
    pub fn clear_filters(&mut self) {
        self.filter.clear();

        // Save filter settings
        self.settings.filter_criteria = self.filter.criteria.clone();
        self.settings.save();

        info!("Filters cleared");
    }

    pub fn process_black_images(&mut self) {
        if self.dataset.get_image_files().is_empty() {
            warn!("No images to process for black image removal");
            return;
        }

        info!(
            "Starting batch processing to remove black images, total images: {}",
            self.dataset.get_image_files().len()
        );
        // Set batch processing flag
        self.batch.processing = true;

        // Initialize stats
        let stats = BatchStats::default();
        self.batch.stats = Some(stats);

        // Create a channel for progress updates
        let (tx, rx) = channel::<BatchProgressMessage>();
        self.batch.progress_receiver = Some(rx);

        // Create cancellation flag
        let cancel_flag = Arc::new(AtomicBool::new(false));
        self.batch.cancel_flag = Some(cancel_flag.clone());

        // Clone the data needed for the background thread
        let image_files: Vec<PathBuf> = self.dataset.get_image_files().clone();

        // Spawn background thread to process images
        thread::spawn(move || {
            info!("Background thread started for batch image processing");
            let mut stats = BatchStats::default();

            for (idx, img_path) in image_files.iter().enumerate() {
                // Check for cancellation
                if cancel_flag.load(Ordering::Relaxed) {
                    warn!(
                        "Batch processing cancelled by user at image {}/{}",
                        idx,
                        image_files.len()
                    );
                    let _ = tx.send(BatchProgressMessage::Cancelled(stats));
                    return;
                }

                stats.current_progress = idx + 1;
                stats.total_scanned += 1;

                // Load and analyze image
                if let Ok(img) = image::open(img_path) {
                    if let Some((r, g, b)) = core::image::calculate_dominant_color(&img) {
                        if core::image::is_near_black((r, g, b)) {
                            // Delete image file
                            if fs::remove_file(img_path).is_ok() {
                                // Delete corresponding label file using file_operations
                                if let Some(label_path) =
                                    core::operations::get_label_path_for_image(img_path)
                                {
                                    if label_path.exists() {
                                        let _ = fs::remove_file(&label_path);
                                    }
                                }
                                stats.total_deleted += 1;
                            }
                        }
                    }
                }

                // Send progress update every 10 images or on last image
                if idx % 10 == 0 || idx == image_files.len() - 1 {
                    let _ = tx.send(BatchProgressMessage::Progress(stats.clone()));
                }
            }

            // Send completion message
            info!(
                "Batch processing complete. Scanned: {}, Deleted: {}",
                stats.total_scanned, stats.total_deleted
            );
            let _ = tx.send(BatchProgressMessage::Complete(stats));
        });
    }

    pub fn cancel_batch_processing(&mut self) {
        info!("User requested batch processing cancellation");
        if let Some(flag) = &self.batch.cancel_flag {
            flag.store(true, Ordering::Relaxed);
        }
    }

    pub fn analyze_balance(&mut self) {
        if let Some(dataset_path) = self.dataset.dataset_path() {
            info!("Starting balance analysis for current split");
            self.balance.analyzing = true;
            self.balance.show_dialog = true;
            self.balance.current_progress = 0;
            self.balance.total_images = 0;

            // Create a channel for progress updates
            let (tx, rx) = channel();
            self.balance.progress_receiver = Some(rx);

            // Create cancellation flag
            let cancel_flag = Arc::new(AtomicBool::new(false));
            self.balance.cancel_flag = Some(cancel_flag.clone());

            // Clone the data needed for the background thread
            let dataset_path = dataset_path.clone();
            let split = self.dataset.current_split();

            // Spawn background thread to analyze
            thread::spawn(move || {
                info!("Background thread started for balance analysis");
                let _stats = core::analysis::analyze_dataset_with_progress(
                    &dataset_path,
                    split,
                    Some(tx),
                    Some(cancel_flag),
                );
                info!("Background thread completed balance analysis");
            });
        } else {
            warn!("No dataset loaded, cannot analyze balance");
        }
    }

    pub fn cancel_balance_analysis(&mut self) {
        info!("User requested balance analysis cancellation");
        if let Some(flag) = &self.balance.cancel_flag {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

impl eframe::App for DatasetCleanerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for batch processing updates
        let mut complete_stats = None;
        let mut cancelled = false;
        if let Some(receiver) = &self.batch.progress_receiver {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    BatchProgressMessage::Progress(stats) => {
                        self.batch.stats = Some(stats);
                    }
                    BatchProgressMessage::Complete(stats) => {
                        complete_stats = Some(stats);
                    }
                    BatchProgressMessage::Cancelled(stats) => {
                        complete_stats = Some(stats);
                        cancelled = true;
                    }
                }
            }
        }

        // Handle completion or cancellation outside of the borrow
        if let Some(stats) = complete_stats {
            self.batch.stats = Some(stats);
            self.batch.processing = false;
            self.batch.progress_receiver = None;
            self.batch.cancel_flag = None;

            // Don't close the dialog immediately if cancelled, let user see the stats
            if !cancelled {
                // Reload the dataset to refresh file list
                self.dataset.load_current_split();

                // Adjust current index if needed
                if self.current_index >= self.dataset.get_image_files().len()
                    && self.current_index > 0
                {
                    self.current_index = self.dataset.get_image_files().len().saturating_sub(1);
                }

                // Clear current texture to force reload
                self.reset_image_state(false);

                // Parse the label for the current image
                self.parse_label_file();
            } else {
                // For cancelled operations, still reload but keep showing the dialog
                self.dataset.load_current_split();

                // Adjust current index if needed
                if self.current_index >= self.dataset.get_image_files().len()
                    && self.current_index > 0
                {
                    self.current_index = self.dataset.get_image_files().len().saturating_sub(1);
                }

                // Clear current texture to force reload
                self.reset_image_state(false);

                // Parse the label for the current image
                self.parse_label_file();
            }
        }

        // Poll for balance analysis updates
        let mut balance_messages = Vec::new();
        if let Some(receiver) = &self.balance.progress_receiver {
            while let Ok(message) = receiver.try_recv() {
                balance_messages.push(message);
            }
        }

        // Process balance messages outside of the borrow
        for message in balance_messages {
            match message {
                core::analysis::BalanceProgressMessage::Progress {
                    current,
                    total,
                    stats,
                } => {
                    self.balance.current_progress = current;
                    self.balance.total_images = total;
                    self.balance.results = Some(stats);
                }
                core::analysis::BalanceProgressMessage::Complete(stats) => {
                    self.balance.results = Some(stats);
                    self.balance.analyzing = false;
                    self.balance.progress_receiver = None;
                    self.balance.cancel_flag = None;
                }
                core::analysis::BalanceProgressMessage::Cancelled(stats) => {
                    self.balance.results = Some(stats);
                    self.balance.analyzing = false;
                    self.balance.progress_receiver = None;
                    self.balance.cancel_flag = None;
                }
            }
        }

        ui::render_top_panel(self, ctx);
        ui::render_bottom_panel(self, ctx);

        if !self.dataset.get_image_files().is_empty() {
            ui::render_label_panel(self, ctx);
        }

        ui::render_central_panel(self, ctx);
        ui::render_batch_delete_confirmation(self, ctx);
        ui::render_batch_progress(self, ctx);
        ui::render_toast_notification(self, ctx);
        ui::render_filter_dialog(self, ctx);
        ui::render_balance_dialog(self, ctx);

        ui::handle_keyboard_shortcuts(self, ctx);
    }
}
