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

mod ui;



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
        use kmeans_colors::get_kmeans;
        use palette::{FromColor, Lab, Srgb};
        
        // Convert image to RGB
        let img_rgb = img.to_rgb8();
        let (width, height) = img_rgb.dimensions();
        
        // Sample pixels (to avoid processing too many pixels)
        let max_samples = 10000;
        let step = ((width * height) as f32 / max_samples as f32).sqrt().ceil() as u32;
        let step = step.max(1);
        
        let mut lab_pixels: Vec<Lab> = Vec::new();
        
        for y in (0..height).step_by(step as usize) {
            for x in (0..width).step_by(step as usize) {
                let pixel = img_rgb.get_pixel(x, y);
                let rgb = Srgb::new(
                    pixel[0] as f32 / 255.0,
                    pixel[1] as f32 / 255.0,
                    pixel[2] as f32 / 255.0,
                );
                lab_pixels.push(Lab::from_color(rgb));
            }
        }
        
        if lab_pixels.is_empty() {
            return None;
        }
        
        // Run k-means with k=3 to find dominant colors
        let k = 3;
        let max_iter = 20;
        let converge = 1.0;
        let verbose = false;
        let seed = 0;
        
        let result = get_kmeans(
            k,
            max_iter,
            converge,
            verbose,
            &lab_pixels,
            seed,
        );
        
        // Get the centroid with the most members (dominant color)
        let mut centroids_with_counts: Vec<_> = result.centroids
            .iter()
            .enumerate()
            .map(|(i, centroid)| {
                let count = result.indices.iter().filter(|&&idx| idx == i as u8).count();
                (centroid, count)
            })
            .collect();
        
        centroids_with_counts.sort_by(|a, b| b.1.cmp(&a.1));
        
        if let Some((dominant_lab, _)) = centroids_with_counts.first() {
            let rgb: Srgb = Srgb::from_color(**dominant_lab);
            let r = (rgb.red * 255.0).clamp(0.0, 255.0) as u8;
            let g = (rgb.green * 255.0).clamp(0.0, 255.0) as u8;
            let b = (rgb.blue * 255.0).clamp(0.0, 255.0) as u8;
            
            Some(egui::Color32::from_rgb(r, g, b))
        } else {
            None
        }
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
        ui::handle_keyboard_shortcuts(self, ctx);
    }
}
