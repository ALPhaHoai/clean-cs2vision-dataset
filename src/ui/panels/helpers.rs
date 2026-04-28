use crate::app::DatasetCleanerApp;
use eframe::egui;
use egui_phosphor::regular as Icon;

/// Handle manual index input when user presses Enter
/// Returns true if the input was processed successfully
pub fn handle_manual_index_input(
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

/// Render the "No matching images" UI when filters are active but no results found
pub fn render_no_filter_results(app: &mut DatasetCleanerApp, ui: &mut egui::Ui) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);

            // Main icon and message
            ui.label(
                egui::RichText::new(format!("{} No Matching Images", Icon::MAGNIFYING_GLASS))
                    .size(28.0)
                    .color(egui::Color32::from_rgb(150, 150, 150))
                    .strong(),
            );

            ui.add_space(15.0);

            // Explanation
            ui.label(
                egui::RichText::new("No images match the current filter criteria")
                    .size(16.0)
                    .color(egui::Color32::GRAY),
            );

            ui.add_space(20.0);

            // Show active filter criteria
            ui.group(|ui| {
                ui.set_min_width(300.0);
                ui.label(egui::RichText::new("Active Filters:").strong().size(14.0));
                ui.add_space(5.0);

                // Show team filter if not All
                if app.filter.criteria.team != crate::core::filter::TeamFilter::All {
                    ui.label(format!("• Team: {:?}", app.filter.criteria.team));
                }

                // Show player count filter if not Any
                if app.filter.criteria.player_count != crate::core::filter::PlayerCountFilter::Any {
                    ui.label(format!(
                        "• Player Count: {:?}",
                        app.filter.criteria.player_count
                    ));
                }
            });

            ui.add_space(20.0);

            // Action buttons
            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new(format!("{} Clear Filters", Icon::X)).size(16.0))
                    .clicked()
                {
                    app.clear_filters();
                }

                ui.add_space(10.0);

                if ui
                    .button(
                        egui::RichText::new(format!("{} Modify Filters", Icon::FUNNEL)).size(16.0),
                    )
                    .clicked()
                {
                    app.ui.show_filter_dialog = true;
                }
            });
        });
    });
}
