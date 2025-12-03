use eframe::egui;
use egui::{ColorImage, TextureHandle};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Instant;
use tracing::{debug, error, info, warn};

mod log_formatter;
use log_formatter::BracketedFormatter;

mod label_parser;
use label_parser::{parse_label_file, LabelInfo};

mod dataset;
use dataset::{Dataset, DatasetSplit};

mod config;
use config::AppConfig;

mod image_analysis;

mod ui;

mod settings;
use settings::Settings;

#[derive(Default, Clone)]
pub struct BatchStats {
    pub total_scanned: usize,
    pub total_deleted: usize,
    pub current_progress: usize,
}

enum BatchProgressMessage {
    Progress(BatchStats),
    Complete(BatchStats),
    Cancelled(BatchStats),
}

#[derive(Clone)]
pub struct UndoState {
    pub image_path: PathBuf,
    pub label_path: Option<PathBuf>,
    pub image_filename: String,
    pub deleted_at: Instant,
    pub temp_image_path: PathBuf,
    pub temp_label_path: Option<PathBuf>,
}

fn main() -> Result<(), eframe::Error> {
    // Create logs directory
    let log_dir = std::env::current_dir().unwrap().join("logs");
    fs::create_dir_all(&log_dir).expect("Failed to create logs directory");

    // Create log file with timestamp
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let log_filename = format!("dataset_cleaner_{}.log", timestamp);
    let log_path = log_dir.join(&log_filename);

    // Create file appender
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .expect("Failed to create log file");

    // Initialize tracing subscriber with file output
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let file_layer = fmt::layer()
        .event_format(BracketedFormatter)
        .with_writer(std::sync::Mutex::new(file))
        .with_ansi(false); // Disable ANSI colors in file

    let stdout_layer = fmt::layer()
        .event_format(BracketedFormatter)
        .with_writer(std::io::stdout);

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // Keep our app at trace level, but suppress verbose third-party logs
            EnvFilter::new("trace")
                .add_directive("winit=warn".parse().unwrap())
                .add_directive("log=warn".parse().unwrap())
                .add_directive("egui=warn".parse().unwrap())
                .add_directive("eframe=warn".parse().unwrap())
        }))
        .with(file_layer)
        .with(stdout_layer)
        .init();

    info!("Starting YOLO Dataset Cleaner application");
    info!("Log file created at: {:?}", log_path);

    // Load settings to get window dimensions
    let settings = Settings::load();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([settings.window_width, settings.window_height])
            .with_title("YOLO Dataset Cleaner"),
        ..Default::default()
    };

    info!("Launching application window");
    eframe::run_native(
        "YOLO Dataset Cleaner",
        options,
        Box::new(|cc| {
            // Initialize egui-phosphor
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(DatasetCleanerApp::default()))
        }),
    )
}

pub struct DatasetCleanerApp {
    pub dataset: Dataset,
    pub current_index: usize,
    pub current_texture: Option<TextureHandle>,
    pub current_label: Option<LabelInfo>,
    pub config: AppConfig,
    pub dominant_color: Option<egui::Color32>,
    pub show_batch_delete_confirm: bool,
    pub batch_processing: bool,
    pub batch_stats: Option<BatchStats>,
    batch_progress_receiver: Option<Receiver<BatchProgressMessage>>,
    batch_cancel_flag: Option<Arc<AtomicBool>>,
    pub undo_state: Option<UndoState>,
    pub manual_index_input: String,
    pub image_load_error: Option<String>,
    pub settings: Settings,
    pub fullscreen_mode: bool,
    pub show_filter_dialog: bool,
    pub zoom_level: f32,
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

        let mut app = Self {
            dataset,
            current_index,
            current_texture: None,
            current_label: None,
            config,
            dominant_color: None,
            show_batch_delete_confirm: false,
            batch_processing: false,
            batch_stats: None,
            batch_progress_receiver: None,
            batch_cancel_flag: None,
            undo_state: None,
            manual_index_input: String::from("1"),
            image_load_error: None,
            settings,
            fullscreen_mode: false,
            show_filter_dialog: false,
            zoom_level: 1.0,
        };

        // Parse label for the current image if dataset was loaded
        if !app.dataset.get_image_files().is_empty() {
            app.parse_label_file();
        }

        app
    }
}

impl DatasetCleanerApp {
    pub fn load_dataset(&mut self, path: PathBuf) {
        info!("Loading dataset from: {:?}", path);
        self.dataset.load(path.clone());
        self.current_index = 0;
        self.current_texture = None;
        self.current_label = None;
        self.dominant_color = None;
        self.image_load_error = None;
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
        self.current_texture = None;
        self.current_label = None;
        self.dominant_color = None;
        self.image_load_error = None;
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
        self.image_load_error = None;

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
                self.dominant_color = Self::calculate_dominant_color(&img);

                self.current_texture = Some(texture);
                info!("Image loaded successfully");
            }
            Err(e) => {
                let error_msg = format!("Failed to load image: {}", e);
                error!("{:?}: {}", img_path, error_msg);
                self.image_load_error = Some(error_msg);
            }
        }
    }

    fn calculate_dominant_color(img: &image::DynamicImage) -> Option<egui::Color32> {
        image_analysis::calculate_dominant_color(img)
            .map(|(r, g, b)| egui::Color32::from_rgb(r, g, b))
    }

    pub fn parse_label_file(&mut self) {
        if self.dataset.get_image_files().is_empty() {
            self.current_label = None;
            return;
        }

        let img_path = &self.dataset.get_image_files()[self.current_index];

        // Get corresponding label file path
        let label_path = match self.get_label_path_for_image(img_path) {
            Some(path) => path,
            None => {
                self.current_label = None;
                return;
            }
        };

        // Parse label file using the dedicated module
        self.current_label = parse_label_file(&label_path);
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
        let label_path = self.get_label_path_for_image(img_path);
        info!("Label path: {:?}", label_path);

        // Create temp directory in system temp
        let temp_dir = std::env::temp_dir().join("yolo_dataset_cleaner_undo");
        info!("Temp dir: {:?}", temp_dir);

        if let Err(e) = fs::create_dir_all(&temp_dir) {
            eprintln!("ERROR creating temp directory: {}", e);
            return;
        }
        info!("Temp directory created successfully");

        // Generate unique temp paths using timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let temp_image_name = format!("{}_{}", timestamp, image_filename);
        let temp_image_path = temp_dir.join(&temp_image_name);
        info!("Temp image path: {:?}", temp_image_path);

        // Move image to temp location (use copy + remove for cross-drive compatibility)
        info!(
            "Attempting to move image from {:?} to {:?}",
            img_path, temp_image_path
        );
        if let Err(e) = fs::copy(img_path, &temp_image_path) {
            error!("ERROR copying image to temp: {}", e);
            error!("Source exists: {}", img_path.exists());
            error!(
                "Dest parent exists: {}",
                temp_image_path.parent().is_some_and(|p| p.exists())
            );
            return;
        }
        info!("Image copied to temp successfully");

        // Remove original image after successful copy
        if let Err(e) = fs::remove_file(img_path) {
            error!("ERROR removing original image after copy: {}", e);
            // Try to clean up the temp file
            let _ = fs::remove_file(&temp_image_path);
            return;
        }
        info!("Original image removed successfully");

        // Move label file to temp location if it exists
        let temp_label_path = if let Some(ref lbl_path) = label_path {
            if lbl_path.exists() {
                info!("Label file exists, attempting to move: {:?}", lbl_path);
                let temp_label_name = format!(
                    "{}_{}",
                    timestamp,
                    lbl_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("label.txt")
                );
                let temp_lbl = temp_dir.join(&temp_label_name);

                // Use copy + remove for cross-drive compatibility
                if let Err(e) = fs::copy(lbl_path, &temp_lbl) {
                    error!("ERROR copying label to temp: {}", e);
                    None
                } else if let Err(e) = fs::remove_file(lbl_path) {
                    error!("ERROR removing original label after copy: {}", e);
                    // Clean up temp label
                    let _ = fs::remove_file(&temp_lbl);
                    None
                } else {
                    info!("Label moved to temp successfully: {:?}", temp_lbl);
                    Some(temp_lbl)
                }
            } else {
                info!("Label file doesn't exist");
                None
            }
        } else {
            info!("No label path computed");
            None
        };

        // Create undo state
        info!("Creating undo state");
        self.undo_state = Some(UndoState {
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
        self.current_texture = None;
        self.current_label = None;
        self.dominant_color = None;
        self.image_load_error = None;
        info!("Cleared currentstate");

        // Parse the label for the new current image
        self.parse_label_file();
        info!("=== DELETE_CURRENT_IMAGE COMPLETED SUCCESSFULLY ===");
    }

    pub fn undo_delete(&mut self) {
        if let Some(undo_state) = self.undo_state.take() {
            info!(
                "Attempting to undo delete for: {}",
                undo_state.image_filename
            );
            // Restore image file (use copy + remove for cross-drive compatibility)
            if let Err(e) = fs::copy(&undo_state.temp_image_path, &undo_state.image_path) {
                error!("Error copying temp image back to original location: {}", e);
                // Put undo_state back if restore failed
                self.undo_state = Some(undo_state);
                return;
            }

            // Remove temp image after successful copy
            if let Err(e) = fs::remove_file(&undo_state.temp_image_path) {
                error!("Error removing temp image after restoration: {}", e);
                // Continue anyway since the restore was successful
            }
            debug!("Image successfully restored");

            // Restore label file if it exists
            if let (Some(temp_label), Some(orig_label)) =
                (&undo_state.temp_label_path, &undo_state.label_path)
            {
                if let Err(e) = fs::copy(temp_label, orig_label) {
                    error!("Error copying temp label back to original location: {}", e);
                } else if let Err(e) = fs::remove_file(temp_label) {
                    error!("Error removing temp label after restoration: {}", e);
                }
            }

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
            self.current_texture = None;
            self.current_label = None;
            self.dominant_color = None;
            self.image_load_error = None;

            // Parse the label for the current/restored image
            self.parse_label_file();
        }
    }

    pub fn finalize_delete(&mut self) {
        if let Some(undo_state) = self.undo_state.take() {
            // Permanently delete temp files
            if undo_state.temp_image_path.exists() {
                if let Err(e) = fs::remove_file(&undo_state.temp_image_path) {
                    error!("Error deleting temp image: {}", e);
                }
            }

            if let Some(temp_label) = &undo_state.temp_label_path {
                if temp_label.exists() {
                    if let Err(e) = fs::remove_file(temp_label) {
                        error!("Error deleting temp label: {}", e);
                    }
                }
            }
        }
    }

    pub fn next_image(&mut self) {
        if !self.dataset.get_image_files().is_empty()
            && self.current_index < self.dataset.get_image_files().len() - 1
        {
            self.current_index += 1;
            self.current_texture = None;
            self.current_label = None;
            self.dominant_color = None;
            self.image_load_error = None;
            self.zoom_level = 1.0;
            // Immediately parse the label file to ensure synchronization
            self.parse_label_file();

            // Save image index to settings
            self.settings.last_image_index = self.current_index;
            self.settings.save();
        }
    }

    pub fn prev_image(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.current_texture = None;
            self.current_label = None;
            self.dominant_color = None;
            self.image_load_error = None;
            self.zoom_level = 1.0;
            // Immediately parse the label file to ensure synchronization
            self.parse_label_file();

            // Save image index to settings
            self.settings.last_image_index = self.current_index;
            self.settings.save();
        }
    }

    pub fn jump_to_first(&mut self) {
        if !self.dataset.get_image_files().is_empty() && self.current_index != 0 {
            info!("Jumping to first image");
            self.current_index = 0;
            self.current_texture = None;
            self.current_label = None;
            self.dominant_color = None;
            self.image_load_error = None;
            self.zoom_level = 1.0;
            self.parse_label_file();

            // Save image index to settings
            self.settings.last_image_index = self.current_index;
            self.settings.save();
        }
    }

    pub fn jump_to_last(&mut self) {
        if !self.dataset.get_image_files().is_empty() {
            let last_index = self.dataset.get_image_files().len() - 1;
            if self.current_index != last_index {
                info!("Jumping to last image");
                self.current_index = last_index;
                self.current_texture = None;
                self.current_label = None;
                self.dominant_color = None;
                self.image_load_error = None;
                self.zoom_level = 1.0;
                self.parse_label_file();

                // Save image index to settings
                self.settings.last_image_index = self.current_index;
                self.settings.save();
            }
        }
    }

    pub fn jump_by_offset(&mut self, offset: isize) {
        if self.dataset.get_image_files().is_empty() {
            return;
        }

        let total_images = self.dataset.get_image_files().len();
        let new_index = if offset < 0 {
            // Jump backward
            self.current_index.saturating_sub((-offset) as usize)
        } else {
            // Jump forward
            (self.current_index + offset as usize).min(total_images - 1)
        };

        if new_index != self.current_index {
            info!("Jumping by {} to index {}", offset, new_index);
            self.current_index = new_index;
            self.current_texture = None;
            self.current_label = None;
            self.dominant_color = None;
            self.image_load_error = None;
            self.zoom_level = 1.0;
            self.parse_label_file();

            // Save image index to settings
            self.settings.last_image_index = self.current_index;
            self.settings.save();
        }
    }

    pub fn toggle_fullscreen(&mut self) {
        self.fullscreen_mode = !self.fullscreen_mode;
        info!("Fullscreen mode toggled: {}", self.fullscreen_mode);
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
        self.batch_processing = true;

        // Initialize stats
        let stats = BatchStats::default();
        self.batch_stats = Some(stats);

        // Create a channel for progress updates
        let (tx, rx) = channel::<BatchProgressMessage>();
        self.batch_progress_receiver = Some(rx);

        // Create cancellation flag
        let cancel_flag = Arc::new(AtomicBool::new(false));
        self.batch_cancel_flag = Some(cancel_flag.clone());

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
                    if let Some((r, g, b)) = image_analysis::calculate_dominant_color(&img) {
                        if image_analysis::is_near_black((r, g, b)) {
                            // Delete image file
                            if fs::remove_file(img_path).is_ok() {
                                // Delete corresponding label file
                                let label_path = img_path.to_str().map(|img_str| {
                                    let label_str = img_str
                                        .replace("\\images\\", "\\labels\\")
                                        .replace("/images/", "/labels/");
                                    PathBuf::from(label_str).with_extension("txt")
                                });

                                if let Some(label_path) = label_path {
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
        if let Some(flag) = &self.batch_cancel_flag {
            flag.store(true, Ordering::Relaxed);
        }
    }

    fn get_label_path_for_image(&self, img_path: &PathBuf) -> Option<PathBuf> {
        img_path.to_str().map(|img_str| {
            let label_str = img_str
                .replace("\\images\\", "\\labels\\")
                .replace("/images/", "/labels/");
            PathBuf::from(label_str).with_extension("txt")
        })
    }
}

impl eframe::App for DatasetCleanerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for batch processing updates
        let mut complete_stats = None;
        let mut cancelled = false;
        if let Some(receiver) = &self.batch_progress_receiver {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    BatchProgressMessage::Progress(stats) => {
                        self.batch_stats = Some(stats);
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
            self.batch_stats = Some(stats);
            self.batch_processing = false;
            self.batch_progress_receiver = None;
            self.batch_cancel_flag = None;

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
                self.current_texture = None;
                self.current_label = None;
                self.dominant_color = None;
                self.image_load_error = None;

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
                self.current_texture = None;
                self.current_label = None;
                self.dominant_color = None;
                self.image_load_error = None;

                // Parse the label for the current image
                self.parse_label_file();
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

        ui::handle_keyboard_shortcuts(self, ctx);
    }
}
