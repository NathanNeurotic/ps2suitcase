use eframe::egui::{self, Color32, RichText};

use crate::{ui::theme, IconFlagSelection, PackerApp};
use psu_packer::{
    color_config_to_rgba, color_f_config_to_rgba, rgba_to_color_config, rgba_to_color_f_config,
    sanitize_icon_sys_line, shift_jis_byte_length, ColorConfig, ColorFConfig, IconSysPreset,
    VectorConfig, ICON_SYS_FLAG_OPTIONS, ICON_SYS_PRESETS, ICON_SYS_TITLE_CHAR_LIMIT,
};

const TITLE_INPUT_WIDTH: f32 = (ICON_SYS_TITLE_CHAR_LIMIT as f32) * 9.0;

pub(crate) fn icon_sys_editor(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.heading(theme::display_heading_text(ui, "icon.sys metadata"));
    ui.small("Configure the save icon title, flags, and lighting.");
    ui.add_space(8.0);

    let mut config_changed = false;

    let checkbox = ui.checkbox(&mut app.icon_sys_enabled, "Enable icon.sys metadata");
    let checkbox_changed = checkbox.changed();
    checkbox
        .on_hover_text("Use an existing icon.sys file or generate a new one when packing the PSU.");

    if checkbox_changed {
        config_changed = true;
    }

    if !app.icon_sys_enabled {
        app.icon_sys_use_existing = false;
    } else if app.icon_sys_existing.is_none() {
        app.icon_sys_use_existing = false;
    }

    if app.icon_sys_enabled {
        if let Some(existing_icon) = app.icon_sys_existing.clone() {
            let previous = app.icon_sys_use_existing;
            ui.horizontal(|ui| {
                ui.label("Mode:");
                let use_existing = ui.selectable_value(
                    &mut app.icon_sys_use_existing,
                    true,
                    "Use existing icon.sys",
                );
                if use_existing.changed() {
                    config_changed = true;
                }
                let generate_new = ui.selectable_value(
                    &mut app.icon_sys_use_existing,
                    false,
                    "Generate new icon.sys",
                );
                if generate_new.changed() {
                    config_changed = true;
                }
            });

            if app.icon_sys_use_existing && !previous {
                app.apply_icon_sys_file(&existing_icon);
                config_changed = true;
            }

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
        inner_changed |= title_section(app, ui);
        ui.add_space(12.0);
        inner_changed |= flag_section(app, ui);
        ui.add_space(12.0);
        inner_changed |= presets_section(app, ui);
        ui.add_space(12.0);
        inner_changed |= background_section(app, ui);
        ui.add_space(12.0);
        inner_changed |= lighting_section(app, ui);
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

fn title_section(app: &mut PackerApp, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Title"));
        ui.small("Each line supports up to 16 characters that must round-trip through Shift-JIS");

        egui::Grid::new("icon_sys_title_grid")
            .num_columns(2)
            .spacing(egui::vec2(8.0, 4.0))
            .show(ui, |ui| {
                ui.label("Line 1");
                if title_input(
                    ui,
                    egui::Id::new("icon_sys_title_line1"),
                    &mut app.icon_sys_title_line1,
                ) {
                    changed = true;
                }
                ui.end_row();

                ui.label("Line 2");
                if title_input(
                    ui,
                    egui::Id::new("icon_sys_title_line2"),
                    &mut app.icon_sys_title_line2,
                ) {
                    changed = true;
                }
                ui.end_row();

                ui.label("Preview");
                ui.vertical(|ui| {
                    ui.monospace(format!(
                        "{:<width$}",
                        app.icon_sys_title_line1,
                        width = ICON_SYS_TITLE_CHAR_LIMIT
                    ));
                    ui.monospace(format!(
                        "{:<width$}",
                        app.icon_sys_title_line2,
                        width = ICON_SYS_TITLE_CHAR_LIMIT
                    ));

                    match shift_jis_byte_length(&app.icon_sys_title_line1) {
                        Ok(break_pos) => {
                            ui.small(format!("Shift-JIS byte length: {break_pos}"));
                            ui.small(format!("Line break position: {break_pos}"));
                        }
                        Err(_) => {
                            let warning = RichText::new(
                                "Shift-JIS byte length: invalid (non-encodable characters)",
                            )
                            .color(Color32::RED);
                            ui.small(warning);
                            ui.small(
                                RichText::new("Line break position: -- (invalid Shift-JIS)")
                                    .color(Color32::RED),
                            );
                        }
                    }
                });
                ui.end_row();
            });
    });
    changed
}

fn title_input(ui: &mut egui::Ui, id: egui::Id, value: &mut String) -> bool {
    let mut edit = egui::TextEdit::singleline(value)
        .char_limit(ICON_SYS_TITLE_CHAR_LIMIT)
        .desired_width(TITLE_INPUT_WIDTH);
    edit = edit.id_source(id);

    let response = ui.add(edit);
    let mut changed = false;
    if response.changed() {
        let sanitized = sanitize_icon_sys_line(value, ICON_SYS_TITLE_CHAR_LIMIT);
        if *value != sanitized {
            *value = sanitized;
        }
        changed = true;
    }

    let char_count = value.chars().count();
    ui.small(format!(
        "{char_count} / {ICON_SYS_TITLE_CHAR_LIMIT} characters (Shift-JIS compatible)"
    ));
    changed
}

fn flag_section(app: &mut PackerApp, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Flags"));
        egui::Grid::new("icon_sys_flag_grid")
            .num_columns(2)
            .spacing(egui::vec2(8.0, 4.0))
            .show(ui, |ui| {
                ui.label("Icon type");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_source("icon_sys_flag_combo")
                        .selected_text(app.icon_flag_label())
                        .show_ui(ui, |ui| {
                            for (idx, (_, label)) in ICON_SYS_FLAG_OPTIONS.iter().enumerate() {
                                let response = ui.selectable_value(
                                    &mut app.icon_sys_flag_selection,
                                    IconFlagSelection::Preset(idx),
                                    *label,
                                );
                                if response.changed() {
                                    changed = true;
                                }
                            }
                            let response = ui.selectable_value(
                                &mut app.icon_sys_flag_selection,
                                IconFlagSelection::Custom,
                                "Custom…",
                            );
                            if response.changed() {
                                changed = true;
                            }
                        });

                    if matches!(app.icon_sys_flag_selection, IconFlagSelection::Custom) {
                        let response = ui.add(
                            egui::DragValue::new(&mut app.icon_sys_custom_flag)
                                .clamp_range(0.0..=u16::MAX as f64)
                                .speed(1),
                        );
                        if response.changed() {
                            changed = true;
                        }
                        response.on_hover_text("Enter the raw flag value (0-65535).");
                        ui.label(format!("0x{:04X}", app.icon_sys_custom_flag));
                    }
                });
                ui.end_row();
            });
    });
    changed
}

fn presets_section(app: &mut PackerApp, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Presets"));
        ui.small("Choose a preset to populate the colors and lights automatically.");

        let selected_label = match app.icon_sys_selected_preset.as_deref() {
            Some(id) => find_preset(id)
                .map(|preset| preset.label.to_string())
                .unwrap_or_else(|| format!("Custom ({id})")),
            None => "Manual".to_string(),
        };

        egui::ComboBox::from_id_source("icon_sys_preset_combo")
            .selected_text(selected_label)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(app.icon_sys_selected_preset.is_none(), "Manual")
                    .clicked()
                {
                    app.clear_icon_sys_preset();
                    changed = true;
                }
                for preset in ICON_SYS_PRESETS {
                    let selected = app
                        .icon_sys_selected_preset
                        .as_deref()
                        .map(|id| id == preset.id)
                        .unwrap_or(false);
                    if ui.selectable_label(selected, preset.label).clicked() {
                        apply_preset(app, preset);
                        changed = true;
                    }
                }
            });

        ui.add_space(6.0);
        preset_preview(app, ui);
    });
    changed
}

fn preset_preview(app: &PackerApp, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.label("Background gradient");
        ui.horizontal(|ui| {
            for color in app.icon_sys_background_colors {
                let rgba = color_config_to_rgba(color);
                draw_color_swatch(ui, color32_from_rgba_u8(rgba));
            }
        });

        ui.label("Light colors");
        ui.horizontal(|ui| {
            for color in app.icon_sys_light_colors {
                let rgba = color_f_config_to_rgba(color);
                draw_color_swatch(ui, color32_from_rgba_f32(rgba));
            }
        });

        ui.label("Ambient");
        let ambient = color_f_config_to_rgba(app.icon_sys_ambient_color);
        draw_color_swatch(ui, color32_from_rgba_f32(ambient));
    });
}

fn draw_color_swatch(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(20.0, 14.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 3.0, color);
}

fn background_section(app: &mut PackerApp, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Background"));
        ui.small("Adjust the gradient colors and alpha layer.");

        if ui
            .add(
                egui::DragValue::new(&mut app.icon_sys_background_transparency)
                    .clamp_range(0.0..=255.0)
                    .speed(1)
                    .suffix(" α"),
            )
            .changed()
        {
            app.clear_icon_sys_preset();
            changed = true;
        }

        let mut background_changed = false;
        egui::Grid::new("icon_sys_background_grid")
            .num_columns(2)
            .spacing(egui::vec2(8.0, 4.0))
            .show(ui, |ui| {
                for (index, color) in app.icon_sys_background_colors.iter_mut().enumerate() {
                    ui.label(format!("Color {}", index + 1));
                    let rgba = color_config_to_rgba(*color);
                    let mut display = color32_from_rgba_u8(rgba);
                    if ui.color_edit_button_srgba(&mut display).changed() {
                        let updated = [display.r(), display.g(), display.b(), display.a()];
                        *color = rgba_to_color_config(updated);
                        background_changed = true;
                    }
                    ui.end_row();
                }
            });
        if background_changed {
            app.clear_icon_sys_preset();
            changed = true;
        }
    });
    changed
}

fn lighting_section(app: &mut PackerApp, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Lighting"));
        ui.small("Tweak light directions, colors, and the ambient glow.");

        let mut lighting_changed = false;

        for (index, (color, direction)) in app
            .icon_sys_light_colors
            .iter_mut()
            .zip(app.icon_sys_light_directions.iter_mut())
            .enumerate()
        {
            let mut light_dirty = false;
            ui.collapsing(format!("Light {}", index + 1), |ui| {
                ui.label("Color");
                let mut rgba = color_f_config_to_rgba(*color);
                if ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed() {
                    *color = rgba_to_color_f_config(rgba);
                    light_dirty = true;
                }

                ui.add_space(4.0);
                ui.label("Direction");
                for (label, component) in [
                    ("x", &mut direction.x),
                    ("y", &mut direction.y),
                    ("z", &mut direction.z),
                    ("w", &mut direction.w),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        if ui
                            .add(
                                egui::DragValue::new(component)
                                    .clamp_range(-1.0..=1.0)
                                    .speed(0.01),
                            )
                            .changed()
                        {
                            light_dirty = true;
                        }
                    });
                }
            });
            if light_dirty {
                lighting_changed = true;
            }
            ui.add_space(4.0);
        }

        ui.label("Ambient color");
        let mut ambient = color_f_config_to_rgba(app.icon_sys_ambient_color);
        if ui
            .color_edit_button_rgba_unmultiplied(&mut ambient)
            .changed()
        {
            app.icon_sys_ambient_color = rgba_to_color_f_config(ambient);
            lighting_changed = true;
        }

        if lighting_changed {
            app.clear_icon_sys_preset();
            changed = true;
        }
    });
    changed
}

fn apply_preset(app: &mut PackerApp, preset: &IconSysPreset) {
    app.icon_sys_background_transparency = preset.background_transparency;
    app.icon_sys_background_colors = preset.background_colors;
    app.icon_sys_light_directions = preset.light_directions;
    app.icon_sys_light_colors = preset.light_colors;
    app.icon_sys_ambient_color = preset.ambient_color;
    app.icon_sys_selected_preset = Some(preset.id.to_string());
}

fn find_preset(id: &str) -> Option<&'static IconSysPreset> {
    ICON_SYS_PRESETS.iter().find(|preset| preset.id == id)
}

#[derive(Clone, Debug, PartialEq)]
pub struct IconSysSnapshot {
    pub enabled: bool,
    pub background_transparency: u32,
    pub background_colors: [ColorConfig; 4],
    pub light_directions: [VectorConfig; 3],
    pub light_colors: [ColorFConfig; 3],
    pub ambient_color: ColorFConfig,
    pub selected_preset: Option<String>,
}

pub fn icon_sys_snapshot(app: &PackerApp) -> IconSysSnapshot {
    IconSysSnapshot {
        enabled: app.icon_sys_enabled,
        background_transparency: app.icon_sys_background_transparency,
        background_colors: app.icon_sys_background_colors,
        light_directions: app.icon_sys_light_directions,
        light_colors: app.icon_sys_light_colors,
        ambient_color: app.icon_sys_ambient_color,
        selected_preset: app.icon_sys_selected_preset.clone(),
    }
}

fn color32_from_rgba_u8(rgba: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}

fn color32_from_rgba_f32(rgba: [f32; 4]) -> Color32 {
    let clamp = |value: f32| -> u8 { (value.clamp(0.0, 1.0) * 255.0).round() as u8 };
    Color32::from_rgba_unmultiplied(
        clamp(rgba[0]),
        clamp(rgba[1]),
        clamp(rgba[2]),
        clamp(rgba[3]),
    )
}
