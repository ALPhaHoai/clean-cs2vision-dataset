use crate::DatasetCleanerApp;
use eframe::egui;

/// Render a placeholder filter dialog
pub fn render_filter_dialog(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    if !app.show_filter_dialog {
        return;
    }

    egui::Window::new("🔍 Filter Images")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);

                ui.label(
                    egui::RichText::new("Filter Feature Coming Soon!")
                        .size(18.0)
                        .strong(),
                );

                ui.add_space(10.0);

                ui.label(egui::RichText::new("This feature will allow you to:").size(14.0));

                ui.add_space(5.0);

                ui.label("• Filter images by class");
                ui.label("• Filter by detection count");
                ui.label("• Filter by resolution");
                ui.label("• Search by filename");

                ui.add_space(15.0);

                ui.label(
                    egui::RichText::new("Press Escape or click OK to close")
                        .size(12.0)
                        .italics()
                        .color(egui::Color32::GRAY),
                );

                ui.add_space(10.0);

                if ui.button("OK").clicked() {
                    app.show_filter_dialog = false;
                }
            });
        });
}
