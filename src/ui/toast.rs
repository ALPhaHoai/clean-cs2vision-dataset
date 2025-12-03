use crate::DatasetCleanerApp;
use eframe::egui;

/// Render the toast notification for undo/redo operations
pub fn render_toast_notification(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    let mut should_undo = false;
    let mut should_redo = false;

    // Get the most recent action from undo or redo stacks
    let has_undo = app.undo_manager.can_undo();
    let has_redo = app.undo_manager.can_redo();

    // Only show if there are actions available
    if !has_undo && !has_redo {
        return;
    }

    // Show toast in bottom-left corner
    egui::Window::new("deletion_toast")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .fixed_pos(egui::pos2(20.0, ctx.screen_rect().height() - 120.0))
        .show(ctx, |ui| {
            ui.set_min_width(300.0);

            // Style with background color
            let frame = egui::Frame::none()
                .fill(egui::Color32::from_rgb(45, 45, 48))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(100, 100, 100),
                ))
                .rounding(6.0)
                .inner_margin(12.0);

            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("⟲").size(20.0));
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Undo/Redo Available")
                                .strong()
                                .color(egui::Color32::WHITE),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{} undo{} | {} redo{}",
                                app.undo_manager.undo_count(),
                                if app.undo_manager.undo_count() == 1 {
                                    ""
                                } else {
                                    "s"
                                },
                                app.undo_manager.redo_count(),
                                if app.undo_manager.redo_count() == 1 {
                                    ""
                                } else {
                                    "s"
                                }
                            ))
                            .small()
                            .color(egui::Color32::from_rgb(180, 180, 180)),
                        );
                    });
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    // Undo button
                    if ui
                        .add_enabled(
                            has_undo,
                            egui::Button::new(
                                egui::RichText::new("↶ Undo (Ctrl+Z)").color(egui::Color32::WHITE),
                            )
                            .fill(if has_undo {
                                egui::Color32::from_rgb(70, 130, 220)
                            } else {
                                egui::Color32::from_rgb(50, 50, 50)
                            }),
                        )
                        .clicked()
                    {
                        should_undo = true;
                    }

                    // Redo button
                    if ui
                        .add_enabled(
                            has_redo,
                            egui::Button::new(
                                egui::RichText::new("↷ Redo (Ctrl+Y)").color(egui::Color32::WHITE),
                            )
                            .fill(if has_redo {
                                egui::Color32::from_rgb(70, 130, 220)
                            } else {
                                egui::Color32::from_rgb(50, 50, 50)
                            }),
                        )
                        .clicked()
                    {
                        should_redo = true;
                    }
                });
            });
        });

    // Handle actions outside of the borrow
    if should_undo {
        app.undo_delete();
    }
    if should_redo {
        app.redo_delete();
    }
}
