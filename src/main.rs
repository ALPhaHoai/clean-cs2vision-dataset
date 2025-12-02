use eframe::egui;
use egui::{ColorImage, TextureHandle};
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone)]
struct YoloDetection {
    class_id: u32,
    x_center: f32,
    y_center: f32,
    width: f32,
    height: f32,
}

impl YoloDetection {
    fn class_name(&self) -> &str {
        match self.class_id {
            0 => "CT",
            1 => "T",
            _ => "Unknown",
        }
    }
}

#[derive(Debug, Clone)]
struct LabelInfo {
    detections: Vec<YoloDetection>,
    resolution: Option<String>,
    map: Option<String>,
    timestamp: Option<String>,
}

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
    current_label: Option<LabelInfo>,
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
        let mut app = Self {
            dataset_path: None,
            current_split: DatasetSplit::Train,
            image_files: Vec::new(),
            current_index: 0,
            current_texture: None,
            current_label: None,
            show_delete_confirm: false,
        };
        
        // Load default dataset path
        let default_path = PathBuf::from(r"E:\CS2Vison\cs2-data-dumper\dump");
        if default_path.exists() {
            app.load_dataset(default_path);
        }
        
        app
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
        self.current_label = None;
        
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
    
    fn parse_label_file(&mut self) {
        if self.image_files.is_empty() {
            self.current_label = None;
            return;
        }
        
        let img_path = &self.image_files[self.current_index];
        
        // Get corresponding label file path
        let label_path = if let Some(img_str) = img_path.to_str() {
            let label_str = img_str.replace("\\images\\", "\\labels\\").replace("/images/", "/labels/");
            PathBuf::from(label_str).with_extension("txt")
        } else {
            self.current_label = None;
            return;
        };
        
        // Read and parse label file
        if let Ok(content) = fs::read_to_string(&label_path) {
            let mut detections = Vec::new();
            let mut resolution = None;
            let mut map = None;
            let mut timestamp = None;
            
            for line in content.lines() {
                let line = line.trim();
                
                // Parse metadata from comment line
                // Format: # Resolution: 2560x1440, Map: de_dust2, Time: 1764637338
                if line.starts_with('#') {
                    let parts: Vec<&str> = line[1..].split(',').collect();
                    for part in parts {
                        let part = part.trim();
                        if let Some(res) = part.strip_prefix("Resolution:") {
                            resolution = Some(res.trim().to_string());
                        } else if let Some(m) = part.strip_prefix("Map:") {
                            map = Some(m.trim().to_string());
                        } else if let Some(t) = part.strip_prefix("Time:") {
                            timestamp = Some(t.trim().to_string());
                        }
                    }
                } else if !line.is_empty() {
                    // Parse detection line
                    // Format: class_id x_center y_center width height
                    let values: Vec<&str> = line.split_whitespace().collect();
                    if values.len() == 5 {
                        if let (Ok(class_id), Ok(x), Ok(y), Ok(w), Ok(h)) = (
                            values[0].parse::<u32>(),
                            values[1].parse::<f32>(),
                            values[2].parse::<f32>(),
                            values[3].parse::<f32>(),
                            values[4].parse::<f32>(),
                        ) {
                            detections.push(YoloDetection {
                                class_id,
                                x_center: x,
                                y_center: y,
                                width: w,
                                height: h,
                            });
                        }
                    }
                }
            }
            
            self.current_label = Some(LabelInfo {
                detections,
                resolution,
                map,
                timestamp,
            });
        } else {
            self.current_label = None;
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
            self.current_label = None;
        }
    }
    
    fn prev_image(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.current_texture = None;
            self.current_label = None;
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
        
        // Right panel for label information
        if !self.image_files.is_empty() {
            egui::SidePanel::right("label_panel")
                .default_width(300.0)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.heading("📊 Label Information");
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);
                    
                    if let Some(label) = &self.current_label {
                        // Detection count
                        ui.label(egui::RichText::new(format!("🎯 Detections: {}", label.detections.len()))
                            .strong()
                            .size(16.0));
                        
                        ui.add_space(10.0);
                        
                        // Metadata
                        if let Some(res) = &label.resolution {
                            ui.label(format!("📐 Resolution: {}", res));
                        }
                        if let Some(map) = &label.map {
                            ui.label(format!("🗺 Map: {}", map));
                        }
                        if let Some(time) = &label.timestamp {
                            ui.label(format!("⏰ Timestamp: {}", time));
                        }
                        
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);
                        
                        // Detection details
                        if label.detections.is_empty() {
                            ui.label(egui::RichText::new("No players detected")
                                .italics()
                                .color(egui::Color32::GRAY));
                        } else {
                            ui.label(egui::RichText::new("Detected Players:")
                                .strong()
                                .size(14.0));
                            ui.add_space(5.0);
                            
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                for (i, detection) in label.detections.iter().enumerate() {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            let class_color = match detection.class_id {
                                                0 => egui::Color32::from_rgb(100, 149, 237), // CT - Blue
                                                1 => egui::Color32::from_rgb(255, 140, 0),   // T - Orange
                                                _ => egui::Color32::GRAY,
                                            };
                                            
                                            ui.label(egui::RichText::new(format!("#{}", i + 1))
                                                .strong());
                                            ui.label(egui::RichText::new(detection.class_name())
                                                .strong()
                                                .color(class_color));
                                        });
                                        
                                        ui.add_space(5.0);
                                        
                                        ui.label(format!("Center: ({:.4}, {:.4})", 
                                            detection.x_center, detection.y_center));
                                        ui.label(format!("Size: {:.4} × {:.4}", 
                                            detection.width, detection.height));
                                    });
                                    
                                    ui.add_space(5.0);
                                }
                            });
                        }
                    } else {
                        ui.label(egui::RichText::new("No label file found")
                            .italics()
                            .color(egui::Color32::GRAY));
                    }
                });
        }
        
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
                
                // Parse label file if not already parsed
                if self.current_label.is_none() {
                    self.parse_label_file();
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
