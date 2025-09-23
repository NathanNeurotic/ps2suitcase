use eframe::egui;

use crate::{
    state::{self, EditorTab, PackerApp},
    ui::{self, theme},
};

pub trait View<TState> {
    fn show(&mut self, ui: &mut egui::Ui, state: &mut TState);
}

#[derive(Default)]
pub struct TimestampRulesPanel;

impl View<PackerApp> for TimestampRulesPanel {
    fn show(&mut self, ui: &mut egui::Ui, state: &mut PackerApp) {
        ui::timestamps::timestamp_rules_editor(state, ui);
    }
}

impl eframe::App for PackerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_pack_job();

        if ctx.input(|i| i.viewport().close_requested()) && !self.exit_confirmed {
            self.exit_confirmed = false;
            self.show_exit_confirm = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        self.zoom_factor = self.zoom_factor.clamp(0.5, 2.0);
        ctx.set_pixels_per_point(self.zoom_factor);

        let source_present = self.has_source();
        if !source_present && self.source_present_last_frame {
            self.reset_metadata_fields();
        }
        self.source_present_last_frame = source_present;

        if let Some(progress) = self.pack_progress_value() {
            ctx.request_repaint();
            egui::Window::new("packing_progress")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .frame(egui::Frame::popup(&ctx.style()))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Packing PSU…")
                                .font(theme::display_font(26.0))
                                .color(self.theme.neon_accent),
                        );
                        ui.add_space(8.0);
                        ui.add(
                            egui::ProgressBar::new(progress)
                                .desired_width(200.0)
                                .animate(true),
                        );
                    });
                });
        }

        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::none().fill(self.theme.background))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                theme::draw_vertical_gradient(
                    ui.painter(),
                    rect,
                    self.theme.header_top,
                    self.theme.header_bottom,
                );
                let separator_rect =
                    egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - 2.0), rect.max);
                theme::draw_separator(ui.painter(), separator_rect, self.theme.separator);
                egui::menu::bar(ui, |ui| {
                    ui::file_picker::file_menu(self, ui);
                    ui.menu_button("View", |ui| {
                        if ui.button("Zoom In").clicked() {
                            self.zoom_factor = (self.zoom_factor + 0.1).min(2.0);
                            ui.close_menu();
                        }
                        if ui.button("Zoom Out").clicked() {
                            self.zoom_factor = (self.zoom_factor - 0.1).max(0.5);
                            ui.close_menu();
                        }
                        if ui.button("Reset Zoom").clicked() {
                            self.zoom_factor = 1.0;
                            ui.close_menu();
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);
                        let zoom_text = format!("Zoom: {:.0}%", self.zoom_factor * 100.0);
                        ui.label(egui::RichText::new(zoom_text).color(self.theme.neon_accent));
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(self.theme.background))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                theme::draw_vertical_gradient(
                    ui.painter(),
                    rect,
                    self.theme.footer_top,
                    self.theme.footer_bottom,
                );
                let top_separator =
                    egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.min.y + 2.0));
                theme::draw_separator(ui.painter(), top_separator, self.theme.separator);
                let bottom_separator =
                    egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - 2.0), rect.max);
                theme::draw_separator(ui.painter(), bottom_separator, self.theme.separator);
                ui.add_space(8.0);

                let tab_font = theme::display_font(18.0);
                let tab_bar = ui.horizontal_wrapped(|ui| {
                    let spacing = ui.spacing_mut();
                    spacing.item_spacing.x = 12.0;
                    spacing.item_spacing.y = 8.0;

                    self.editor_tab_button(ui, EditorTab::PsuSettings, ".psu", false, &tab_font);

                    #[cfg(feature = "psu-toml-editor")]
                    {
                        let psu_toml_label = if self.psu_toml_editor.modified {
                            "psu.toml*"
                        } else {
                            "psu.toml"
                        };
                        self.editor_tab_button(
                            ui,
                            EditorTab::PsuToml,
                            psu_toml_label,
                            self.psu_toml_editor.modified,
                            &tab_font,
                        );
                    }

                    let title_cfg_label = if self.title_cfg_editor.modified {
                        "title.cfg*"
                    } else {
                        "title.cfg"
                    };
                    self.editor_tab_button(
                        ui,
                        EditorTab::TitleCfg,
                        title_cfg_label,
                        self.title_cfg_editor.modified,
                        &tab_font,
                    );

                    self.editor_tab_button(ui, EditorTab::IconSys, "icon.sys", false, &tab_font);

                    let timestamp_label = if self.timestamp_rules_modified {
                        "Timestamp rules*"
                    } else {
                        "Timestamp rules"
                    };
                    self.editor_tab_button(
                        ui,
                        EditorTab::TimestampAuto,
                        timestamp_label,
                        self.timestamp_rules_modified,
                        &tab_font,
                    );
                });

                let tab_rect = tab_bar.response.rect;
                let tab_separator = egui::Rect::from_min_max(
                    egui::pos2(rect.min.x, tab_rect.max.y + 4.0),
                    egui::pos2(rect.max.x, tab_rect.max.y + 6.0),
                );
                theme::draw_separator(ui.painter(), tab_separator, self.theme.separator);
                ui.add_space(10.0);

                match self.editor_tab {
                    EditorTab::PsuSettings => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui::centered_column(ui, state::CENTERED_COLUMN_MAX_WIDTH, |ui| {
                                ui::file_picker::folder_section(self, ui);

                                let showing_psu = self.showing_loaded_psu();
                                if showing_psu {
                                    ui.add_space(8.0);
                                    ui::file_picker::loaded_psu_section(self, ui);
                                }

                                ui.add_space(8.0);

                                let two_column_layout = ui.available_width()
                                    >= state::PACK_CONTROLS_TWO_COLUMN_MIN_WIDTH;
                                if two_column_layout {
                                    ui.columns(2, |columns| {
                                        columns[0].vertical(|ui| {
                                            ui::pack_controls::metadata_section(self, ui);
                                            ui.add_space(8.0);
                                            ui::pack_controls::output_section(self, ui);
                                        });

                                        columns[1].vertical(|ui| {
                                            if !showing_psu {
                                                ui::pack_controls::file_filters_section(self, ui);
                                                ui.add_space(8.0);
                                            }
                                            ui::pack_controls::packaging_section(self, ui);
                                        });
                                    });
                                } else {
                                    ui::pack_controls::metadata_section(self, ui);

                                    if !showing_psu {
                                        ui.add_space(8.0);
                                        ui::pack_controls::file_filters_section(self, ui);
                                    }

                                    ui.add_space(8.0);
                                    ui::pack_controls::output_section(self, ui);

                                    ui.add_space(8.0);
                                    ui::pack_controls::packaging_section(self, ui);
                                }
                            });
                        });
                    }
                    #[cfg(feature = "psu-toml-editor")]
                    EditorTab::PsuToml => {
                        let editing_enabled = true; // Allow editing even without a source selection.
                        let save_enabled = self.folder.is_some();
                        let actions = state::text_editor_ui(
                            ui,
                            "psu.toml",
                            editing_enabled,
                            save_enabled,
                            &mut self.psu_toml_editor,
                        );
                        if actions.save_clicked {
                            match state::save_editor_to_disk(
                                self.folder.as_deref(),
                                "psu.toml",
                                &mut self.psu_toml_editor,
                            ) {
                                Ok(path) => {
                                    self.status = format!("Saved {}", path.display());
                                    self.clear_error_message();
                                }
                                Err(err) => {
                                    self.set_error_message(format!(
                                        "Failed to save psu.toml: {err}"
                                    ));
                                }
                            }
                        }
                        if actions.apply_clicked {
                            self.apply_psu_toml_edits();
                        }
                    }
                    EditorTab::TitleCfg => {
                        let editing_enabled = true; // Allow editing even without a source selection.
                        let save_enabled = self.folder.is_some();
                        let actions = state::title_cfg_form_ui(
                            ui,
                            editing_enabled,
                            save_enabled,
                            &mut self.title_cfg_editor,
                        );
                        if actions.save_clicked {
                            match state::save_editor_to_disk(
                                self.folder.as_deref(),
                                "title.cfg",
                                &mut self.title_cfg_editor,
                            ) {
                                Ok(path) => {
                                    self.status = format!("Saved {}", path.display());
                                    self.clear_error_message();
                                }
                                Err(err) => {
                                    self.set_error_message(format!(
                                        "Failed to save title.cfg: {err}"
                                    ));
                                }
                            }
                        }
                        if actions.apply_clicked {
                            self.apply_title_cfg_edits();
                        }
                    }
                    EditorTab::IconSys => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui::centered_column(ui, state::CENTERED_COLUMN_MAX_WIDTH, |ui| {
                                ui::icon_sys::icon_sys_editor(self, ui);
                            });
                        });
                    }
                    EditorTab::TimestampAuto => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui::centered_column(ui, state::CENTERED_COLUMN_MAX_WIDTH, |ui| {
                                let mut timestamp_view = TimestampRulesPanel::default();
                                timestamp_view.show(ui, self);
                            });
                        });
                    }
                }
            });

        ui::dialogs::pack_confirmation(self, ctx);
        ui::dialogs::exit_confirmation(self, ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::TIMESTAMP_FORMAT;
    use chrono::NaiveDateTime;
    use eframe::egui;
    use std::{fs, path::Path, thread, time::Duration};
    use tempfile::tempdir;

    fn write_required_files(folder: &Path) {
        for file in crate::state::REQUIRED_PROJECT_FILES {
            let path = folder.join(file);
            fs::write(&path, b"data").expect("write required file");
        }
    }

    fn wait_for_pack_completion(app: &mut PackerApp) {
        while app.pack_job_active() {
            thread::sleep(Duration::from_millis(10));
            app.poll_pack_job();
        }
    }

    #[test]
    fn timestamp_panel_serializes_rules() {
        let mut app = PackerApp::default();
        app.timestamp_rules_ui.set_seconds_between_items(8);
        app.mark_timestamp_rules_modified();

        let mut panel = TimestampRulesPanel::default();
        let ctx = egui::Context::default();
        let palette = app.theme.clone();
        theme::install(&ctx, &palette);
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.show(ui, &mut app));
        });

        let serialized = app
            .timestamp_rules_ui
            .serialize()
            .expect("serialize timestamp rules");
        assert!(serialized.contains("\"seconds_between_items\": 8"));
    }

    #[test]
    fn pack_job_completes_after_view_interaction() {
        let workspace = tempdir().expect("temp workspace");
        let project_dir = workspace.path().join("project");
        fs::create_dir_all(&project_dir).expect("create project folder");
        write_required_files(&project_dir);

        let mut app = PackerApp::default();
        app.timestamp = Some(
            NaiveDateTime::parse_from_str("2024-01-01 12:00:00", TIMESTAMP_FORMAT)
                .expect("parse timestamp"),
        );

        let mut panel = TimestampRulesPanel::default();
        let ctx = egui::Context::default();
        let palette = app.theme.clone();
        theme::install(&ctx, &palette);
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.show(ui, &mut app));
        });

        let output_path = workspace.path().join("output.psu");
        let config = psu_packer::Config {
            name: "APP_TEST".to_string(),
            timestamp: None,
            include: None,
            exclude: None,
            icon_sys: None,
        };

        app.start_pack_job(project_dir, output_path.clone(), config);
        wait_for_pack_completion(&mut app);

        assert!(output_path.exists());
        assert!(app.error_message.is_none());
    }
}
