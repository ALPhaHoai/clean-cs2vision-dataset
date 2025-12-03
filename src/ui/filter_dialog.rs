use crate::app::DatasetCleanerApp;
use crate::core::filter::{PlayerCountFilter, TeamFilter};
use eframe::egui;
use egui_phosphor::regular as Icon;

/// Render the filter dialog for configuring image filters
pub fn render_filter_dialog(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    if !app.ui.show_filter_dialog {
        return;
    }

    let mut apply_clicked = false;
    let mut clear_clicked = false;
    let mut close_dialog = false;

    egui::Window::new(format!("{} Filter Images", Icon::FUNNEL))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(350.0);

            // Team Filter Section
            ui.group(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} Team Filter", Icon::USERS))
                        .strong()
                        .size(16.0),
                );
                ui.add_space(5.0);

                ui.horizontal_wrapped(|ui| {
                    let selected_team = &mut app.filter.criteria.team;

                    if ui
                        .selectable_label(*selected_team == TeamFilter::All, "All Teams")
                        .clicked()
                    {
                        *selected_team = TeamFilter::All;
                    }
                    if ui
                        .selectable_label(*selected_team == TeamFilter::TOnly, "T Only")
                        .clicked()
                    {
                        *selected_team = TeamFilter::TOnly;
                    }
                    if ui
                        .selectable_label(*selected_team == TeamFilter::CTOnly, "CT Only")
                        .clicked()
                    {
                        *selected_team = TeamFilter::CTOnly;
                    }
                    if ui
                        .selectable_label(*selected_team == TeamFilter::Both, "Both T & CT")
                        .clicked()
                    {
                        *selected_team = TeamFilter::Both;
                    }
                    if ui
                        .selectable_label(*selected_team == TeamFilter::TExclusive, "T Exclusive")
                        .clicked()
                    {
                        *selected_team = TeamFilter::TExclusive;
                    }
                    if ui
                        .selectable_label(*selected_team == TeamFilter::CTExclusive, "CT Exclusive")
                        .clicked()
                    {
                        *selected_team = TeamFilter::CTExclusive;
                    }
                });
            });

            ui.add_space(10.0);

            // Player Count Filter Section
            ui.group(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} Player Count", Icon::USER))
                        .strong()
                        .size(16.0),
                );
                ui.add_space(5.0);

                ui.horizontal_wrapped(|ui| {
                    let selected_count = &mut app.filter.criteria.player_count;

                    if ui
                        .selectable_label(*selected_count == PlayerCountFilter::Any, "Any")
                        .clicked()
                    {
                        *selected_count = PlayerCountFilter::Any;
                    }
                    if ui
                        .selectable_label(*selected_count == PlayerCountFilter::Single, "Single")
                        .clicked()
                    {
                        *selected_count = PlayerCountFilter::Single;
                    }
                    if ui
                        .selectable_label(
                            *selected_count == PlayerCountFilter::Multiple,
                            "Multiple (2+)",
                        )
                        .clicked()
                    {
                        *selected_count = PlayerCountFilter::Multiple;
                    }
                    if ui
                        .selectable_label(
                            *selected_count == PlayerCountFilter::Background,
                            "Background (No Players)",
                        )
                        .clicked()
                    {
                        *selected_count = PlayerCountFilter::Background;
                    }
                });
            });

            ui.add_space(15.0);

            // Preview count (estimate based on current criteria)
            if app.filter.criteria.is_active() {
                ui.separator();
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{} Filter Preview:", Icon::MAGNIFYING_GLASS))
                            .italics(),
                    );
                    if app.filter.total_count > 0 {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} / {} images",
                                app.filter.filtered_count(),
                                app.filter.total_count
                            ))
                            .strong()
                            .color(egui::Color32::from_rgb(100, 149, 237)),
                        );
                    } else {
                        ui.label(egui::RichText::new("Click 'Apply' to see results").italics());
                    }
                });
                ui.add_space(10.0);
            }

            ui.separator();
            ui.add_space(10.0);

            // Action buttons
            ui.horizontal(|ui| {
                if ui
                    .button(
                        egui::RichText::new(format!("{} Apply Filters", Icon::CHECK)).size(14.0),
                    )
                    .clicked()
                {
                    apply_clicked = true;
                    close_dialog = true;
                }

                if ui
                    .button(egui::RichText::new(format!("{} Clear All", Icon::X)).size(14.0))
                    .clicked()
                {
                    clear_clicked = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        close_dialog = true;
                    }
                });
            });

            ui.add_space(5.0);

            // Hint text
            ui.label(
                egui::RichText::new("Press Escape to close")
                    .size(11.0)
                    .italics()
                    .color(egui::Color32::GRAY),
            );
        });

    // Handle actions after the dialog  is drawn
    if apply_clicked {
        app.apply_filters();
    }

    if clear_clicked {
        app.filter.criteria.clear();
        // Optionally apply immediately after clearing
        if !app.filter.filtered_indices.is_empty() {
            app.clear_filters();
            app.apply_filters(); // Resets to show all
        }
    }

    if close_dialog {
        app.ui.show_filter_dialog = false;
    }
}
