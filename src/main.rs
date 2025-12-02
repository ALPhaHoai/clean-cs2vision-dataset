use eframe::egui;
use egui::{ColorImage, TextureHandle};
use std::path::PathBuf;
use std::fs;

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


fn main() -> Result<(), eframe::Error> {
    let config = AppConfig::default();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([config.window_width, config.window_height])
            .with_title("YOLO Dataset Cleaner"),
        ..Default::default()
    };
    
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
    pub show_delete_confirm: bool,
    pub config: AppConfig,
    pub dominant_color: Option<egui::Color32>,
    pub show_batch_delete_confirm: bool,
    pub batch_processing: bool,
    pub batch_stats: Option<BatchStats>,
}

impl Default for DatasetCleanerApp {
    fn default() -> Self {
        let config = AppConfig::default();
        let mut dataset = Dataset::new();
        
        // Load default dataset path from config
        if config.default_dataset_path.exists() {
            dataset.load(config.default_dataset_path.clone());
        }
        
        Self {
            dataset,
            current_index: 0,
            current_texture: None,
            current_label: None,
            show_delete_confirm: false,
            config,
            dominant_color: None,
            show_batch_delete_confirm: false,
            batch_processing: false,
            batch_stats: None,
        }
    }
}

impl DatasetCleanerApp {
    pub fn load_dataset(&mut self, path: PathBuf) {
        self.dataset.load(path);
        self.current_index = 0;
        self.current_texture = None;
        self.current_label = None;
        self.dominant_color = None;
    }
    
    pub fn change_split(&mut self, new_split: DatasetSplit) {
        self.dataset.change_split(new_split);
        self.current_index = 0;
        self.current_texture = None;
        self.current_label = None;
        self.dominant_color = None;
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
            return;
        }
        
        let img_path = &self.dataset.get_image_files()[self.current_index];
        
        // Delete the image file
        if let Err(e) = fs::remove_file(img_path) {
            eprintln!("Error deleting image: {}", e);
            return;
        }
        
        // Delete the corresponding label file (.txt) from labels folder
        let label_path = self.get_label_path_for_image(img_path)
            .unwrap_or_else(|| img_path.with_extension("txt"));
        
        if label_path.exists() {
            if let Err(e) = fs::remove_file(&label_path) {
                eprintln!("Error deleting label: {}", e);
            }
        }
        
        // Reload the current split to refresh the file list
        self.dataset.load_current_split();
        
        // Adjust index if needed
        if self.current_index >= self.dataset.get_image_files().len() && self.current_index > 0 {
            self.current_index -= 1;
        }
        
        // Clear current texture
        self.current_texture = None;
        self.show_delete_confirm = false;
    }
    
    pub fn next_image(&mut self) {
        if !self.dataset.get_image_files().is_empty() && self.current_index < self.dataset.get_image_files().len() - 1 {
            self.current_index += 1;
            self.current_texture = None;
            self.current_label = None;
            self.dominant_color = None;
        }
    }
    
    pub fn prev_image(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.current_texture = None;
            self.current_label = None;
            self.dominant_color = None;
        }
    }
    
    fn is_near_black(color: egui::Color32) -> bool {
        image_analysis::is_near_black((color.r(), color.g(), color.b()))
    }
    
    pub fn process_black_images(&mut self) {
        if self.dataset.get_image_files().is_empty() {
            return;
        }
        
        self.batch_processing = true;
        let mut stats = BatchStats::default();
        
        // Get all image files in current split
        let image_files: Vec<PathBuf> = self.dataset.get_image_files().clone();
        
        for (idx, img_path) in image_files.iter().enumerate() {
            stats.current_progress = idx + 1;
            stats.total_scanned += 1;
            
            // Load and analyze image
            if let Ok(img) = image::open(img_path) {
                if let Some(dominant_color) = Self::calculate_dominant_color(&img) {
                    if Self::is_near_black(dominant_color) {
                        // Delete image file
                        if fs::remove_file(img_path).is_ok() {
                            // Delete corresponding label file
                            if let Some(label_path) = self.get_label_path_for_image(img_path) {
                                if label_path.exists() {
                                    let _ = fs::remove_file(&label_path);
                                }
                            }
                            stats.total_deleted += 1;
                        }
                    }
                }
            }
            
            // Update stats
            self.batch_stats = Some(stats.clone());
        }
        
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
        
        self.batch_processing = false;
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
        ui::render_top_panel(self, ctx);
        ui::render_bottom_panel(self, ctx);
        
        if !self.dataset.get_image_files().is_empty() {
            ui::render_label_panel(self, ctx);
        }
        
        ui::render_central_panel(self, ctx);
        ui::render_delete_confirmation(self, ctx);
        ui::render_batch_delete_confirmation(self, ctx);
        ui::render_batch_progress(self, ctx);
        ui::handle_keyboard_shortcuts(self, ctx);
    }
}
