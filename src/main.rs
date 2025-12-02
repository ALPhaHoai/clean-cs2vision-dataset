use eframe::egui;
use egui::{ColorImage, TextureHandle};
use std::path::PathBuf;
use std::fs;

mod label_parser;
use label_parser::{LabelInfo, parse_label_file};



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
        let label_path = self.get_label_path_for_image(img_path)
            .unwrap_or_else(|| img_path.with_extension("txt"));
        
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
    
    fn get_label_path_for_image(&self, img_path: &PathBuf) -> Option<PathBuf> {
        img_path.to_str().map(|img_str| {
            let label_str = img_str
                .replace("\\images\\", "\\labels\\")
                .replace("/images/", "/labels/");
            PathBuf::from(label_str).with_extension("txt")
        })
    }
    
    fn render_top_panel(&mut self, ctx: &egui::Context) {
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
    }
    
    fn render_bottom_panel(&mut self, ctx: &egui::Context) {
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
    }
    
    fn render_label_panel(&mut self, ctx: &egui::Context) {
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
    
    fn render_central_panel(&mut self, ctx: &egui::Context) {
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
                    
                    // Center the image and get the rect where it's drawn
                    let available_rect = ui.available_rect_before_wrap();
                    let image_rect = egui::Rect::from_center_size(
                        available_rect.center(),
                        scaled_size
                    );
                    
                    // Draw the image
                    ui.put(image_rect, egui::Image::new((texture.id(), scaled_size)));
                    
                    // Draw bounding boxes if label data exists
                    if let Some(label) = &self.current_label {
                        let painter = ui.painter();
                        
                        for detection in &label.detections {
                            // Convert normalized YOLO coordinates to screen coordinates
                            // YOLO format: center_x, center_y, width, height (all normalized 0-1)
                            let bbox_center_x = detection.x_center * scaled_size.x;
                            let bbox_center_y = detection.y_center * scaled_size.y;
                            let bbox_width = detection.width * scaled_size.x;
                            let bbox_height = detection.height * scaled_size.y;
                            
                            // Calculate top-left corner
                            let bbox_x = bbox_center_x - (bbox_width / 2.0);
                            let bbox_y = bbox_center_y - (bbox_height / 2.0);
                            
                            // Create rect in screen space (offset by image position)
                            let bbox_rect = egui::Rect::from_min_size(
                                egui::pos2(
                                    image_rect.min.x + bbox_x,
                                    image_rect.min.y + bbox_y
                                ),
                                egui::vec2(bbox_width, bbox_height)
                            );
                            
                            // Choose color based on class
                            let (stroke_color, fill_color) = match detection.class_id {
                                0 => (
                                    egui::Color32::from_rgb(100, 149, 237), // CT - Blue
                                    egui::Color32::from_rgba_unmultiplied(100, 149, 237, 30)
                                ),
                                1 => (
                                    egui::Color32::from_rgb(255, 140, 0),   // T - Orange
                                    egui::Color32::from_rgba_unmultiplied(255, 140, 0, 30)
                                ),
                                _ => (
                                    egui::Color32::GRAY,
                                    egui::Color32::from_rgba_unmultiplied(128, 128, 128, 30)
                                ),
                            };
                            
                            // Draw filled rectangle
                            painter.rect_filled(bbox_rect, 0.0, fill_color);
                            
                            // Draw border
                            painter.rect_stroke(
                                bbox_rect,
                                0.0,
                                egui::Stroke::new(2.0, stroke_color)
                            );
                            
                            // Draw label text
                            let label_text = detection.class_name();
                            let font_id = egui::FontId::proportional(14.0);
                            let text_galley = painter.layout_no_wrap(
                                label_text.to_string(),
                                font_id,
                                egui::Color32::WHITE
                            );
                            
                            // Draw text background
                            let text_pos = bbox_rect.min + egui::vec2(2.0, -18.0);
                            let text_bg_rect = egui::Rect::from_min_size(
                                text_pos,
                                egui::vec2(text_galley.size().x + 6.0, 16.0)
                            );
                            painter.rect_filled(text_bg_rect, 2.0, stroke_color);
                            
                            // Draw text
                            painter.galley(
                                text_pos + egui::vec2(3.0, 0.0),
                                text_galley,
                                egui::Color32::WHITE
                            );
                        }
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.spinner();
                        ui.label("Loading image...");
                    });
                }
            }
        });
    }
    
    fn render_delete_confirmation(&mut self, ctx: &egui::Context) {
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
    }
    
    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
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

impl eframe::App for DatasetCleanerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_top_panel(ctx);
        self.render_bottom_panel(ctx);
        
        if !self.image_files.is_empty() {
            self.render_label_panel(ctx);
        }
        
        self.render_central_panel(ctx);
        self.render_delete_confirmation(ctx);
        self.handle_keyboard_shortcuts(ctx);
    }
}
