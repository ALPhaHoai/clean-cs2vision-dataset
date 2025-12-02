use crate::DatasetCleanerApp;
use eframe::egui;

/// Render the top panel with navigation and dataset controls
pub fn render_top_panel(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("🗂 YOLO Dataset Cleaner");
            
            ui.add_space(20.0);
            
            if ui.button("📁 Open Dataset Folder").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    app.load_dataset(path);
                }
            }
            
            ui.add_space(20.0);
            
            // Split selection buttons
            if app.dataset_path.is_some() {
                ui.label("Split:");
                
                if ui.selectable_label(
                    app.current_split == crate::DatasetSplit::Train,
                    "Train"
                ).clicked() {
                    app.change_split(crate::DatasetSplit::Train);
                }
                
                if ui.selectable_label(
                    app.current_split == crate::DatasetSplit::Val,
                    "Val"
                ).clicked() {
                    app.change_split(crate::DatasetSplit::Val);
                }
                
                if ui.selectable_label(
                    app.current_split == crate::DatasetSplit::Test,
                    "Test"
                ).clicked() {
                    app.change_split(crate::DatasetSplit::Test);
                }
                
                ui.add_space(20.0);
            }
            
            if !app.image_files.is_empty() {
                ui.label(format!(
                    "Image {} of {}",
                    app.current_index + 1,
                    app.image_files.len()
                ));
            }
        });
    });
}

/// Render the bottom panel with navigation controls
pub fn render_bottom_panel(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            
            // Navigation buttons
            if ui.add_enabled(app.current_index > 0, egui::Button::new("◄ Previous"))
                .clicked()
            {
                app.prev_image();
            }
            
            if ui.add_enabled(
                !app.image_files.is_empty() && app.current_index < app.image_files.len() - 1,
                egui::Button::new("Next ►"),
            )
            .clicked()
            {
                app.next_image();
            }
            
            ui.add_space(20.0);
            
            // Delete button
            if ui.add_enabled(
                !app.image_files.is_empty(),
                egui::Button::new("🗑 Delete Image & Label").fill(egui::Color32::from_rgb(200, 50, 50)),
            )
            .clicked()
            {
                app.show_delete_confirm = true;
            }
            
            ui.add_space(20.0);
            
            // Current file name
            if !app.image_files.is_empty() {
                if let Some(filename) = app.image_files[app.current_index].file_name() {
                    ui.label(format!("📄 {}", filename.to_string_lossy()));
                }
            }
        });
        ui.add_space(10.0);
    });
}

/// Render the right side panel with label information
pub fn render_label_panel(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    egui::SidePanel::right("label_panel")
        .default_width(300.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("📊 Label Information");
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            
            if let Some(label) = &app.current_label {
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

/// Render the central panel with the main image display
pub fn render_central_panel(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        if app.image_files.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.heading("No dataset loaded. Click 'Open Dataset Folder' to begin.");
            });
        } else {
            // Load image if not already loaded
            if app.current_texture.is_none() {
                app.load_current_image(ctx);
            }
            
            // Parse label file if not already parsed
            if app.current_label.is_none() {
                app.parse_label_file();
            }
            
            // Display the image
            if let Some(texture) = &app.current_texture {
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
                if let Some(label) = &app.current_label {
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

/// Render the delete confirmation dialog
pub fn render_delete_confirmation(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    if app.show_delete_confirm {
        egui::Window::new("⚠ Confirm Deletion")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Are you sure you want to delete this image and its label file?");
                ui.add_space(10.0);
                
                if !app.image_files.is_empty() {
                    if let Some(filename) = app.image_files[app.current_index].file_name() {
                        ui.label(format!("File: {}", filename.to_string_lossy()));
                    }
                }
                
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("✓ Yes, Delete").clicked() {
                        app.delete_current_image();
                    }
                    
                    if ui.button("✗ Cancel").clicked() {
                        app.show_delete_confirm = false;
                    }
                });
            });
    }
}
