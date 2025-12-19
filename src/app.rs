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
    BalanceAnalysisState, BatchProgressMessage, BatchState, FilterState, ImageState, 
    IntegrityState, RebalanceState, Settings, UIState, UndoManager, UndoState,
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
    pub rebalance: RebalanceState,
    pub integrity: IntegrityState,
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
            rebalance: RebalanceState::new(),
            integrity: IntegrityState::new(),
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

    /// Helper method to adjust current index if out of bounds
    fn adjust_current_index(&mut self) {
        if self.current_index >= self.dataset.get_image_files().len() && self.current_index > 0 {
            self.current_index = self.dataset.get_image_files().len().saturating_sub(1);
        }
    }

    /// Helper method to reload dataset and refresh current state
    /// 
    /// ⚠️ **DEPRECATED**: Use `reload_dataset_with_filters()` instead to ensure
    /// filters are automatically reapplied after dataset changes.
    #[deprecated(note = "Use reload_dataset_with_filters() instead")]
    fn reload_and_refresh(&mut self, reset_zoom: bool) {
        self.dataset.load_current_split();
        self.adjust_current_index();
        self.reset_image_state(reset_zoom);
        self.parse_label_file();
    }

    /// Reload the dataset and automatically reapply filters if active
    /// 
    /// This is the recommended method to use whenever the dataset changes
    /// (e.g., after delete, undo, batch operations) to ensure filtered
    /// indices stay in sync with the dataset.
    fn reload_dataset_with_filters(&mut self, reset_zoom: bool) {
        self.reload_and_refresh(reset_zoom);
        
        // Automatically reapply filters if active to keep filtered_indices in sync
        if self.filter.is_active() {
            info!("Auto-reapplying filters after dataset reload");
            self.apply_filters();
        }
    }

    /// Reload the dataset and reapply filters without automatic navigation
    /// 
    /// Use this when you want to control navigation yourself after reload,
    /// such as during delete operations where position should be preserved.
    fn reload_dataset_without_navigation(&mut self, reset_zoom: bool) {
        self.reload_and_refresh(reset_zoom);
        
        // Reapply filters but skip navigation - caller will handle position
        if self.filter.is_active() {
            info!("Reapplying filters after dataset reload (skipping auto-navigation)");
            self.apply_filters_no_navigation();
        }
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
        
        // Reapply filters if active (using manual approach since we don't reload here)
        if self.filter.is_active() {
            info!("Reapplying filters after loading new dataset");
            self.apply_filters();
        }
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
        
        // Reapply filters if active (using manual approach since we don't reload here)
        if self.filter.is_active() {
            info!("Reapplying filters after changing split");
            self.apply_filters();
        }
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

        // Save current filtered position if filters are active
        let current_filtered_pos = if self.filter.is_active() {
            self.filter.get_filtered_index(self.current_index)
        } else {
            None
        };
        info!("Current filtered position: {:?}", current_filtered_pos);

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
        self.reload_dataset_without_navigation(false);
        info!(
            "After reload, dataset has {} images",
            self.dataset.get_image_files().len()
        );

        // Navigate to appropriate position after deletion
        if let Some(filtered_pos) = current_filtered_pos {
            // Filters were active - maintain position in filtered list
            info!(
                "Filters active, restoring position. Previous filtered pos: {}",
                filtered_pos
            );

            // Try to stay at the same filtered position (which now shows the next image)
            // If we were at the end, go to the new last position
            let new_filtered_count = self.filter.filtered_count();
            let target_filtered_pos = if filtered_pos >= new_filtered_count {
                new_filtered_count.saturating_sub(1)
            } else {
                filtered_pos
            };

            if let Some(actual_index) = self.filter.get_actual_index(target_filtered_pos) {
                info!(
                    "Navigating to actual index {} (filtered pos {})",
                    actual_index, target_filtered_pos
                );
                self.current_index = actual_index;
                self.reset_image_state(false);
                self.parse_label_file();
            }
        } else {
            // No filters active - just ensure index is valid
            // The adjust_current_index call in reload already handled this
            self.parse_label_file();
        }

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

            // Reload the dataset and reapply filters if needed
            self.reload_dataset_with_filters(false);

            // Try to find the restored image and navigate to it
            if let Some(index) = self
                .dataset
                .get_image_files()
                .iter()
                .position(|p| p == &undo_state.image_path)
            {
                self.current_index = index;
                self.reset_image_state(false);
                self.parse_label_file();
            }
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
            self.reload_dataset_with_filters(false);
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
        self.apply_filters_internal(true);
    }

    /// Apply filters without automatic navigation (used during delete operations)
    fn apply_filters_no_navigation(&mut self) {
        self.apply_filters_internal(false);
    }

    /// Internal method to apply filters with optional navigation
    fn apply_filters_internal(&mut self, navigate: bool) {
        let image_files = self.dataset.get_image_files();
        self.filter.total_count = image_files.len();
        self.filter.filtered_indices =
            core::filter::apply_filters(image_files, &self.filter.criteria);

        info!(
            "Filters applied: {} / {} images match criteria",
            self.filter.filtered_indices.len(),
            self.filter.total_count
        );

        if navigate {
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

    /// Calculate a rebalance plan based on current balance stats
    pub fn calculate_rebalance_plan(&mut self, config: core::analysis::RebalanceConfig) {
        if let Some(stats) = &self.balance.results {
            if let Some(dataset_path) = self.dataset.dataset_path() {
                info!("Calculating rebalance plan for {:?}", config.category);
                
                let plan = core::analysis::calculate_rebalance_plan(
                    dataset_path,
                    &config,
                    stats,
                );

                if plan.is_empty() {
                    info!("No images need to be moved");
                    self.rebalance.error_message = Some(
                        "No images need to be moved - already balanced!".to_string()
                    );
                } else {
                    info!("Plan calculated: {} images to move", plan.len());
                    self.rebalance.plan = Some(plan);
                    self.rebalance.show_preview = true;
                    self.rebalance.error_message = None;
                }
                
                self.rebalance.config = Some(config);
            } else {
                warn!("No dataset loaded, cannot calculate rebalance");
            }
        } else {
            warn!("No balance analysis results, cannot calculate rebalance");
        }
    }

    /// Execute the current rebalance plan
    pub fn execute_rebalance(&mut self) {
        if let (Some(plan), Some(dataset_path)) = 
            (&self.rebalance.plan, self.dataset.dataset_path().cloned()) 
        {
            info!("Executing rebalance plan with {} actions", plan.len());
            
            self.rebalance.is_active = true;
            self.rebalance.show_preview = false;
            self.rebalance.progress = Some((0, plan.len()));

            // Create channel for progress updates
            let (tx, rx) = channel();
            self.rebalance.progress_receiver = Some(rx);

            // Create cancellation flag
            let cancel_flag = Arc::new(AtomicBool::new(false));
            self.rebalance.cancel_flag = Some(cancel_flag.clone());

            // Clone plan for background thread
            let plan_clone = plan.clone();

            // Spawn background thread
            thread::spawn(move || {
                info!("Background thread started for rebalance execution");
                core::analysis::execute_rebalance_plan(
                    &dataset_path,
                    &plan_clone,
                    Some(tx),
                    Some(cancel_flag),
                );
                info!("Background thread completed rebalance execution");
            });
        } else {
            warn!("No rebalance plan to execute");
        }
    }

    /// Cancel ongoing rebalance execution
    pub fn cancel_rebalance(&mut self) {
        info!("User requested rebalance cancellation");
        if let Some(flag) = &self.rebalance.cancel_flag {
            flag.store(true, Ordering::Relaxed);
        }
    }

    /// Undo the last rebalance operation
    pub fn undo_rebalance(&mut self) {
        if !self.rebalance.can_undo() {
            warn!("No rebalance to undo");
            return;
        }

        if let Some(results) = self.rebalance.last_results.take() {
            info!("Undoing rebalance with {} results", results.len());

            self.rebalance.is_active = true;
            let success_count = results.iter().filter(|r| r.success).count();
            self.rebalance.progress = Some((0, success_count));

            // Create channel for progress updates
            let (tx, rx) = channel();
            self.rebalance.progress_receiver = Some(rx);

            // Create cancellation flag
            let cancel_flag = Arc::new(AtomicBool::new(false));
            self.rebalance.cancel_flag = Some(cancel_flag.clone());

            // Spawn background thread
            thread::spawn(move || {
                info!("Background thread started for rebalance undo");
                core::analysis::undo_rebalance(&results, Some(tx), Some(cancel_flag));
                info!("Background thread completed rebalance undo");
            });
        }
    }

    /// Close rebalance dialogs and reset state
    pub fn close_rebalance(&mut self) {
        self.rebalance.reset();
    }

    /// Calculate a global rebalance plan for all splits
    pub fn calculate_global_rebalance(&mut self) {
        info!("calculate_global_rebalance called!");
        if let Some(dataset_path) = self.dataset.dataset_path() {
            info!("Calculating global rebalance plan for all splits");
            
            let config = core::analysis::GlobalRebalanceConfig::default();
            let plan = core::analysis::calculate_global_rebalance_plan(
                dataset_path,
                &config,
            );

            if plan.is_empty() {
                info!("No moves possible - splits cannot be improved by redistribution");
                self.rebalance.error_message = Some(
                    "ℹ️ No redistribution possible. All splits have similar ratios - consider adding background images to reach target 10% BG.".to_string()
                );
            } else {
                info!("Global plan calculated: {} total moves in {} groups", 
                    plan.total_moves, plan.moves.len());
                self.rebalance.global_plan = Some(plan);
                self.rebalance.is_global = true;
                self.rebalance.show_preview = true;
                self.rebalance.error_message = None;
            }
        } else {
            warn!("No dataset loaded, cannot calculate global rebalance");
        }
    }

    /// Execute the current global rebalance plan
    pub fn execute_global_rebalance(&mut self) {
        if let (Some(plan), Some(dataset_path)) = 
            (&self.rebalance.global_plan, self.dataset.dataset_path().cloned()) 
        {
            info!("Executing global rebalance plan with {} total moves", plan.total_moves);
            
            self.rebalance.is_active = true;
            self.rebalance.show_preview = false;
            self.rebalance.progress = Some((0, plan.total_moves));

            let (tx, rx) = channel();
            self.rebalance.progress_receiver = Some(rx);

            let cancel_flag = Arc::new(AtomicBool::new(false));
            self.rebalance.cancel_flag = Some(cancel_flag.clone());

            let plan_clone = plan.clone();
            thread::spawn(move || {
                info!("Background thread started for global rebalance execution");
                core::analysis::execute_global_rebalance_plan(
                    &dataset_path,
                    &plan_clone,
                    Some(tx),
                    Some(cancel_flag),
                );
                info!("Background thread completed global rebalance execution");
            });
        } else {
            warn!("No global rebalance plan to execute");
        }
    }

    // =========================================================================
    // DATA INTEGRITY METHODS
    // =========================================================================

    /// Start analyzing dataset integrity in background thread
    pub fn analyze_integrity(&mut self) {
        if let Some(dataset_path) = self.dataset.dataset_path() {
            info!("Starting integrity analysis for current split");
            self.integrity.analyzing = true;
            self.integrity.current_progress = 0;
            self.integrity.total_files = 0;
            self.integrity.results = None;
            self.integrity.selected_images_without_labels.clear();
            self.integrity.selected_labels_without_images.clear();

            let (tx, rx) = channel();
            self.integrity.progress_receiver = Some(rx);

            let cancel_flag = Arc::new(AtomicBool::new(false));
            self.integrity.cancel_flag = Some(cancel_flag.clone());

            let dataset_path = dataset_path.clone();
            let split = self.dataset.current_split();

            thread::spawn(move || {
                info!("Background thread started for integrity analysis");
                core::analysis::analyze_dataset_integrity_with_progress(
                    &dataset_path,
                    split,
                    Some(tx),
                    Some(cancel_flag),
                );
                info!("Background thread completed integrity analysis");
            });
        } else {
            warn!("No dataset loaded, cannot analyze integrity");
        }
    }

    /// Cancel ongoing integrity analysis
    pub fn cancel_integrity_analysis(&mut self) {
        info!("User requested integrity analysis cancellation");
        if let Some(flag) = &self.integrity.cancel_flag {
            flag.store(true, Ordering::Relaxed);
        }
    }

    /// Delete selected integrity issues (orphaned files)
    pub fn delete_selected_integrity_issues(&mut self) {
        if let Some(ref stats) = self.integrity.results {
            let mut deleted_count = 0;
            let mut errors = Vec::new();

            // Delete selected images without labels
            let selected_images: Vec<usize> = self.integrity.selected_images_without_labels
                .iter()
                .copied()
                .collect();
            
            for idx in selected_images.iter().rev() {
                if let Some(issue) = stats.images_without_labels.get(*idx) {
                    if issue.path.exists() {
                        match fs::remove_file(&issue.path) {
                            Ok(_) => {
                                info!("Deleted orphaned image: {:?}", issue.path);
                                deleted_count += 1;
                            }
                            Err(e) => {
                                error!("Failed to delete {:?}: {}", issue.path, e);
                                errors.push(format!("{}: {}", issue.path.display(), e));
                            }
                        }
                    }
                }
            }

            // Delete selected labels without images
            let selected_labels: Vec<usize> = self.integrity.selected_labels_without_images
                .iter()
                .copied()
                .collect();
            
            for idx in selected_labels.iter().rev() {
                if let Some(issue) = stats.labels_without_images.get(*idx) {
                    if issue.path.exists() {
                        match fs::remove_file(&issue.path) {
                            Ok(_) => {
                                info!("Deleted orphaned label: {:?}", issue.path);
                                deleted_count += 1;
                            }
                            Err(e) => {
                                error!("Failed to delete {:?}: {}", issue.path, e);
                                errors.push(format!("{}: {}", issue.path.display(), e));
                            }
                        }
                    }
                }
            }

            info!("Deleted {} orphaned files", deleted_count);

            if !errors.is_empty() {
                self.integrity.error_message = Some(format!(
                    "Failed to delete {} files. See log for details.",
                    errors.len()
                ));
            }

            // Clear selections
            self.integrity.selected_images_without_labels.clear();
            self.integrity.selected_labels_without_images.clear();

            // Re-run integrity analysis to refresh the list
            self.analyze_integrity();
            
            // Reload dataset in case we deleted images
            self.reload_dataset_with_filters(false);
        }
    }

    /// Delete all integrity issues
    pub fn delete_all_integrity_issues(&mut self) {
        if let Some(ref stats) = self.integrity.results {
            // Select all issues
            for i in 0..stats.images_without_labels.len() {
                self.integrity.selected_images_without_labels.insert(i);
            }
            for i in 0..stats.labels_without_images.len() {
                self.integrity.selected_labels_without_images.insert(i);
            }
            // Then delete them
            self.delete_selected_integrity_issues();
        }
    }
}

impl eframe::App for DatasetCleanerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for batch processing updates
        let mut complete_stats = None;
        if let Some(receiver) = &self.batch.progress_receiver {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    BatchProgressMessage::Progress(stats) => {
                        self.batch.stats = Some(stats);
                    }
                    BatchProgressMessage::Complete(stats)
                    | BatchProgressMessage::Cancelled(stats) => {
                        complete_stats = Some(stats);
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

            // Reload dataset and refresh state (same for both cancelled and completed)
            self.reload_dataset_with_filters(false);
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
                    self.balance.results = Some(stats.clone());
                    self.balance.analyzing = false;
                    self.balance.progress_receiver = None;
                    self.balance.cancel_flag = None;
                    
                    // Cache best destinations for rebalance buttons
                    if let Some(dataset_path) = self.dataset.dataset_path() {
                        let current_split = self.dataset.current_split();
                        let target_ratios = core::analysis::TargetRatios {
                            player_ratio: self.config.target_player_ratio,
                            background_ratio: self.config.target_background_ratio,
                            hardcase_ratio: self.config.target_hardcase_ratio,
                        };
                        
                        self.balance.cached_best_bg_dest = core::analysis::find_best_destination_split(
                            dataset_path,
                            current_split,
                            core::analysis::ImageCategory::Background,
                            &target_ratios,
                        );
                        self.balance.cached_best_player_dest = core::analysis::find_best_destination_split(
                            dataset_path,
                            current_split,
                            core::analysis::ImageCategory::CTOnly,
                            &target_ratios,
                        );
                    }
                }
                core::analysis::BalanceProgressMessage::Cancelled(stats) => {
                    self.balance.results = Some(stats);
                    self.balance.analyzing = false;
                    self.balance.progress_receiver = None;
                    self.balance.cancel_flag = None;
                }
            }
        }

        // Poll for integrity analysis updates
        let mut integrity_messages = Vec::new();
        if let Some(receiver) = &self.integrity.progress_receiver {
            while let Ok(message) = receiver.try_recv() {
                integrity_messages.push(message);
            }
        }

        // Process integrity messages outside of the borrow
        for message in integrity_messages {
            match message {
                core::analysis::IntegrityProgressMessage::Progress {
                    current,
                    total,
                    stats,
                } => {
                    self.integrity.current_progress = current;
                    self.integrity.total_files = total;
                    self.integrity.results = Some(stats);
                }
                core::analysis::IntegrityProgressMessage::Complete(stats) => {
                    self.integrity.results = Some(stats);
                    self.integrity.analyzing = false;
                    self.integrity.progress_receiver = None;
                    self.integrity.cancel_flag = None;
                }
                core::analysis::IntegrityProgressMessage::Cancelled(stats) => {
                    self.integrity.results = Some(stats);
                    self.integrity.analyzing = false;
                    self.integrity.progress_receiver = None;
                    self.integrity.cancel_flag = None;
                }
            }
        }

        // Poll for rebalance progress updates
        let mut rebalance_complete = None;
        let mut rebalance_error = None;
        if let Some(receiver) = &self.rebalance.progress_receiver {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    core::analysis::RebalanceProgressMessage::Progress { current, total, last_moved } => {
                        self.rebalance.progress = Some((current, total));
                        self.rebalance.last_moved = Some(last_moved);
                    }
                    core::analysis::RebalanceProgressMessage::Complete { success_count, failed_count, results } => {
                        rebalance_complete = Some((true, success_count, failed_count, results));
                    }
                    core::analysis::RebalanceProgressMessage::Cancelled { completed_count, results } => {
                        rebalance_complete = Some((false, completed_count, 0, results));
                    }
                    core::analysis::RebalanceProgressMessage::Error(msg) => {
                        rebalance_error = Some(msg);
                    }
                }
            }
        }

        // Handle error outside of borrow
        if let Some(msg) = rebalance_error {
            self.rebalance.error_message = Some(msg);
            self.rebalance.is_active = false;
            self.rebalance.progress_receiver = None;
            self.rebalance.cancel_flag = None;
        }

        // Handle rebalance completion outside of borrow
        if let Some((_completed, success_count, _failed_count, results)) = rebalance_complete {
            self.rebalance.is_active = false;
            self.rebalance.progress_receiver = None;
            self.rebalance.cancel_flag = None;
            self.rebalance.show_result = true;
            
            // Store results for potential undo
            if success_count > 0 {
                self.rebalance.last_results = Some(results);
            }
            
            // Reload the dataset to reflect changes
            #[allow(deprecated)]
            self.reload_and_refresh(false);
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
        ui::render_rebalance_dialog(self, ctx);

        ui::handle_keyboard_shortcuts(self, ctx);
    }
}
