use crate::app::DatasetCleanerApp;
use eframe::egui;
use egui_phosphor::regular as Icon;

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
                if let Some(time) = &label.timestamp {
                    ui.label(format!("{} Timestamp: {}", Icon::CLOCK, time));
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
