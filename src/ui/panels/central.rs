use crate::app::DatasetCleanerApp;
use crate::ui::image_renderer::ImageRenderer;
use eframe::egui;
use egui_phosphor::regular as Icon;

use super::helpers::render_no_filter_results;

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
            egui::Stroke::NONE,
        ));
    }

    painter.add(egui::Shape::convex_polygon(
        points,
        arrow_color,
        egui::Stroke::NONE,
    ));
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
            render_no_filter_results(app, ui);
        } else {
            // Load image if not already loaded
            if app.image.texture.is_none() {
                app.load_current_image(ctx);
            }

            // Display the image
            if let Some(texture) = &app.image.texture {
                let available_size = ui.available_size();
                let img_size = texture.size_vec2();

                // Calculate scaling to fit the image within available space (this is the container size)
                let base_scale = ImageRenderer::calculate_image_scale(img_size, available_size);
                let container_size = img_size * base_scale;

                // Apply zoom level to get the actual image size
                let scale = base_scale * app.image.zoom_level;
                let scaled_size = img_size * scale;

                // Get the available rect for the container
                let available_rect = ui.available_rect_before_wrap();
                let container_rect =
                    egui::Rect::from_center_size(available_rect.center(), container_size);

                // Handle zoom with Ctrl + mouse wheel (only when hovering over the container)
                ctx.input(|i| {
                    if i.modifiers.ctrl && i.smooth_scroll_delta.y != 0.0 {
                        if let Some(pointer_pos) = i.pointer.hover_pos() {
                            // Only zoom if the mouse is over the container
                            if container_rect.contains(pointer_pos) {
                                // Zoom in/out based on scroll direction
                                // Positive scroll_delta.y = scroll up = zoom in
                                let zoom_delta = i.smooth_scroll_delta.y * 0.001;
                                app.image.zoom_level =
                                    (app.image.zoom_level + zoom_delta).clamp(0.5, 3.0);
                            }
                        }
                    }
                });

                // Create a scroll area for the image
                let scroll_response = egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Set minimum size to the container size to ensure centering works
                        ui.set_min_size(container_size);
                        
                        // Center the image within the scroll area
                        ui.centered_and_justified(|ui| {
                            // Add the image
                            let img_response = ui.add(
                                egui::Image::new((texture.id(), scaled_size))
                                    .fit_to_original_size(1.0)
                            );
                            
                            // Get the actual rect where the image was placed
                            let image_rect = img_response.rect;
                            
                            // Draw bounding boxes if label data exists (not in fullscreen mode)
                            if !app.ui.fullscreen_mode {
                                if let Some(label) = &app.image.label {
                                    ImageRenderer::draw_bounding_boxes(
                                        ui.painter(),
                                        label,
                                        image_rect,
                                        img_size,
                                        &app.config,
                                    );
                                }
                            }
                            
                            image_rect
                        }).inner
                    });
                
                let image_rect = scroll_response.inner;

                // Show fullscreen hint overlay
                if app.ui.fullscreen_mode {
                    // Top-center overlay with hint
                    let hint_text = "Press Space to exit fullscreen";
                    let font_id = egui::FontId::proportional(16.0);
                    let galley = ui.painter().layout_no_wrap(
                        hint_text.to_string(),
                        font_id.clone(),
                        egui::Color32::WHITE,
                    );

                    let hint_pos = egui::pos2(
                        available_rect.center().x - galley.size().x / 2.0,
                        available_rect.min.y + 20.0,
                    );

                    let hint_bg_rect = egui::Rect::from_min_size(
                        hint_pos - egui::vec2(10.0, 5.0),
                        galley.size() + egui::vec2(20.0, 10.0),
                    );

                    ui.painter().rect_filled(
                        hint_bg_rect,
                        4.0,
                        egui::Color32::from_black_alpha(180),
                    );

                    ui.painter().galley(hint_pos, galley, egui::Color32::WHITE);
                }

                // Show zoom indicator when not at 100%
                if (app.image.zoom_level - 1.0).abs() > 0.01 {
                    let zoom_text = format!("{}%", (app.image.zoom_level * 100.0) as i32);
                    let font_id = egui::FontId::proportional(14.0);
                    let galley = ui.painter().layout_no_wrap(
                        zoom_text,
                        font_id.clone(),
                        egui::Color32::WHITE,
                    );

                    // Bottom-right corner
                    let zoom_pos = egui::pos2(
                        available_rect.max.x - galley.size().x - 20.0,
                        available_rect.max.y - 30.0,
                    );

                    let zoom_bg_rect = egui::Rect::from_min_size(
                        zoom_pos - egui::vec2(5.0, 3.0),
                        galley.size() + egui::vec2(10.0, 6.0),
                    );

                    ui.painter().rect_filled(
                        zoom_bg_rect,
                        4.0,
                        egui::Color32::from_black_alpha(180),
                    );

                    ui.painter().galley(zoom_pos, galley, egui::Color32::WHITE);
                }

                // --- Navigation Overlays (hidden in fullscreen mode) ---
                if !app.ui.fullscreen_mode {
                    // Previous Button (Left)
                    if app.current_index > 0 {
                        let prev_rect = egui::Rect::from_min_size(
                            container_rect.min,
                            egui::vec2(NAVIGATION_OVERLAY_WIDTH, container_rect.height()),
                        );

                        let response = ui.allocate_rect(prev_rect, egui::Sense::click());
                        let is_hovered = response.hovered();

                        // Draw background (only on hover)
                        if is_hovered {
                            ui.painter().rect_filled(
                                prev_rect,
                                0.0,
                                egui::Color32::from_black_alpha(OVERLAY_HOVER_ALPHA),
                            );
                        }

                        // Draw arrow icon (always visible, brighter on hover)
                        draw_navigation_arrow(
                            ui.painter(),
                            prev_rect.center(),
                            ArrowDirection::Left,
                            is_hovered,
                        );

                        if response.clicked() {
                            app.prev_image();
                        }
                    }

                    // Next Button (Right)
                    if !app.dataset.get_image_files().is_empty()
                        && app.current_index < app.dataset.get_image_files().len() - 1
                    {
                        let next_rect = egui::Rect::from_min_size(
                            egui::pos2(
                                container_rect.max.x - NAVIGATION_OVERLAY_WIDTH,
                                container_rect.min.y,
                            ),
                            egui::vec2(NAVIGATION_OVERLAY_WIDTH, container_rect.height()),
                        );

                        let response = ui.allocate_rect(next_rect, egui::Sense::click());
                        let is_hovered = response.hovered();

                        // Draw background (only on hover)
                        if is_hovered {
                            ui.painter().rect_filled(
                                next_rect,
                                0.0,
                                egui::Color32::from_black_alpha(OVERLAY_HOVER_ALPHA),
                            );
                        }

                        // Draw arrow icon (always visible, brighter on hover)
                        draw_navigation_arrow(
                            ui.painter(),
                            next_rect.center(),
                            ArrowDirection::Right,
                            is_hovered,
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
                                .strong(),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(error_msg)
                                .size(14.0)
                                .color(egui::Color32::GRAY),
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
