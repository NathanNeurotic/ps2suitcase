use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use eframe::egui;
use egui_extras::DatePickerButton;

use crate::{ui::theme, PackerApp, TimestampStrategy, TIMESTAMP_FORMAT};
use gui_core::actions::{Action, TimestampAction, TimestampStrategyAction};
use gui_core::ActionDispatcher;

pub(crate) fn metadata_timestamp_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        let default_timestamp = default_timestamp();
        let source_timestamp = app.packer_state.source_timestamp;
        let planned_timestamp = app.packer_state.planned_timestamp_for_current_source();
        let recommended_strategy = recommended_timestamp_strategy(source_timestamp, planned_timestamp);

        ui.small(
            "Deterministic timestamps ensure repeated packs produce identical archives for verification.",
        );
        ui.add_space(6.0);

        let mut strategy = app.packer_state.timestamp_strategy;
        let recommended_badge = |ui: &mut egui::Ui| {
            let badge_text = egui::RichText::new("Recommended")
                .color(egui::Color32::WHITE)
                .background_color(egui::Color32::from_rgb(38, 166, 65))
                .strong();
            ui.add(egui::Label::new(badge_text))
                .on_hover_text("Best choice based on the available metadata");
        };

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let response = ui.radio_value(
                        &mut strategy,
                        TimestampStrategy::None,
                        "No timestamp",
                    );
                    if response.changed()
                        && app.packer_state.timestamp_strategy != TimestampStrategy::None
                        && strategy == TimestampStrategy::None
                    {
                        app.trigger_action(Action::Timestamp(
                            TimestampAction::SelectStrategy(
                                TimestampStrategyAction::None,
                            ),
                        ));
                    }
                });
                ui.label("• Use when verifying contents does not require metadata timestamps.");
                ui.label("• Relies on: no metadata—timestamp field will be omitted.");
            });
        });

        ui.add_space(6.0);

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let response = ui.radio_value(
                        &mut strategy,
                        TimestampStrategy::InheritSource,
                        "Use source timestamp",
                    );
                    if recommended_strategy == Some(TimestampStrategy::InheritSource) {
                        recommended_badge(ui);
                    }
                    if response.changed()
                        && app.packer_state.timestamp_strategy != TimestampStrategy::InheritSource
                        && strategy == TimestampStrategy::InheritSource
                    {
                        app.trigger_action(Action::Timestamp(
                            TimestampAction::SelectStrategy(
                                TimestampStrategyAction::InheritSource,
                            ),
                        ));
                    }
                });
                ui.label("• Use when the loaded source already contains a trusted timestamp.");
                ui.label(format!(
                    "• Relies on: Source timestamp ({}).",
                    availability_text(source_timestamp, "available", "unavailable")
                ));
                if let Some(ts) = source_timestamp {
                    ui.small(format!("  Source value: {}", ts.format(TIMESTAMP_FORMAT)));
                }
            });
        });

        ui.add_space(6.0);

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let response = ui.radio_value(
                        &mut strategy,
                        TimestampStrategy::SasRules,
                        "Use SAS prefix rules",
                    );
                    if recommended_strategy == Some(TimestampStrategy::SasRules) {
                        recommended_badge(ui);
                    }
                    if response.changed()
                        && app.packer_state.timestamp_strategy != TimestampStrategy::SasRules
                        && strategy == TimestampStrategy::SasRules
                    {
                        app.trigger_action(Action::Timestamp(
                            TimestampAction::SelectStrategy(
                                TimestampStrategyAction::SasRules,
                            ),
                        ));
                    }
                });
                ui.label("• Use when project names follow SAS conventions for deterministic scheduling.");
                let project_name = project_name_text(app);
                ui.label(format!(
                    "• Relies on: Project name ({project_name}) and timestamp rules (planned value {}).",
                    availability_text(planned_timestamp, "available", "unavailable")
                ));
                if let Some(ts) = planned_timestamp {
                    ui.small(format!("  Planned value: {}", ts.format(TIMESTAMP_FORMAT)));
                }
            });
        });

        ui.add_space(6.0);

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let response = ui.radio_value(
                        &mut strategy,
                        TimestampStrategy::Manual,
                        "Manual timestamp",
                    );
                    if recommended_strategy == Some(TimestampStrategy::Manual) {
                        recommended_badge(ui);
                    }
                    if response.changed()
                        && app.packer_state.timestamp_strategy != TimestampStrategy::Manual
                        && strategy == TimestampStrategy::Manual
                    {
                        app.trigger_action(Action::Timestamp(
                            TimestampAction::SelectStrategy(
                                TimestampStrategyAction::Manual,
                            ),
                        ));
                    }
                });
                ui.label("• Use when you must pin the archive to an explicit, reviewer-approved timestamp.");
                ui.label("• Relies on: Manual date and time you enter here.");

                if strategy == TimestampStrategy::Manual {
                    if app.packer_state.manual_timestamp.is_none() {
                        app.trigger_action(Action::Timestamp(
                            TimestampAction::SetManualTimestamp(Some(default_timestamp)),
                        ));
                    }
                }

                if strategy == TimestampStrategy::Manual {
                    let mut timestamp = app.packer_state.manual_timestamp.unwrap_or(default_timestamp);
                    let mut date: NaiveDate = timestamp.date();
                    let time = timestamp.time();
                    let mut hour = time.hour();
                    let mut minute = time.minute();
                    let mut second = time.second();
                    let mut changed = false;

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let date_response = ui.add(
                            DatePickerButton::new(&mut date)
                                .id_source("metadata_timestamp_date_picker"),
                        );
                        changed |= date_response.changed();

                        ui.label("Time");
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut hour)
                                    .clamp_range(0..=23)
                                    .suffix(" h"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut minute)
                                    .clamp_range(0..=59)
                                    .suffix(" m"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut second)
                                    .clamp_range(0..=59)
                                    .suffix(" s"),
                            )
                            .changed();
                    });

                    if changed {
                        if let Some(new_time) = NaiveTime::from_hms_opt(hour, minute, second) {
                            timestamp = NaiveDateTime::new(date, new_time);
                            if app.packer_state.manual_timestamp != Some(timestamp) {
                                app.trigger_action(Action::Timestamp(
                                    TimestampAction::SetManualTimestamp(Some(timestamp)),
                                ));
                            }
                        }
                    } else if app.packer_state.manual_timestamp != Some(timestamp) {
                        app.trigger_action(Action::Timestamp(
                            TimestampAction::SetManualTimestamp(Some(timestamp)),
                        ));
                    }

                    if let Some(ts) = app.packer_state.manual_timestamp {
                        ui.small(format!("Selected: {}", ts.format(TIMESTAMP_FORMAT)));
                    }

                    if let Some(planned) = planned_timestamp {
                        if ui.button("Copy planned timestamp").clicked() {
                            if app.packer_state.manual_timestamp != Some(planned) {
                                app.trigger_action(Action::Timestamp(
                                    TimestampAction::SetManualTimestamp(Some(planned)),
                                ));
                            }
                        }
                    }
                }
            });
        });

        if strategy != app.packer_state.timestamp_strategy {
            let strategy_action = match strategy {
                TimestampStrategy::None => TimestampStrategyAction::None,
                TimestampStrategy::InheritSource => TimestampStrategyAction::InheritSource,
                TimestampStrategy::SasRules => TimestampStrategyAction::SasRules,
                TimestampStrategy::Manual => TimestampStrategyAction::Manual,
            };
            app.trigger_action(Action::Timestamp(TimestampAction::SelectStrategy(
                strategy_action,
            )));
        }

        ui.add_space(8.0);

        let summary_title = current_strategy_title(app.packer_state.timestamp_strategy);
        let summary_reason = current_strategy_reason(app, source_timestamp, planned_timestamp);
        let summary_text = format!("Currently using: {summary_title} because {summary_reason}.");

        ui.group(|ui| {
            ui.label(egui::RichText::new(summary_text).strong());
        });

        ui.add_space(6.0);
    });
}

fn recommended_timestamp_strategy(
    source_timestamp: Option<NaiveDateTime>,
    planned_timestamp: Option<NaiveDateTime>,
) -> Option<TimestampStrategy> {
    if source_timestamp.is_some() {
        Some(TimestampStrategy::InheritSource)
    } else if planned_timestamp.is_some() {
        Some(TimestampStrategy::SasRules)
    } else {
        Some(TimestampStrategy::Manual)
    }
}

fn availability_text(
    timestamp: Option<NaiveDateTime>,
    available_text: &str,
    unavailable_text: &str,
) -> String {
    if timestamp.is_some() {
        available_text.to_string()
    } else {
        unavailable_text.to_string()
    }
}

fn project_name_text(app: &PackerApp) -> String {
    let name = app.packer_state.folder_name();
    if name.trim().is_empty() {
        "not set".to_string()
    } else {
        name
    }
}

fn current_strategy_title(strategy: TimestampStrategy) -> &'static str {
    match strategy {
        TimestampStrategy::None => "No timestamp",
        TimestampStrategy::InheritSource => "Inherited source timestamp",
        TimestampStrategy::SasRules => "SAS rules timestamp",
        TimestampStrategy::Manual => "Manual timestamp",
    }
}

fn current_strategy_reason(
    app: &PackerApp,
    source_timestamp: Option<NaiveDateTime>,
    planned_timestamp: Option<NaiveDateTime>,
) -> String {
    match app.packer_state.timestamp_strategy {
        TimestampStrategy::None => {
            "timestamps are intentionally omitted from the archive".to_string()
        }
        TimestampStrategy::InheritSource => match source_timestamp {
            Some(ts) => format!(
                "the loaded source provided {} to preserve",
                ts.format(TIMESTAMP_FORMAT)
            ),
            None => "no source timestamp was found to inherit".to_string(),
        },
        TimestampStrategy::SasRules => match planned_timestamp {
            Some(ts) => format!(
                "SAS rules computed {} for {}",
                ts.format(TIMESTAMP_FORMAT),
                app.packer_state.folder_name()
            ),
            None => "automatic SAS rules could not determine a timestamp".to_string(),
        },
        TimestampStrategy::Manual => match app.packer_state.manual_timestamp {
            Some(ts) => format!("you entered {}", ts.format(TIMESTAMP_FORMAT)),
            None => "a manual timestamp is required until other data is provided".to_string(),
        },
    }
}

pub(crate) fn timestamp_rules_editor(app: &mut PackerApp, ui: &mut egui::Ui) {
    {
        let state = &mut app.packer_state;
        let (ui_state, rules) = (&mut state.timestamp_rules_ui, &state.timestamp_rules);
        ui_state.ensure_matches(rules);
    }

    ui.heading(theme::display_heading_text(ui, "Automatic timestamp rules"));
    ui.small("Adjust deterministic timestamp spacing, category order, and aliases.");

    if let Some(error) = &app.packer_state.timestamp_rules_error {
        ui.add_space(6.0);
        ui.colored_label(egui::Color32::YELLOW, error);
    }

    if let Some(path) = app.packer_state.timestamp_rules_path() {
        ui.label(format!("Configuration file: {}", path.display()));
    } else {
        ui.small("Select a project folder to save these settings alongside psu.toml.");
    }

    if app.packer_state.timestamp_rules_modified {
        ui.colored_label(egui::Color32::LIGHT_YELLOW, "Unsaved changes");
    }

    ui.add_space(8.0);
    egui::Grid::new("timestamp_rules_settings")
        .num_columns(2)
        .spacing(egui::vec2(12.0, 6.0))
        .show(ui, |ui| {
            ui.label("Seconds between items");
            let mut seconds = app.packer_state.timestamp_rules_ui.seconds_between_items();
            if ui
                .add(
                    egui::DragValue::new(&mut seconds)
                        .clamp_range(2..=3600)
                        .speed(1.0),
                )
                .changed()
                && app
                    .packer_state
                    .timestamp_rules_ui
                    .set_seconds_between_items(seconds)
            {
                app.mark_timestamp_rules_modified();
            }
            ui.end_row();

            ui.label("Slots per category");
            let mut slots = app.packer_state.timestamp_rules_ui.slots_per_category();
            if ui
                .add(
                    egui::DragValue::new(&mut slots)
                        .clamp_range(1..=200_000)
                        .speed(10.0),
                )
                .changed()
                && app
                    .packer_state
                    .timestamp_rules_ui
                    .set_slots_per_category(slots)
            {
                app.mark_timestamp_rules_modified();
            }
            ui.end_row();
        });

    ui.add_space(12.0);
    ui.heading(theme::display_heading_text(
        ui,
        "Category order and aliases",
    ));
    ui.small("Toggle canonical aliases to map known unprefixed names to their categories.");
    ui.add_space(6.0);

    let mut move_request: Option<(usize, MoveDirection)> = None;
    let category_len = app.packer_state.timestamp_rules_ui.len();

    for index in 0..category_len {
        let Some(category) = app.packer_state.timestamp_rules_ui.category(index) else {
            continue;
        };
        let key = category.key().to_string();
        let alias_count = category.alias_count();
        let available_aliases = category.available_aliases().to_vec();
        let _ = category;

        let header_title = if alias_count == 1 {
            format!("{key} (1 alias)")
        } else {
            format!("{key} ({alias_count} aliases)")
        };

        let mut aliases_changed = false;
        egui::CollapsingHeader::new(header_title)
            .id_source(format!("timestamp_category_{index}"))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(index > 0, egui::Button::new("Move up"))
                        .clicked()
                    {
                        move_request = Some((index, MoveDirection::Up));
                    }
                    if ui
                        .add_enabled(index + 1 < category_len, egui::Button::new("Move down"))
                        .clicked()
                    {
                        move_request = Some((index, MoveDirection::Down));
                    }
                });

                ui.label("Canonical aliases:");
                if available_aliases.is_empty() {
                    ui.small("No canonical aliases are defined for this category.");
                } else {
                    for alias in &available_aliases {
                        let mut is_selected = app
                            .packer_state
                            .timestamp_rules_ui
                            .category(index)
                            .map(|category| category.is_alias_selected(alias))
                            .unwrap_or(false);
                        if ui.checkbox(&mut is_selected, alias).changed() {
                            if app.packer_state.timestamp_rules_ui.set_alias_selected(
                                index,
                                alias,
                                is_selected,
                            ) {
                                aliases_changed = true;
                            }
                        }
                    }

                    if let Some(warning) = app.packer_state.timestamp_rules_ui.alias_warning(index)
                    {
                        ui.colored_label(egui::Color32::from_rgb(229, 115, 115), warning);
                    }
                }
            });

        if aliases_changed {
            app.mark_timestamp_rules_modified();
        }

        ui.add_space(6.0);
    }

    if let Some((index, direction)) = move_request {
        let moved = match direction {
            MoveDirection::Up => app.packer_state.timestamp_rules_ui.move_category_up(index),
            MoveDirection::Down => app
                .packer_state
                .timestamp_rules_ui
                .move_category_down(index),
        };
        if moved {
            app.mark_timestamp_rules_modified();
        }
    }

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if ui.button("Restore defaults").clicked() {
            app.reset_timestamp_rules_to_default();
        }

        let save_enabled = app.packer_state.folder.is_some();
        if ui
            .add_enabled(save_enabled, egui::Button::new("Save"))
            .clicked()
        {
            match app.packer_state.save_timestamp_rules() {
                Ok(path) => {
                    app.packer_state.status =
                        format!("Saved timestamp rules to {}", path.display());
                    app.clear_error_message();
                    if matches!(
                        app.packer_state.timestamp_strategy,
                        TimestampStrategy::SasRules
                    ) {
                        app.apply_planned_timestamp();
                    }
                }
                Err(err) => app.set_error_message(err),
            }
        }

        if ui
            .add_enabled(save_enabled, egui::Button::new("Reload from disk"))
            .clicked()
        {
            if let Some(folder) = app.packer_state.folder.clone() {
                app.packer_state.load_timestamp_rules_from_folder(&folder);
                if matches!(
                    app.packer_state.timestamp_strategy,
                    TimestampStrategy::SasRules
                ) {
                    app.apply_planned_timestamp();
                }
            }
        }
    });
}

fn default_timestamp() -> NaiveDateTime {
    let now = Local::now().naive_local();
    now.with_nanosecond(0).unwrap_or(now)
}

#[derive(Clone, Copy)]
enum MoveDirection {
    Up,
    Down,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PackerApp, SasPrefix, TimestampStrategy};
    use chrono::{Duration, NaiveDate};
    use eframe::egui;

    #[test]
    fn summary_references_source_when_inheriting() {
        let mut app = PackerApp::default();
        let source = NaiveDate::from_ymd_opt(2024, 1, 2)
            .unwrap()
            .and_hms_opt(3, 4, 5)
            .unwrap();
        app.packer_state.source_timestamp = Some(source);
        app.set_timestamp_strategy(TimestampStrategy::InheritSource);
        app.refresh_timestamp_from_strategy();

        let rendered = render_metadata_text(&mut app);

        assert!(rendered.contains("No timestamp"));
        assert!(rendered.contains("Source timestamp (available)"));
        assert!(rendered.contains(
            "Currently using: Inherited source timestamp because the loaded source provided 2024-01-02 03:04:05 to preserve."
        ));
        assert!(rendered.contains("Recommended"));
    }

    #[test]
    fn summary_references_planned_when_using_sas_rules() {
        let mut app = PackerApp::default();
        app.packer_state.source_timestamp = None;
        app.packer_state.set_selected_prefix(SasPrefix::App);
        app.packer_state.set_folder_base_name("TEST".to_string());
        app.set_timestamp_strategy(TimestampStrategy::SasRules);
        app.refresh_timestamp_from_strategy();

        let rendered = render_metadata_text(&mut app);

        assert!(rendered.contains("Project name (APP_TEST)"));
        assert!(rendered.contains("planned value available"));
        assert!(rendered.contains("SAS rules timestamp because SAS rules computed"));
        assert!(rendered.contains("Recommended"));
    }

    #[test]
    fn manual_summary_updates_after_manual_timestamp_change() {
        let mut app = PackerApp::default();
        app.packer_state.source_timestamp = None;
        app.set_timestamp_strategy(TimestampStrategy::Manual);
        let initial = NaiveDate::from_ymd_opt(2024, 5, 6)
            .unwrap()
            .and_hms_opt(7, 8, 9)
            .unwrap();
        let _ = app.packer_state.set_manual_timestamp(Some(initial));

        let rendered = render_metadata_text(&mut app);
        assert!(rendered.contains(
            "Currently using: Manual timestamp because you entered 2024-05-06 07:08:09."
        ));

        let updated = initial + Duration::minutes(5);
        app.packer_state.set_manual_timestamp(Some(updated));

        let rerendered = render_metadata_text(&mut app);
        assert!(rerendered.contains(
            "Currently using: Manual timestamp because you entered 2024-05-06 07:13:09."
        ));
    }

    fn render_metadata_text(app: &mut PackerApp) -> String {
        let ctx = egui::Context::default();
        ctx.begin_frame(egui::RawInput::default());
        egui::CentralPanel::default().show(&ctx, |ui| {
            metadata_timestamp_section(app, ui);
        });
        let full_output = ctx.end_frame();
        let mut texts = Vec::new();
        collect_text_from_clipped_shapes(&full_output.shapes, &mut texts);
        texts.join("\n")
    }

    fn collect_text_from_clipped_shapes(
        shapes: &[egui::epaint::ClippedShape],
        output: &mut Vec<String>,
    ) {
        for clipped in shapes {
            collect_text_from_shape(&clipped.shape, output);
        }
    }

    fn collect_text_from_shape(shape: &egui::epaint::Shape, output: &mut Vec<String>) {
        match shape {
            egui::epaint::Shape::Vec(shapes) => {
                for nested in shapes {
                    collect_text_from_shape(nested, output);
                }
            }
            egui::epaint::Shape::Text(text_shape) => {
                output.push(text_shape.galley.text().to_string());
            }
            _ => {}
        }
    }
}
