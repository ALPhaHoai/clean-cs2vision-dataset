use crate::DatasetCleanerApp;
use eframe::egui;
use super::image_renderer::ImageRenderer;

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
            if app.dataset.dataset_path().is_some() {
                ui.label("Split:");
                
                if ui.selectable_label(
                    app.dataset.current_split() == crate::DatasetSplit::Train,
                    "Train"
                ).clicked() {
                    app.change_split(crate::DatasetSplit::Train);
                }
                
                if ui.selectable_label(
                    app.dataset.current_split() == crate::DatasetSplit::Val,
                    "Val"
                ).clicked() {
                    app.change_split(crate::DatasetSplit::Val);
                }
                
                if ui.selectable_label(
                    app.dataset.current_split() == crate::DatasetSplit::Test,
                    "Test"
                ).clicked() {
                    app.change_split(crate::DatasetSplit::Test);
                }
                
                ui.add_space(20.0);
            }
            
            if !app.dataset.get_image_files().is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Image");
                    
                    let current_display = (app.current_index + 1).to_string();
                    
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut app.manual_index_input)
                            .desired_width(60.0)
                    );
                    
                    // Handle manual input when user presses Enter FIRST before syncing
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if let Ok(new_index) = app.manual_index_input.trim().parse::<usize>() {
                            if new_index > 0 && new_index <= app.dataset.get_image_files().len() {
                                app.current_index = new_index - 1;
                                app.current_texture = None;
                                app.current_label = None;
                                app.dominant_color = None;
                                app.parse_label_file();
                                app.manual_index_input = new_index.to_string();
                            } else {
                                // Reset to current valid value if out of range
                                app.manual_index_input = current_display.clone();
                            }
                        } else {
                            // Reset to current valid value if invalid input
                            app.manual_index_input = current_display.clone();
                        }
                    } 
                    // Sync the input text with current index when not focused and not pressing Enter
                    else if !response.has_focus() && app.manual_index_input != current_display {
                        app.manual_index_input = current_display;
                    }
                    
                    ui.label(format!("of {}", app.dataset.get_image_files().len()));
                });
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
                !app.dataset.get_image_files().is_empty() && app.current_index < app.dataset.get_image_files().len() - 1,
                egui::Button::new("Next ►"),
            )
            .clicked()
            {
                app.next_image();
            }
            
            ui.add_space(20.0);
            
            // Delete button
            if ui.add_enabled(
                !app.dataset.get_image_files().is_empty(),
                egui::Button::new("🗑 Delete Image & Label").fill(egui::Color32::from_rgb(200, 50, 50)),
            )
            .clicked()
            {
                app.delete_current_image();
            }
            
            ui.add_space(20.0);
            
            // Batch delete black images button
            let button_text = if app.batch_processing {
                if let Some(stats) = &app.batch_stats {
                    let total = stats.total_scanned.max(stats.current_progress);
                    let percentage = if total > 0 {
                        (stats.current_progress as f32 / total as f32 * 100.0) as u32
                    } else {
                        0
                    };
                    format!("🧹 Processing... {}%", percentage)
                } else {
                    "🧹 Processing...".to_string()
                }
            } else {
                "🧹 Remove Black Images".to_string()
            };
            
            let button = egui::Button::new(&button_text).fill(egui::Color32::from_rgb(100, 100, 180));
            if ui.add_enabled(
                !app.dataset.get_image_files().is_empty() && !app.batch_processing,
                button,
            )
            .clicked()
            {
                app.show_batch_delete_confirm = true;
            }
            
            // Cancel button (only visible during batch processing)
            if app.batch_processing {
                if ui.button("❌ Cancel").clicked() {
                    app.cancel_batch_processing();
                }
            }
            
            ui.add_space(20.0);

            
            // Current file name
            if !app.dataset.get_image_files().is_empty() {
                if let Some(filename) = app.dataset.get_image_files()[app.current_index].file_name() {
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
        .default_width(app.config.side_panel_width)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("📊 Label Information");
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            
            // Display dominant color
            if let Some(color) = app.dominant_color {
                ui.label(egui::RichText::new("🎨 Dominant Color")
                    .strong()
                    .size(16.0));
                ui.add_space(5.0);
                
                ui.horizontal(|ui| {
                    // Color swatch
                    let (rect, _response) = ui.allocate_exact_size(
                        egui::vec2(60.0, 40.0),
                        egui::Sense::hover()
                    );
                    ui.painter().rect_filled(rect, 4.0, color);
                    ui.painter().rect_stroke(
                        rect,
                        4.0,
                        egui::Stroke::new(2.0, egui::Color32::from_gray(128))
                    );
                    
                    ui.add_space(10.0);
                    
                    // RGB values
                    ui.vertical(|ui| {
                        ui.label(format!("R: {}", color.r()));
                        ui.label(format!("G: {}", color.g()));
                        ui.label(format!("B: {}", color.b()));
                    });
                });
                
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
            }
            
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
                                    let (class_color, _) = app.config.get_class_colors(detection.class_id);
                                    
                                    ui.label(egui::RichText::new(format!("#{}", i + 1))
                                        .strong());
                                    ui.label(egui::RichText::new(app.config.get_class_name(detection.class_id))
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
        if app.dataset.get_image_files().is_empty() {
            ui.centered_and_justified(|ui| {
                ui.heading("No dataset loaded. Click 'Open Dataset Folder' to begin.");
            });
        } else {
            // Load image if not already loaded
            if app.current_texture.is_none() {
                app.load_current_image(ctx);
            }
            
            // Display the image
            if let Some(texture) = &app.current_texture {
                let available_size = ui.available_size();
                let img_size = texture.size_vec2();
                
                // Calculate scaling to fit the image within available space
                let scale = ImageRenderer::calculate_image_scale(img_size, available_size);
                
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
                    ImageRenderer::draw_bounding_boxes(
                        ui.painter(),
                        label,
                        image_rect,
                        scaled_size,
                        &app.config
                    );
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


