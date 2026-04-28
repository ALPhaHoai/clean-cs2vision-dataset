use crate::app::DatasetCleanerApp;
use eframe::egui;
use egui_phosphor::regular as Icon;

/// Format a Unix timestamp as a relative time string (e.g., "2 hours ago")
fn format_relative_time(timestamp_str: &str) -> String {
    // Parse the Unix timestamp
    if let Ok(timestamp) = timestamp_str.parse::<i64>() {
        let timestamp_dt = chrono::DateTime::from_timestamp(timestamp, 0);
        
        if let Some(timestamp_dt) = timestamp_dt {
            let now = chrono::Local::now();
            let duration = now.signed_duration_since(timestamp_dt);
            
            let seconds = duration.num_seconds();
            
            if seconds < 0 {
                return "in the future".to_string();
            } else if seconds < 60 {
                return "just now".to_string();
            } else if seconds < 3600 {
                let minutes = seconds / 60;
                return format!("{} minute{} ago", minutes, if minutes == 1 { "" } else { "s" });
            } else if seconds < 86400 {
                let hours = seconds / 3600;
                return format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" });
            } else if seconds < 2592000 {
                let days = seconds / 86400;
                return format!("{} day{} ago", days, if days == 1 { "" } else { "s" });
            } else if seconds < 31536000 {
                let months = seconds / 2592000;
                return format!("{} month{} ago", months, if months == 1 { "" } else { "s" });
            } else {
                let years = seconds / 31536000;
                return format!("{} year{} ago", years, if years == 1 { "" } else { "s" });
            }
        }
    }
    
    // If parsing fails, return empty string
    String::new()
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
                ui.label(
                    egui::RichText::new(format!("{} Dominant Color", Icon::PALETTE))
                        .strong()
                        .size(16.0),
                );
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    // Color swatch
                    let (rect, _response) =
                        ui.allocate_exact_size(egui::vec2(60.0, 40.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4.0, color);
                    ui.painter().rect_stroke(
                        rect,
                        4.0,
                        egui::Stroke::new(2.0, egui::Color32::from_gray(128)),
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
                ui.label(
                    egui::RichText::new(format!(
                        "{} Detections: {}",
                        Icon::TARGET,
                        label.detections.len()
                    ))
                    .strong()
                    .size(16.0),
                );

                ui.add_space(10.0);

                // Metadata
                if let Some(res) = &label.resolution {
                    ui.label(format!("{} Resolution: {}", Icon::RULER, res));
                }
                if let Some(map) = &label.map {
                    ui.label(format!("{} Map: {}", Icon::MAP_TRIFOLD, map));
                }
                if let Some(loc) = &label.location {
                    ui.label(format!("{} Location: {}", Icon::MAP_PIN, loc));
                }
                if let Some(pos) = &label.position {
                    ui.label(format!("{} Position: {}", Icon::CROSSHAIR, pos));
                }
                if let Some(time) = &label.timestamp {
                    let relative = format_relative_time(time);
                    if !relative.is_empty() {
                        ui.label(format!("{} Timestamp: {} ({})", Icon::CLOCK, time, relative));
                    } else {
                        ui.label(format!("{} Timestamp: {}", Icon::CLOCK, time));
                    }
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Detection details
                if label.detections.is_empty() {
                    ui.label(
                        egui::RichText::new("No players detected")
                            .italics()
                            .color(egui::Color32::GRAY),
                    );
                } else {
                    ui.label(egui::RichText::new("Detected Players:").strong().size(14.0));
                    ui.add_space(5.0);

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (i, detection) in label.detections.iter().enumerate() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    let (class_color, _) =
                                        app.config.get_class_colors(detection.class_id);

                                    ui.label(egui::RichText::new(format!("#{}", i + 1)).strong());
                                    ui.label(
                                        egui::RichText::new(
                                            app.config.get_class_name(detection.class_id),
                                        )
                                        .strong()
                                        .color(class_color),
                                    );
                                });

                                ui.add_space(5.0);

                                ui.label(format!(
                                    "Center: ({:.4}, {:.4})",
                                    detection.x_center, detection.y_center
                                ));
                                ui.label(format!(
                                    "Size: {:.4} × {:.4}",
                                    detection.width, detection.height
                                ));
                            });

                            ui.add_space(5.0);
                        }
                    });
                }
            } else {
                ui.label(
                    egui::RichText::new("No label file found")
                        .italics()
                        .color(egui::Color32::GRAY),
                );
            }
        });
}
