use eframe::egui;
use egui::{ColorImage, TextureHandle, Vec2};
use std::path::{Path, PathBuf};
use std::fs;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("YOLO Dataset Cleaner"),
        ..Default::default()
    };
    
    eframe::run_native(
        "YOLO Dataset Cleaner",
        options,
        Box::new(|_cc| Ok(Box::new(DatasetCleanerApp::default()))),
    )
}

struct DatasetCleanerApp {
    dataset_path: Option<PathBuf>,
    current_split: DatasetSplit,
    image_files: Vec<PathBuf>,
    current_index: usize,
    current_texture: Option<TextureHandle>,
    show_delete_confirm: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DatasetSplit {
    Train,
    Val,
    Test,
}

impl DatasetSplit {
    fn as_str(&self) -> &str {
        match self {
            DatasetSplit::Train => "train",
            DatasetSplit::Val => "val",
            DatasetSplit::Test => "test",
        }
    }
}

impl Default for DatasetCleanerApp {
    fn default() -> Self {
        Self {
            dataset_path: None,
            current_split: DatasetSplit::Train,
            image_files: Vec::new(),
            current_index: 0,
            current_texture: None,
            show_delete_confirm: false,
        }
    }
}

impl DatasetCleanerApp {
    fn load_dataset(&mut self, path: PathBuf) {
        self.dataset_path = Some(path.clone());
        self.load_current_split();
    }
    
    fn load_current_split(&mut self) {
        self.image_files.clear();
        self.current_index = 0;
        self.current_texture = None;
        
        if let Some(base_path) = &self.dataset_path {
            // Navigate to split/images folder
            let images_path = base_path
                .join(self.current_split.as_str())
                .join("images");
            
            // Load all image files from the split directory
            if let Ok(entries) = fs::read_dir(&images_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        let ext = ext.to_string_lossy().to_lowercase();
                        if ext == "png" || ext == "jpg" || ext == "jpeg" {
                            self.image_files.push(path);
                        }
                    }
                }
            }
            
            // Sort files for consistent ordering
            self.image_files.sort();
        }
    }
    
    fn change_split(&mut self, new_split: DatasetSplit) {
        if self.current_split != new_split {
            self.current_split = new_split;
            self.load_current_split();
        }
    }
    
    fn load_current_image(&mut self, ctx: &egui::Context) {
        if self.image_files.is_empty() {
            return;
        }
        
        let img_path = &self.image_files[self.current_index];
        
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
            
            self.current_texture = Some(texture);
        }
    }
    
    fn delete_current_image(&mut self) {
        if self.image_files.is_empty() {
            return;
        }
        
        let img_path = &self.image_files[self.current_index];
        
        // Delete the image file
        if let Err(e) = fs::remove_file(img_path) {
            eprintln!("Error deleting image: {}", e);
            return;
        }
        
        // Delete the corresponding label file (.txt) from labels folder
        // Replace /images/ with /labels/ in the path
        let label_path = if let Some(img_str) = img_path.to_str() {
            let label_str = img_str.replace("\\images\\", "\\labels\\").replace("/images/", "/labels/");
            PathBuf::from(label_str).with_extension("txt")
        } else {
            img_path.with_extension("txt")
        };
        
        if label_path.exists() {
            if let Err(e) = fs::remove_file(&label_path) {
                eprintln!("Error deleting label: {}", e);
            }
        }
        
        // Remove from the list
        self.image_files.remove(self.current_index);
        
        // Adjust index if needed
        if self.current_index >= self.image_files.len() && self.current_index > 0 {
            self.current_index -= 1;
        }
        
        // Clear current texture
        self.current_texture = None;
        self.show_delete_confirm = false;
    }
    
    fn next_image(&mut self) {
        if !self.image_files.is_empty() && self.current_index < self.image_files.len() - 1 {
            self.current_index += 1;
            self.current_texture = None;
        }
    }
    
    fn prev_image(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.current_texture = None;
        }
    }
}

impl eframe::App for DatasetCleanerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top panel with controls
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🗂 YOLO Dataset Cleaner");
                
                ui.add_space(20.0);
                
                if ui.button("📁 Open Dataset Folder").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.load_dataset(path);
                    }
                }
                
                ui.add_space(20.0);
                
                // Split selection buttons
                if self.dataset_path.is_some() {
                    ui.label("Split:");
                    
                    if ui.selectable_label(
                        self.current_split == DatasetSplit::Train,
                        "Train"
                    ).clicked() {
                        self.change_split(DatasetSplit::Train);
                    }
                    
                    if ui.selectable_label(
                        self.current_split == DatasetSplit::Val,
                        "Val"
                    ).clicked() {
                        self.change_split(DatasetSplit::Val);
                    }
                    
                    if ui.selectable_label(
                        self.current_split == DatasetSplit::Test,
                        "Test"
                    ).clicked() {
                        self.change_split(DatasetSplit::Test);
                    }
                    
                    ui.add_space(20.0);
                }
                
                if !self.image_files.is_empty() {
                    ui.label(format!(
                        "Image {} of {}",
                        self.current_index + 1,
                        self.image_files.len()
                    ));
                }
            });
        });
        
        // Bottom panel with navigation and actions
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                
                // Navigation buttons
                if ui.add_enabled(self.current_index > 0, egui::Button::new("◄ Previous"))
                    .clicked()
                {
                    self.prev_image();
                }
                
                if ui.add_enabled(
                    !self.image_files.is_empty() && self.current_index < self.image_files.len() - 1,
                    egui::Button::new("Next ►"),
                )
                .clicked()
                {
                    self.next_image();
                }
                
                ui.add_space(20.0);
                
                // Delete button
                if ui.add_enabled(
                    !self.image_files.is_empty(),
                    egui::Button::new("🗑 Delete Image & Label").fill(egui::Color32::from_rgb(200, 50, 50)),
                )
                .clicked()
                {
                    self.show_delete_confirm = true;
                }
                
                ui.add_space(20.0);
                
                // Current file name
                if !self.image_files.is_empty() {
                    if let Some(filename) = self.image_files[self.current_index].file_name() {
                        ui.label(format!("📄 {}", filename.to_string_lossy()));
                    }
                }
            });
            ui.add_space(10.0);
        });
        
        // Central panel with image display
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.image_files.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.heading("No dataset loaded. Click 'Open Dataset Folder' to begin.");
                });
            } else {
                // Load image if not already loaded
                if self.current_texture.is_none() {
                    self.load_current_image(ctx);
                }
                
                // Display the image
                if let Some(texture) = &self.current_texture {
                    let available_size = ui.available_size();
                    let img_size = texture.size_vec2();
                    
                    // Calculate scaling to fit the image within available space
                    let scale = (available_size.x / img_size.x)
                        .min(available_size.y / img_size.y)
                        .min(1.0);
                    
                    let scaled_size = img_size * scale;
                    
                    ui.centered_and_justified(|ui| {
                        ui.image((texture.id(), scaled_size));
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.spinner();
                        ui.label("Loading image...");
                    });
                }
            }
        });
        
        // Delete confirmation dialog
        if self.show_delete_confirm {
            egui::Window::new("⚠ Confirm Deletion")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Are you sure you want to delete this image and its label file?");
                    ui.add_space(10.0);
                    
                    if !self.image_files.is_empty() {
                        if let Some(filename) = self.image_files[self.current_index].file_name() {
                            ui.label(format!("File: {}", filename.to_string_lossy()));
                        }
                    }
                    
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("✓ Yes, Delete").clicked() {
                            self.delete_current_image();
                        }
                        
                        if ui.button("✗ Cancel").clicked() {
                            self.show_delete_confirm = false;
                        }
                    });
                });
        }
        
        // Keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            self.next_image();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            self.prev_image();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
            if !self.image_files.is_empty() {
                self.show_delete_confirm = true;
            }
        }
    }
}
