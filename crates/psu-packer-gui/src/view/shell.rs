use eframe::egui::{self, Margin, RichText};

use super::View;

use crate::{
    state::{self, PackerApp},
    ui::{self, theme},
};
use gui_core::actions::{Action, EditorAction};
use gui_core::shell::{self, ContentPane, ContextPanel, NavigationItem, ShellStyle};

pub fn show_shell(ctx: &egui::Context, app: &mut PackerApp) {
    let style = shell_style(app);
    let navigation = navigation_items(app);
    let mut content = content_panes();
    let mut trailing = trailing_panels(&style);
    let mut leading: [ContextPanel<'static, PackerApp>; 0] = [];
    let active_intent = app.active_editor;
    shell::show_shell(
        ctx,
        app,
        &style,
        &navigation,
        &mut leading,
        &mut trailing,
        &active_intent,
        &mut content,
    );
}

fn shell_style(app: &PackerApp) -> ShellStyle {
    let mut style = ShellStyle::default();
    let nav_inner_margin = Margin::symmetric(12, 10);
    let nav_rounding = egui::CornerRadius::same(10);

    style.nav_width = 240.0;
    style.navigation_panel_frame = egui::Frame::default()
        .fill(app.theme.panel.gamma_multiply(0.6))
        .inner_margin(Margin::symmetric(12, 16))
        .stroke(egui::Stroke::new(
            1.0,
            app.theme.separator.gamma_multiply(0.4),
        ));
    style.navigation_item_frame = egui::Frame::default()
        .fill(app.theme.panel.gamma_multiply(0.75))
        .corner_radius(nav_rounding)
        .inner_margin(nav_inner_margin.clone());
    style.navigation_selected_item_frame = egui::Frame::default()
        .fill(app.theme.neon_accent.gamma_multiply(0.25))
        .corner_radius(nav_rounding)
        .inner_margin(nav_inner_margin);
    style.navigation_text = app.theme.text_primary;
    style.navigation_selected_text = app.theme.neon_accent;
    style.navigation_alert_text = egui::Color32::from_rgb(255, 198, 92);
    style.content_frame = egui::Frame::default()
        .fill(app.theme.background)
        .inner_margin(Margin::symmetric(16, 12));
    style.context_panel_frame = egui::Frame::default()
        .fill(app.theme.panel.gamma_multiply(0.85))
        .inner_margin(Margin::symmetric(14, 16))
        .stroke(egui::Stroke::new(
            1.0,
            app.theme.separator.gamma_multiply(0.6),
        ))
        .corner_radius(egui::CornerRadius::same(8));
    style
}

fn navigation_items(app: &PackerApp) -> Vec<NavigationItem<'static, EditorAction>> {
    let mut items = Vec::new();

    items.push(
        NavigationItem::new(
            "nav_psu_settings",
            ".psu",
            EditorAction::PsuSettings,
            Action::OpenEditor(EditorAction::PsuSettings),
        )
        .with_description("Project sources and packaging controls."),
    );

    #[cfg(feature = "psu-toml-editor")]
    {
        items.push(
            NavigationItem::new(
                "nav_psu_toml",
                "psu.toml",
                EditorAction::PsuToml,
                Action::OpenEditor(EditorAction::PsuToml),
            )
            .with_description("Advanced metadata overrides.")
            .alert(app.psu_toml_editor.modified),
        );
    }

    items.push(
        NavigationItem::new(
            "nav_title_cfg",
            "title.cfg",
            EditorAction::TitleCfg,
            Action::OpenEditor(EditorAction::TitleCfg),
        )
        .with_description("Title metadata editor.")
        .alert(app.title_cfg_editor.modified),
    );

    items.push(
        NavigationItem::new(
            "nav_icon_sys",
            "icon.sys",
            EditorAction::IconSys,
            Action::OpenEditor(EditorAction::IconSys),
        )
        .with_description("Icon configuration and presets."),
    );

    items.push(
        NavigationItem::new(
            "nav_timestamp_automation",
            "Timestamp automation",
            EditorAction::TimestampAutomation,
            Action::OpenEditor(EditorAction::TimestampAutomation),
        )
        .with_description("Rules for generated timestamps.")
        .alert(app.packer_state.timestamp_rules_modified),
    );

    items
}

fn content_panes<'a>() -> Vec<ContentPane<'a, PackerApp, EditorAction>> {
    let mut panes: Vec<ContentPane<'a, PackerApp, EditorAction>> = Vec::new();

    panes.push(ContentPane::new(EditorAction::PsuSettings, |ui, app| {
        prepare_shell_body(ui, app);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui::centered_column(ui, state::CENTERED_COLUMN_MAX_WIDTH, |ui| {
                ui::file_picker::folder_section(app, ui);

                let showing_psu = app.showing_loaded_psu();
                if showing_psu {
                    ui.add_space(8.0);
                    ui::file_picker::loaded_psu_section(app, ui);
                }

                ui.add_space(8.0);

                let two_column_layout =
                    ui.available_width() >= state::PACK_CONTROLS_TWO_COLUMN_MIN_WIDTH;
                if two_column_layout {
                    ui.columns(2, |columns| {
                        columns[0].vertical(|ui| {
                            ui::pack_controls::metadata_section(app, ui);
                            ui.add_space(8.0);
                            ui::pack_controls::output_section(app, ui);
                        });

                        columns[1].vertical(|ui| {
                            if !showing_psu {
                                ui::pack_controls::file_filters_section(app, ui);
                                ui.add_space(8.0);
                            }
                            ui::pack_controls::packaging_section(app, ui);
                        });
                    });
                } else {
                    ui::pack_controls::metadata_section(app, ui);

                    if !showing_psu {
                        ui.add_space(8.0);
                        ui::pack_controls::file_filters_section(app, ui);
                    }

                    ui.add_space(8.0);
                    ui::pack_controls::output_section(app, ui);

                    ui.add_space(8.0);
                    ui::pack_controls::packaging_section(app, ui);
                }
            });
        });
    }));

    #[cfg(feature = "psu-toml-editor")]
    panes.push(ContentPane::new(EditorAction::PsuToml, |ui, app| {
        prepare_shell_body(ui, app);
        let editing_enabled = true; // Allow editing even without a source selection.
        let save_enabled = app.packer_state.folder.is_some();
        let actions = state::text_editor_ui(
            ui,
            "psu.toml",
            editing_enabled,
            save_enabled,
            &mut app.psu_toml_editor,
        );
        if actions.save_clicked {
            let folder = app.packer_state.folder.as_deref();
            match state::save_editor_to_disk(folder, "psu.toml", &mut app.psu_toml_editor) {
                Ok(path) => {
                    app.packer_state.status = format!("Saved {}", path.display());
                    app.clear_error_message();
                }
                Err(err) => {
                    app.set_error_message(format!("Failed to save psu.toml: {err}"));
                }
            }
        }
        if actions.apply_clicked {
            app.apply_psu_toml_edits();
        }
    }));

    panes.push(ContentPane::new(EditorAction::TitleCfg, |ui, app| {
        prepare_shell_body(ui, app);
        let editing_enabled = true; // Allow editing even without a source selection.
        let save_enabled = app.packer_state.folder.is_some();
        let actions =
            state::title_cfg_form_ui(ui, editing_enabled, save_enabled, &mut app.title_cfg_editor);
        if actions.save_clicked {
            let folder = app.packer_state.folder.clone();
            match state::save_editor_to_disk(
                folder.as_deref(),
                "title.cfg",
                &mut app.title_cfg_editor,
            ) {
                Ok(path) => {
                    app.packer_state.status = format!("Saved {}", path.display());
                    app.clear_error_message();
                }
                Err(err) => {
                    app.set_error_message(format!("Failed to save title.cfg: {err}"));
                }
            }
        }
        if actions.apply_clicked {
            app.apply_title_cfg_edits();
        }
    }));

    panes.push(ContentPane::new(EditorAction::IconSys, |ui, app| {
        prepare_shell_body(ui, app);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui::centered_column(ui, state::CENTERED_COLUMN_MAX_WIDTH, |ui| {
                ui::icon_sys::icon_sys_editor(app, ui);
            });
        });
    }));

    panes.push(ContentPane::new(
        EditorAction::TimestampAutomation,
        |ui, app| {
            prepare_shell_body(ui, app);
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui::centered_column(ui, state::CENTERED_COLUMN_MAX_WIDTH, |ui| {
                    let mut timestamp_view = super::TimestampRulesPanel::default();
                    timestamp_view.show(ui, app);
                });
            });
        },
    ));

    panes
}

fn trailing_panels(style: &ShellStyle) -> Vec<ContextPanel<'static, PackerApp>> {
    let mut panels = Vec::new();
    let frame = style.context_panel_frame.clone();

    panels.push(ContextPanel::new(
        "shell_project_overview",
        280.0,
        frame.clone(),
        |ui: &mut egui::Ui, app: &mut PackerApp| {
            ui.vertical(|ui| {
                ui.add_sized(
                    [ui.available_width(), 0.0],
                    egui::Label::new(theme::display_heading_text(ui, "Project")),
                );
                ui.add_space(6.0);
                if let Some(folder) = app.packer_state.folder.as_ref() {
                    ui.label(RichText::new("Source folder").strong());
                    ui.label(folder.display().to_string());
                } else if let Some(psu) = app.packer_state.loaded_psu_path.as_ref() {
                    ui.label(RichText::new("Loaded PSU").strong());
                    ui.label(psu.display().to_string());
                } else {
                    ui.label("Select a project folder or PSU archive to begin.");
                }

                if !app.packer_state.output.is_empty() {
                    ui.add_space(6.0);
                    ui.label(RichText::new("Output destination").strong());
                    ui.label(&app.packer_state.output);
                }

                if !app.packer_state.missing_required_project_files.is_empty() {
                    ui.add_space(6.0);
                    ui.label(RichText::new("Missing assets").strong());
                    for missing in &app.packer_state.missing_required_project_files {
                        ui.label(format!("• {}", missing.name));
                    }
                }
            });
        },
    ));

    panels.push(ContextPanel::new(
        "shell_activity",
        280.0,
        frame,
        |ui: &mut egui::Ui, app: &mut PackerApp| {
            ui.vertical(|ui| {
                ui.add_sized(
                    [ui.available_width(), 0.0],
                    egui::Label::new(theme::display_heading_text(ui, "Activity")),
                );
                ui.add_space(6.0);
                if app.is_pack_running() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Packing PSU…");
                    });
                }
                if let Some(error) = &app.packer_state.error_message {
                    ui.colored_label(egui::Color32::from_rgb(255, 102, 102), error);
                }
                if !app.packer_state.status.is_empty() {
                    ui.label(&app.packer_state.status);
                }
                if app.packer_state.pending_pack_action.is_some() {
                    ui.add_space(6.0);
                    ui.label("A pack operation is awaiting confirmation.");
                }
            });
        },
    ));

    panels
}

fn prepare_shell_body(ui: &mut egui::Ui, app: &PackerApp) {
    let rect = ui.max_rect();
    theme::draw_vertical_gradient(
        ui.painter(),
        rect,
        app.theme.footer_top,
        app.theme.footer_bottom,
    );

    let top_separator =
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.min.y + 2.0));
    theme::draw_separator(ui.painter(), top_separator, app.theme.separator);

    let bottom_separator =
        egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - 2.0), rect.max);
    theme::draw_separator(ui.painter(), bottom_separator, app.theme.separator);

    ui.add_space(12.0);
}
