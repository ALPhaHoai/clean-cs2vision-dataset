use eframe::egui;
use egui::{ColorImage, TextureHandle};
use std::path::PathBuf;
use std::fs;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Instant;
use tracing::{info, debug, warn, error};

mod log_formatter;
use log_formatter::BracketedFormatter;

mod label_parser;
use label_parser::{LabelInfo, parse_label_file};

mod dataset;
use dataset::{Dataset, DatasetSplit};

mod config;
use config::AppConfig;

mod image_analysis;

mod ui;

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
    // Initialize tracing subscriber with custom bracketed format
    // Default log level is "trace" to show all logs
    tracing_subscriber::fmt()
        .event_format(BracketedFormatter)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("trace"))
        )
        .init();
    
    info!("Starting YOLO Dataset Cleaner application");
    
    let config = AppConfig::default();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([config.window_width, config.window_height])
            .with_title("YOLO Dataset Cleaner"),
        ..Default::default()
    };
    
    info!("Launching application window");
    eframe::run_native(
        "YOLO Dataset Cleaner",
        options,
        Box::new(|_cc| Ok(Box::new(DatasetCleanerApp::default()))),
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
}

impl Default for DatasetCleanerApp {
    fn default() -> Self {
        let config = AppConfig::default();
        let mut dataset = Dataset::new();
        
        // Load default dataset path from config
        if config.default_dataset_path.exists() {
            dataset.load(config.default_dataset_path.clone());
        }
        
        let mut app = Self {
            dataset,
            current_index: 0,
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
        };
        
        // Parse label for the first image if dataset was loaded
        if !app.dataset.get_image_files().is_empty() {
            app.parse_label_file();
        }
        
        app
    }
}

impl DatasetCleanerApp {
    pub fn load_dataset(&mut self, path: PathBuf) {
        info!("Loading dataset from: {:?}", path);
        self.dataset.load(path);
        self.current_index = 0;
        self.current_texture = None;
        self.current_label = None;
        self.dominant_color = None;
        // Parse label file for the first image
        self.parse_label_file();
        info!("Dataset loaded successfully, total images: {}", self.dataset.get_image_files().len());
    }
    
    pub fn change_split(&mut self, new_split: DatasetSplit) {
        info!("Changing dataset split to: {:?}", new_split);
        self.dataset.change_split(new_split);
        self.current_index = 0;
        self.current_texture = None;
        self.current_label = None;
        self.dominant_color = None;
        // Parse label file for the first image
        self.parse_label_file();
        debug!("Split changed, current images count: {}", self.dataset.get_image_files().len());
    }
    
    pub fn load_current_image(&mut self, ctx: &egui::Context) {
        if self.dataset.get_image_files().is_empty() {
            return;
        }
        
        let img_path = &self.dataset.get_image_files()[self.current_index];
        
        if let Ok(img) = image::open(img_path) {
            let img_rgb = img.to_rgba8();
            let size = [img_rgb.width() as _, img_rgb.height() as _];
            let pixels = img_rgb.as_flat_samples();
            
            let color_image = ColorImage::from_rgba_unmultiplied(
                size,
                pixels.as_slice(),
            );
            
            let texture = ctx.load_texture(
                "current_image",
                color_image,
                egui::TextureOptions::LINEAR,
            );
            
            // Calculate dominant color
            self.dominant_color = Self::calculate_dominant_color(&img);
            
            self.current_texture = Some(texture);
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
        if self.dataset.get_image_files().is_empty() {
            warn!("Attempted to delete image, but no images available");
            return;
        }
        
        let img_path = &self.dataset.get_image_files()[self.current_index].clone();
        debug!("Deleting image at index {}: {:?}", self.current_index, img_path);
        
        // Get image filename for display
        let image_filename = img_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Get corresponding label file path
        let label_path = self.get_label_path_for_image(img_path);
        
        // Create temp directory in system temp
        let temp_dir = std::env::temp_dir().join("yolo_dataset_cleaner_undo");
        if let Err(e) = fs::create_dir_all(&temp_dir) {
            eprintln!("Error creating temp directory: {}", e);
            return;
        }
        
        // Generate unique temp paths using timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        
        let temp_image_name = format!("{}_{}", timestamp, image_filename);
        let temp_image_path = temp_dir.join(&temp_image_name);
        
        // Move image to temp location
        if let Err(e) = fs::rename(img_path, &temp_image_path) {
            error!("Error moving image to temp: {}", e);
            return;
        }
        info!("Image moved to temp location: {:?}", temp_image_path);
        
        // Move label file to temp location if it exists
        let temp_label_path = if let Some(ref lbl_path) = label_path {
            if lbl_path.exists() {
                let temp_label_name = format!("{}_{}", timestamp, 
                    lbl_path.file_name().and_then(|n| n.to_str()).unwrap_or("label.txt"));
                let temp_lbl = temp_dir.join(&temp_label_name);
                
                if fs::rename(lbl_path, &temp_lbl).is_ok() {
                    Some(temp_lbl)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        // Create undo state
        self.undo_state = Some(UndoState {
            image_path: img_path.clone(),
            label_path,
            image_filename,
            deleted_at: Instant::now(),
            temp_image_path,
            temp_label_path,
        });
        
        // Reload the current split to refresh the file list
        self.dataset.load_current_split();
        
        // Adjust index if needed
        if self.current_index >= self.dataset.get_image_files().len() && self.current_index > 0 {
            self.current_index -= 1;
        }
        
        // Clear current texture
        self.current_texture = None;
        self.current_label = None;
        self.dominant_color = None;
        
        // Parse the label for the new current image
        self.parse_label_file();
    }
    
    pub fn undo_delete(&mut self) {
        if let Some(undo_state) = self.undo_state.take() {
            info!("Attempting to undo delete for: {}", undo_state.image_filename);
            // Restore image file
            if let Err(e) = fs::rename(&undo_state.temp_image_path, &undo_state.image_path) {
                error!("Error restoring image: {}", e);
                // Put undo_state back if restore failed
                self.undo_state = Some(undo_state);
                return;
            }
            debug!("Image successfully restored");
            
            // Restore label file if it exists
            if let (Some(temp_label), Some(orig_label)) = 
                (&undo_state.temp_label_path, &undo_state.label_path) {
                if let Err(e) = fs::rename(temp_label, orig_label) {
                    eprintln!("Error restoring label: {}", e);
                }
            }
            
            // Reload the dataset to refresh file list
            self.dataset.load_current_split();
            
            // Try to find the restored image and navigate to it
            if let Some(index) = self.dataset.get_image_files()
                .iter()
                .position(|p| p == &undo_state.image_path) {
                self.current_index = index;
            }
            
            // Clear current texture to force reload
            self.current_texture = None;
            self.current_label = None;
            self.dominant_color = None;
            
            // Parse the label for the current/restored image
            self.parse_label_file();
        }
    }
    
    pub fn finalize_delete(&mut self) {
        if let Some(undo_state) = self.undo_state.take() {
            // Permanently delete temp files
            if undo_state.temp_image_path.exists() {
                if let Err(e) = fs::remove_file(&undo_state.temp_image_path) {
                    eprintln!("Error deleting temp image: {}", e);
                }
            }
            
            if let Some(temp_label) = &undo_state.temp_label_path {
                if temp_label.exists() {
                    if let Err(e) = fs::remove_file(temp_label) {
                        eprintln!("Error deleting temp label: {}", e);
                    }
                }
            }
        }
    }

    
    pub fn next_image(&mut self) {
        if !self.dataset.get_image_files().is_empty() && self.current_index < self.dataset.get_image_files().len() - 1 {
            self.current_index += 1;
            self.current_texture = None;
            self.current_label = None;
            self.dominant_color = None;
            // Immediately parse the label file to ensure synchronization
            self.parse_label_file();
        }
    }
    
    pub fn prev_image(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.current_texture = None;
            self.current_label = None;
            self.dominant_color = None;
            // Immediately parse the label file to ensure synchronization
            self.parse_label_file();
        }
    }
    

    pub fn process_black_images(&mut self) {
        if self.dataset.get_image_files().is_empty() {
            warn!("No images to process for black image removal");
            return;
        }
        
        info!("Starting batch processing to remove black images, total images: {}", self.dataset.get_image_files().len());
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
                    warn!("Batch processing cancelled by user at image {}/{}", idx, image_files.len());
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
            info!("Batch processing complete. Scanned: {}, Deleted: {}", stats.total_scanned, stats.total_deleted);
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
                if self.current_index >= self.dataset.get_image_files().len() && self.current_index > 0 {
                    self.current_index = self.dataset.get_image_files().len().saturating_sub(1);
                }
                
                // Clear current texture to force reload
                self.current_texture = None;
                self.current_label = None;
                self.dominant_color = None;
                
                // Parse the label for the current image
                self.parse_label_file();
            } else {
                // For cancelled operations, still reload but keep showing the dialog
                self.dataset.load_current_split();
                
                // Adjust current index if needed
                if self.current_index >= self.dataset.get_image_files().len() && self.current_index > 0 {
                    self.current_index = self.dataset.get_image_files().len().saturating_sub(1);
                }
                
                // Clear current texture to force reload
                self.current_texture = None;
                self.current_label = None;
                self.dominant_color = None;
                
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
        ui::handle_keyboard_shortcuts(self, ctx);
    }
}
