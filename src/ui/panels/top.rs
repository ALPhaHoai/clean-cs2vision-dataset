use crate::app::DatasetCleanerApp;
use crate::core::dataset::DatasetSplit;
use eframe::egui;
use egui_phosphor::regular as Icon;

use super::helpers::handle_manual_index_input;

/// Render the top panel with navigation and dataset controls
pub fn render_top_panel(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading(format!("{} YOLO Dataset Cleaner", Icon::FOLDERS));

            ui.add_space(20.0);

            if ui
                .button(format!("{} Open Dataset Folder", Icon::FOLDER_OPEN))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    app.load_dataset(path);
                }
            }

            ui.add_space(20.0);

            // Split selection buttons
            if app.dataset.dataset_path().is_some() {
                ui.label("Split:");

                if ui
                    .selectable_label(app.dataset.current_split() == DatasetSplit::Train, "Train")
                    .clicked()
                {
                    app.change_split(DatasetSplit::Train);
                }

                if ui
                    .selectable_label(app.dataset.current_split() == DatasetSplit::Val, "Val")
                    .clicked()
                {
                    app.change_split(DatasetSplit::Val);
                }

                if ui
                    .selectable_label(app.dataset.current_split() == DatasetSplit::Test, "Test")
                    .clicked()
                {
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
                        if let Some(virtual_idx) = app.filter.get_filtered_index(app.current_index)
                        {
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
                            .desired_width(60.0),
                    );

                    // Handle manual input when user presses Enter FIRST before syncing
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        handle_manual_index_input(
                            app,
                            &app.ui.manual_index_input.clone(),
                            &current_display,
                        );
                    }
                    // Sync the input text with current index when not focused and not pressing Enter
                    else if !response.has_focus() && app.ui.manual_index_input != current_display
                    {
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
                                .strong(),
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
