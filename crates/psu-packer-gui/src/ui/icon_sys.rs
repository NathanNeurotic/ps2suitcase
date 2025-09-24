use eframe::egui;

use crate::{ui::theme, PackerApp};
use gui_core::actions::{Action, IconSysAction};
use gui_core::ActionDispatcher;
use icon_sys_ui::{
    background_editor, flag_selector, lighting_editor, preset_selector, title_editor,
    BackgroundSectionState, FlagSectionState, IconSysState, LightingSectionState,
    PresetPreviewData, PresetSectionState, PresetSelection, TitleSectionIds, TitleSectionState,
};

pub(crate) fn icon_sys_editor(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.heading(theme::display_heading_text(ui, "icon.sys metadata"));
    ui.small("Configure the save icon title, flags, and lighting.");
    ui.add_space(8.0);

    let mut config_changed = false;

    let mut icon_sys_enabled = app.icon_sys_enabled;
    let checkbox = ui.checkbox(&mut icon_sys_enabled, "Enable icon.sys metadata");
    let checkbox_changed = checkbox.changed();
    checkbox
        .on_hover_text("Use an existing icon.sys file or generate a new one when packing the PSU.");

    if checkbox_changed {
        let action = if icon_sys_enabled {
            Action::IconSys(IconSysAction::Enable)
        } else {
            Action::IconSys(IconSysAction::Disable)
        };
        app.trigger_action(action);
        config_changed = true;
    }

    if app.icon_sys_enabled {
        if let Some(_existing_icon) = app.icon_sys_existing.clone() {
            ui.horizontal(|ui| {
                ui.label("Mode:");
                let mut use_existing = app.icon_sys_use_existing;
                let use_existing_response =
                    ui.selectable_value(&mut use_existing, true, "Use existing icon.sys");
                let mut generate_new = app.icon_sys_use_existing;
                let generate_new_response =
                    ui.selectable_value(&mut generate_new, false, "Generate new icon.sys");

                if use_existing_response.changed() && use_existing {
                    app.trigger_action(Action::IconSys(IconSysAction::UseExisting));
                    config_changed = true;
                } else if generate_new_response.changed() && !generate_new {
                    app.trigger_action(Action::IconSys(IconSysAction::GenerateNew));
                    config_changed = true;
                }
            });

            if app.icon_sys_use_existing {
                ui.small(concat!(
                    "The existing icon.sys file will be packed without modification. ",
                    "Switch to \"Generate new icon.sys\" to edit metadata.",
                ));
            }
        }
    }

    ui.add_space(8.0);

    let enabled = app.icon_sys_enabled && !app.icon_sys_use_existing;
    let inner_response = ui.add_enabled_ui(enabled, |ui| {
        let mut inner_changed = false;

        let title_response = ui.group(|ui| {
            ui.heading(theme::display_heading_text(ui, "Title"));
            ui.small(
                "Each line supports up to 16 characters that must round-trip through Shift-JIS",
            );
            title_editor(
                ui,
                TitleSectionIds {
                    line1: egui::Id::new("icon_sys_title_line1"),
                    line2: egui::Id::new("icon_sys_title_line2"),
                },
                TitleSectionState {
                    line1: &mut app.icon_sys_title_line1,
                    line2: &mut app.icon_sys_title_line2,
                },
            )
        });
        if title_response.inner.changed {
            inner_changed = true;
        }

        ui.add_space(12.0);

        let flag_response = ui.group(|ui| {
            ui.heading(theme::display_heading_text(ui, "Flags"));
            flag_selector(
                ui,
                FlagSectionState {
                    selection: &mut app.icon_sys_state.flag_selection,
                    custom_flag: &mut app.icon_sys_state.custom_flag,
                },
            )
        });
        if flag_response.inner.changed {
            inner_changed = true;
        }

        ui.add_space(12.0);

        let mut selected_preset = app.icon_sys_state.selected_preset.clone();
        let mut pending_selected: Option<Option<String>> = None;
        {
            let preset_preview = PresetPreviewData {
                background_colors: &app.icon_sys_state.background_colors,
                light_colors: &app.icon_sys_state.light_colors,
                ambient_color: &app.icon_sys_state.ambient_color,
            };
            let preset_response = ui
                .group(|ui| {
                    ui.heading(theme::display_heading_text(ui, "Presets"));
                    ui.small("Choose a preset to populate the colors and lights automatically.");
                    preset_selector(
                        ui,
                        PresetSectionState {
                            selected_preset: &mut selected_preset,
                        },
                        preset_preview,
                    )
                })
                .inner;
            if let Some(selection) = &preset_response.selection {
                match selection {
                    PresetSelection::Manual => {
                        app.icon_sys_state.clear_preset();
                        pending_selected = Some(None);
                        inner_changed = true;
                    }
                    PresetSelection::Preset(preset) => {
                        app.icon_sys_state.apply_preset(preset);
                        pending_selected = Some(app.icon_sys_state.selected_preset.clone());
                        inner_changed = true;
                    }
                }
            }
            if preset_response.changed {
                inner_changed = true;
            }
        }
        if let Some(value) = pending_selected {
            selected_preset = value;
        }
        app.icon_sys_state.selected_preset = selected_preset;

        ui.add_space(12.0);

        let background_response = ui.group(|ui| {
            ui.heading(theme::display_heading_text(ui, "Background"));
            ui.small("Adjust the gradient colors and alpha layer.");
            background_editor(
                ui,
                BackgroundSectionState {
                    transparency: &mut app.icon_sys_state.background_transparency,
                    colors: &mut app.icon_sys_state.background_colors,
                },
            )
        });
        if background_response.inner.changed {
            app.trigger_action(Action::IconSys(IconSysAction::ClearPreset));
            inner_changed = true;
        }

        ui.add_space(12.0);

        let lighting_response = ui.group(|ui| {
            ui.heading(theme::display_heading_text(ui, "Lighting"));
            ui.small("Tweak light directions, colors, and the ambient glow.");
            lighting_editor(
                ui,
                LightingSectionState {
                    light_colors: &mut app.icon_sys_state.light_colors,
                    light_directions: &mut app.icon_sys_state.light_directions,
                    ambient_color: &mut app.icon_sys_state.ambient_color,
                },
            )
        });
        if lighting_response.inner.changed {
            app.trigger_action(Action::IconSys(IconSysAction::ClearPreset));
            inner_changed = true;
        }

        inner_changed
    });

    if inner_response.inner {
        config_changed = true;
    }

    if config_changed {
        app.refresh_psu_toml_editor();
    }
}

pub fn render_icon_sys_editor(app: &mut PackerApp, ui: &mut egui::Ui) {
    icon_sys_editor(app, ui);
}

#[derive(Clone, Debug, PartialEq)]
pub struct IconSysSnapshot {
    pub enabled: bool,
    pub state: IconSysState,
}

pub fn icon_sys_snapshot(app: &PackerApp) -> IconSysSnapshot {
    IconSysSnapshot {
        enabled: app.icon_sys_enabled,
        state: app.icon_sys_state.clone(),
    }
}
