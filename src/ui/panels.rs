use crate::app::DatasetCleanerApp;
use crate::core::dataset::DatasetSplit;
use eframe::egui;
use super::image_renderer::ImageRenderer;
use egui_phosphor::regular as Icon;

// Navigation overlay constants
const NAVIGATION_OVERLAY_WIDTH: f32 = 60.0;
const NAVIGATION_ARROW_SIZE: f32 = 20.0;
const OVERLAY_HOVER_ALPHA: u8 = 50;
const OVERLAY_SHADOW_ALPHA: u8 = 100;

/// Direction for navigation arrows
enum ArrowDirection {
    Left,
    Right,
}

/// Draw a navigation arrow with optional shadow
fn draw_navigation_arrow(
    painter: &egui::Painter,
    center: egui::Pos2,
    direction: ArrowDirection,
    is_hovered: bool,
) {
    let arrow_color = if is_hovered {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_white_alpha(128)
    };
    
    let points = match direction {
        ArrowDirection::Left => vec![
            center + egui::vec2(NAVIGATION_ARROW_SIZE / 2.0, -NAVIGATION_ARROW_SIZE),
            center + egui::vec2(-NAVIGATION_ARROW_SIZE / 2.0, 0.0),
            center + egui::vec2(NAVIGATION_ARROW_SIZE / 2.0, NAVIGATION_ARROW_SIZE),
        ],
        ArrowDirection::Right => vec![
            center + egui::vec2(-NAVIGATION_ARROW_SIZE / 2.0, -NAVIGATION_ARROW_SIZE),
            center + egui::vec2(NAVIGATION_ARROW_SIZE / 2.0, 0.0),
            center + egui::vec2(-NAVIGATION_ARROW_SIZE / 2.0, NAVIGATION_ARROW_SIZE),
        ],
    };
    
    // Add a small shadow/outline for better visibility against light images
    if !is_hovered {
        let shadow_offset = egui::vec2(1.0, 1.0);
        let shadow_points: Vec<egui::Pos2> = points.iter().map(|p| *p + shadow_offset).collect();
        painter.add(egui::Shape::convex_polygon(
            shadow_points,
            egui::Color32::from_black_alpha(OVERLAY_SHADOW_ALPHA),
            egui::Stroke::NONE
        ));
    }

    painter.add(egui::Shape::convex_polygon(
        points,
        arrow_color,
        egui::Stroke::NONE
    ));
}

/// Handle manual index input when user presses Enter
/// Returns true if the input was processed successfully
fn handle_manual_index_input(
    app: &mut DatasetCleanerApp,
    new_index_str: &str,
    current_display: &str,
) -> bool {
    if let Ok(new_index) = new_index_str.trim().parse::<usize>() {
        if app.filter.is_active() {
            // Navigate using virtual (filtered) index
            if new_index > 0 && new_index <= app.filter.filtered_count() {
                if let Some(actual_idx) = app.filter.get_actual_index(new_index - 1) {
                    app.current_index = actual_idx;
                    app.image.texture = None;
                    app.image.label = None;
                    app.image.dominant_color = None;
                    app.parse_label_file();
                    app.ui.manual_index_input = new_index.to_string();
                    return true;
                }
            }
        } else {
            // Navigate using absolute index
            if new_index > 0 && new_index <= app.dataset.get_image_files().len() {
                app.current_index = new_index - 1;
                app.image.texture = None;
                app.image.label = None;
                app.image.dominant_color = None;
                app.parse_label_file();
                app.ui.manual_index_input = new_index.to_string();
                return true;
            }
        }
    }
    
    // Reset to current valid value if invalid input or out of range
    app.ui.manual_index_input = current_display.to_string();
    false
}

/// Render the top panel with navigation and dataset controls
pub fn render_top_panel(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading(format!("{} YOLO Dataset Cleaner", Icon::FOLDERS));
            
            ui.add_space(20.0);
            
            if ui.button(format!("{} Open Dataset Folder", Icon::FOLDER_OPEN)).clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    app.load_dataset(path);
                }
            }
            
            ui.add_space(20.0);
            
            // Split selection buttons
            if app.dataset.dataset_path().is_some() {
                ui.label("Split:");
                
                if ui.selectable_label(
                    app.dataset.current_split() == DatasetSplit::Train,
                    "Train"
                ).clicked() {
                    app.change_split(DatasetSplit::Train);
                }
                
                if ui.selectable_label(
                    app.dataset.current_split() == DatasetSplit::Val,
                    "Val"
                ).clicked() {
                    app.change_split(DatasetSplit::Val);
                }
                
                if ui.selectable_label(
                    app.dataset.current_split() == DatasetSplit::Test,
                    "Test"
                ).clicked() {
                    app.change_split(DatasetSplit::Test);
                }
                
                ui.add_space(20.0);
            }
            
            // Filter button (always visible)
            if ui.button(format!("{} Filter", Icon::FUNNEL)).clicked() {
                app.ui.show_filter_dialog = true;
            }
            
            ui.add_space(20.0);
            
            if !app.dataset.get_image_files().is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Image");
                    
                    // Calculate the display position (virtual if filtered, actual otherwise)
                    let current_display = if app.filter.is_active() {
                        // Show position in filtered list
                        if let Some(virtual_idx) = app.filter.get_filtered_index(app.current_index) {
                            (virtual_idx + 1).to_string()
                        } else {
                            // Current image not in filter, shouldn't happen but fallback
                            "?".to_string()
                        }
                    } else {
                        // Show absolute position
                        (app.current_index + 1).to_string()
                    };
                    
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut app.ui.manual_index_input)
                            .desired_width(60.0)
                    );
                    
                    // Handle manual input when user presses Enter FIRST before syncing
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        handle_manual_index_input(app, &app.ui.manual_index_input.clone(), &current_display);
                    } 
                    // Sync the input text with current index when not focused and not pressing Enter
                    else if !response.has_focus() && app.ui.manual_index_input != current_display {
                        app.ui.manual_index_input = current_display;
                    }
                    
                    // Show filtered count when filters are active
                    if app.filter.is_active() {
                        ui.label(format!(
                            "of {} ({} total)",
                            app.filter.filtered_count(),
                            app.dataset.get_image_files().len()
                        ));
                        
                        // Filter status badge
                        ui.label(
                            egui::RichText::new(format!("{} Filtered", Icon::FUNNEL))
                                .color(egui::Color32::from_rgb(100, 149, 237))
                                .strong()
                        );
                        
                        // Quick clear filters button
                        if ui.small_button(format!("{} Clear", Icon::X)).clicked() {
                            app.clear_filters();
                        }
                    } else {
                        ui.label(format!("of {}", app.dataset.get_image_files().len()));
                    }
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
            
            // Navigation buttons removed (moved to image overlay)
            /*
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
            */
            
            ui.add_space(20.0);
            
            // Delete button
            let delete_enabled = !app.dataset.get_image_files().is_empty();
            let delete_btn = ui.add_enabled(
                delete_enabled,
                egui::Button::new(format!("{} Delete Image & Label", Icon::TRASH)).fill(egui::Color32::from_rgb(200, 50, 50)),
            );
            
            if delete_btn.clicked() {
                tracing::info!("[BUTTON] Delete button clicked!");
                app.delete_current_image();
            } else if delete_btn.hovered()
            && ui.input(|i| i.pointer.any_click()) {
                 tracing::warn!("[BUTTON] Delete button HOVERED and CLICKED (raw), but .clicked() is FALSE. Enabled: {}", delete_enabled);
            }
            
            ui.add_space(20.0);
            
            // Batch delete black images button
            let button_text = if app.batch.processing {
                if let Some(stats) = &app.batch.stats {
                    let total = stats.total_scanned.max(stats.current_progress);
                    let percentage = if total > 0 {
                        (stats.current_progress as f32 / total as f32 * 100.0) as u32
                    } else {
                        0
                    };
                    format!("{} Processing... {}%", Icon::MAGIC_WAND, percentage)
                } else {
                    format!("{} Processing...", Icon::MAGIC_WAND)
                }
            } else {
                format!("{} Remove Black Images", Icon::MAGIC_WAND)
            };
            
            let button = egui::Button::new(&button_text).fill(egui::Color32::from_rgb(100, 100, 180));
            if ui.add_enabled(
                !app.dataset.get_image_files().is_empty() && !app.batch.processing,
                button,
            )
            .clicked()
            {
                app.ui.show_batch_delete_confirm = true;
            }
            
            // Cancel button (only visible during batch processing)
            if app.batch.processing
                && ui.button("❌ Cancel").clicked() {
                    app.cancel_batch_processing();
                }
            
            ui.add_space(20.0);

            // Balance analyzer button
            let balance_btn_text = if app.balance.analyzing {
                format!("{} Analyzing...", Icon::CHART_BAR)
            } else {
                format!("{} Analyze Balance", Icon::CHART_BAR)
            };
            
            let balance_button = egui::Button::new(&balance_btn_text).fill(egui::Color32::from_rgb(100, 150, 100));
            if ui.add_enabled(
                !app.dataset.get_image_files().is_empty() && !app.balance.analyzing,
                balance_button,
            )
            .clicked()
            {
                app.analyze_balance();
            }
            
            ui.add_space(20.0);

            
            // Current file name
            if !app.dataset.get_image_files().is_empty() {
                if let Some(filename) = app.dataset.get_image_files()[app.current_index].file_name() {
                    ui.label(format!("{} {}", Icon::FILE, filename.to_string_lossy()));
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
            if let Some(color) = app.image.dominant_color {
                ui.label(egui::RichText::new(format!("{} Dominant Color", Icon::PALETTE))
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
            
            if let Some(label) = &app.image.label {
                // Detection count
                ui.label(egui::RichText::new(format!("{} Detections: {}", Icon::TARGET, label.detections.len()))
                    .strong()
                    .size(16.0));
                
                ui.add_space(10.0);
                
                // Metadata
                if let Some(res) = &label.resolution {
                    ui.label(format!("{} Resolution: {}", Icon::RULER, res));
                }
                if let Some(map) = &label.map {
                    ui.label(format!("{} Map: {}", Icon::MAP_TRIFOLD, map));
                }
                if let Some(time) = &label.timestamp {
                    ui.label(format!("{} Timestamp: {}", Icon::CLOCK, time));
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
        } else if app.filter.is_active() && app.filter.filtered_count() == 0 {
            // Show "No results" message when filter has 0 matches
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    
                    // Main icon and message
                    ui.label(
                        egui::RichText::new(format!("{} No Matching Images", Icon::MAGNIFYING_GLASS))
                            .size(28.0)
                            .color(egui::Color32::from_rgb(150, 150, 150))
                            .strong()
                    );
                    
                    ui.add_space(15.0);
                    
                    // Explanation
                    ui.label(
                        egui::RichText::new("No images match the current filter criteria")
                            .size(16.0)
                            .color(egui::Color32::GRAY)
                    );
                    
                    ui.add_space(20.0);
                    
                    // Show active filter criteria
                    ui.group(|ui| {
                        ui.set_min_width(300.0);
                        ui.label(
                            egui::RichText::new("Active Filters:")
                                .strong()
                                .size(14.0)
                        );
                        ui.add_space(5.0);
                        
                        // Show team filter if not All
                        if app.filter.criteria.team != crate::core::filter::TeamFilter::All {
                            ui.label(format!("• Team: {:?}", app.filter.criteria.team));
                        }
                        
                        // Show player count filter if not Any
                        if app.filter.criteria.player_count != crate::core::filter::PlayerCountFilter::Any {
                            ui.label(format!("• Player Count: {:?}", app.filter.criteria.player_count));
                        }
                    });
                    
                    ui.add_space(20.0);
                    
                    // Action buttons
                    ui.horizontal(|ui| {
                        if ui.button(
                            egui::RichText::new(format!("{} Clear Filters", Icon::X))
                                .size(16.0)
                        ).clicked() {
                            app.clear_filters();
                        }
                        
                        ui.add_space(10.0);
                        
                        if ui.button(
                            egui::RichText::new(format!("{} Modify Filters", Icon::FUNNEL))
                                .size(16.0)
                        ).clicked() {
                            app.ui.show_filter_dialog = true;
                        }
                    });
                });
            });
        } else {
            // Load image if not already loaded
            if app.image.texture.is_none() {
                app.load_current_image(ctx);
            }
            
            // Display the image
            if let Some(texture) = &app.image.texture {
                let available_size = ui.available_size();
                let img_size = texture.size_vec2();
                
                // Handle zoom with Ctrl + mouse wheel
                ui.input(|i| {
                    if i.modifiers.ctrl && i.smooth_scroll_delta.y != 0.0 {
                        // Zoom in/out based on scroll direction
                        // Positive scroll_delta.y = scroll up = zoom in
                        let zoom_delta = i.smooth_scroll_delta.y * 0.001;
                        app.image.zoom_level = (app.image.zoom_level + zoom_delta).clamp(0.5, 3.0);
                    }
                });
                
                // Calculate scaling to fit the image within available space
                let base_scale = ImageRenderer::calculate_image_scale(img_size, available_size);
                
                // Apply zoom level
                let scale = base_scale * app.image.zoom_level;
                
                let scaled_size = img_size * scale;
                
                // Center the image and get the rect where it's drawn
                let available_rect = ui.available_rect_before_wrap();
                let image_rect = egui::Rect::from_center_size(
                    available_rect.center(),
                    scaled_size
                );
                
                // Draw the image
                ui.put(image_rect, egui::Image::new((texture.id(), scaled_size)));
                
                // Draw bounding boxes if label data exists (not in fullscreen mode)
                if !app.ui.fullscreen_mode {
                    if let Some(label) = &app.image.label {
                        ImageRenderer::draw_bounding_boxes(
                            ui.painter(),
                            label,
                            image_rect,
                            scaled_size,
                            &app.config
                        );
                    }
                }

                // Show fullscreen hint overlay
                if app.ui.fullscreen_mode {
                    // Top-center overlay with hint
                    let hint_text = "Press Space to exit fullscreen";
                    let font_id = egui::FontId::proportional(16.0);
                    let galley = ui.painter().layout_no_wrap(
                        hint_text.to_string(),
                        font_id.clone(),
                        egui::Color32::WHITE
                    );
                    
                    let hint_pos = egui::pos2(
                        available_rect.center().x - galley.size().x / 2.0,
                        available_rect.min.y + 20.0
                    );
                    
                    let hint_bg_rect = egui::Rect::from_min_size(
                        hint_pos - egui::vec2(10.0, 5.0),
                        galley.size() + egui::vec2(20.0, 10.0)
                    );
                    
                    ui.painter().rect_filled(
                        hint_bg_rect,
                        4.0,
                        egui::Color32::from_black_alpha(180)
                    );
                    
                    ui.painter().galley(
                        hint_pos,
                        galley,
                        egui::Color32::WHITE
                    );
                }

                // Show zoom indicator when not at 100%
                if (app.image.zoom_level - 1.0).abs() > 0.01 {
                    let zoom_text = format!("{}%", (app.image.zoom_level * 100.0) as i32);
                    let font_id = egui::FontId::proportional(14.0);
                    let galley = ui.painter().layout_no_wrap(
                        zoom_text,
                        font_id.clone(),
                        egui::Color32::WHITE
                    );
                    
                    // Bottom-right corner
                    let zoom_pos = egui::pos2(
                        available_rect.max.x - galley.size().x - 20.0,
                        available_rect.max.y - 30.0
                    );
                    
                    let zoom_bg_rect = egui::Rect::from_min_size(
                        zoom_pos - egui::vec2(5.0, 3.0),
                        galley.size() + egui::vec2(10.0, 6.0)
                    );
                    
                    ui.painter().rect_filled(
                        zoom_bg_rect,
                        4.0,
                        egui::Color32::from_black_alpha(180)
                    );
                    
                    ui.painter().galley(
                        zoom_pos,
                        galley,
                        egui::Color32::WHITE
                    );
                }

                // --- Navigation Overlays (hidden in fullscreen mode) ---
                if !app.ui.fullscreen_mode {
                    // Previous Button (Left)
                    if app.current_index > 0 {
                        let prev_rect = egui::Rect::from_min_size(
                            image_rect.min,
                            egui::vec2(NAVIGATION_OVERLAY_WIDTH, image_rect.height())
                        );
                        
                        let response = ui.allocate_rect(prev_rect, egui::Sense::click());
                        let is_hovered = response.hovered();
                        
                        // Draw background (only on hover)
                        if is_hovered {
                            ui.painter().rect_filled(
                                prev_rect,
                                0.0,
                                egui::Color32::from_black_alpha(OVERLAY_HOVER_ALPHA)
                            );
                        }
                        
                        // Draw arrow icon (always visible, brighter on hover)
                        draw_navigation_arrow(
                            ui.painter(),
                            prev_rect.center(),
                            ArrowDirection::Left,
                            is_hovered
                        );
                        
                        if response.clicked() {
                            app.prev_image();
                        }
                    }

                    // Next Button (Right)
                    if !app.dataset.get_image_files().is_empty() && app.current_index < app.dataset.get_image_files().len() - 1 {
                        let next_rect = egui::Rect::from_min_size(
                            egui::pos2(image_rect.max.x - NAVIGATION_OVERLAY_WIDTH, image_rect.min.y),
                            egui::vec2(NAVIGATION_OVERLAY_WIDTH, image_rect.height())
                        );
                        
                        let response = ui.allocate_rect(next_rect, egui::Sense::click());
                        let is_hovered = response.hovered();
                        
                        // Draw background (only on hover)
                        if is_hovered {
                            ui.painter().rect_filled(
                                next_rect,
                                0.0,
                                egui::Color32::from_black_alpha(OVERLAY_HOVER_ALPHA)
                            );
                        }
                        
                        // Draw arrow icon (always visible, brighter on hover)
                        draw_navigation_arrow(
                            ui.painter(),
                            next_rect.center(),
                            ArrowDirection::Right,
                            is_hovered
                        );
                        
                        if response.clicked() {
                            app.next_image();
                        }
                    }
                }

            } else if let Some(error_msg) = &app.image.load_error {
                // Display error message instead of loading spinner
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new("❌ Failed to Load Image")
                                .size(24.0)
                                .color(egui::Color32::from_rgb(220, 50, 50))
                                .strong()
                        );
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(error_msg)
                                .size(14.0)
                                .color(egui::Color32::GRAY)
                        );
                    });
                });
            } else {
                // Show loading spinner only if no error
                ui.centered_and_justified(|ui| {
                    ui.spinner();
                    ui.label("Loading image...");
                });
            }
        }
    });
}

