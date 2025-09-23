use egui::{self, Color32, RichText};
use psu_packer::{
    color_config_to_rgba, color_f_config_to_rgba, rgba_to_color_config, rgba_to_color_f_config,
    sanitize_icon_sys_line, shift_jis_byte_length, ColorConfig, ColorFConfig, IconSysPreset,
    VectorConfig, ICON_SYS_FLAG_OPTIONS, ICON_SYS_PRESETS, ICON_SYS_TITLE_CHAR_LIMIT,
};

const TITLE_INPUT_WIDTH: f32 = (ICON_SYS_TITLE_CHAR_LIMIT as f32) * 9.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconFlagSelection {
    Preset(usize),
    Custom,
}

pub struct TitleSectionState<'a> {
    pub line1: &'a mut String,
    pub line2: &'a mut String,
}

pub struct TitleSectionIds {
    pub line1: egui::Id,
    pub line2: egui::Id,
}

#[derive(Default)]
pub struct SectionResponse {
    pub changed: bool,
}

pub struct TitleSectionResponse {
    pub changed: bool,
}

pub fn title_editor(
    ui: &mut egui::Ui,
    ids: TitleSectionIds,
    mut state: TitleSectionState<'_>,
) -> TitleSectionResponse {
    let mut changed = false;
    egui::Grid::new("icon_sys_title_grid")
        .num_columns(2)
        .spacing(egui::vec2(8.0, 4.0))
        .show(ui, |ui| {
            ui.label("Line 1");
            if title_input(ui, ids.line1, &mut state.line1) {
                changed = true;
            }
            ui.end_row();

            ui.label("Line 2");
            if title_input(ui, ids.line2, &mut state.line2) {
                changed = true;
            }
            ui.end_row();

            ui.label("Preview");
            ui.vertical(|ui| {
                ui.monospace(format!(
                    "{:<width$}",
                    state.line1,
                    width = ICON_SYS_TITLE_CHAR_LIMIT
                ));
                ui.monospace(format!(
                    "{:<width$}",
                    state.line2,
                    width = ICON_SYS_TITLE_CHAR_LIMIT
                ));

                match shift_jis_byte_length(&state.line1) {
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

    TitleSectionResponse { changed }
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

pub struct FlagSectionState<'a> {
    pub selection: &'a mut IconFlagSelection,
    pub custom_flag: &'a mut u16,
}

pub fn flag_selector(ui: &mut egui::Ui, state: FlagSectionState<'_>) -> SectionResponse {
    let mut changed = false;
    egui::Grid::new("icon_sys_flag_grid")
        .num_columns(2)
        .spacing(egui::vec2(8.0, 4.0))
        .show(ui, |ui| {
            ui.label("Icon type");
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("icon_sys_flag_combo")
                    .selected_text(icon_flag_label(*state.selection, *state.custom_flag))
                    .show_ui(ui, |ui| {
                        for (idx, (_, label)) in ICON_SYS_FLAG_OPTIONS.iter().enumerate() {
                            let response = ui.selectable_value(
                                state.selection,
                                IconFlagSelection::Preset(idx),
                                *label,
                            );
                            if response.changed() {
                                changed = true;
                            }
                        }
                        let response = ui.selectable_value(
                            state.selection,
                            IconFlagSelection::Custom,
                            "Custom…",
                        );
                        if response.changed() {
                            changed = true;
                        }
                    });

                if matches!(state.selection, IconFlagSelection::Custom) {
                    let response = ui.add(
                        egui::DragValue::new(state.custom_flag)
                            .range(0.0..=u16::MAX as f64)
                            .speed(1),
                    );
                    if response.changed() {
                        changed = true;
                    }
                    response.on_hover_text("Enter the raw flag value (0-65535).");
                    ui.label(format!("0x{:04X}", *state.custom_flag));
                }
            });
            ui.end_row();
        });

    SectionResponse { changed }
}

pub fn icon_flag_label(selection: IconFlagSelection, custom_flag: u16) -> String {
    match selection {
        IconFlagSelection::Preset(index) => ICON_SYS_FLAG_OPTIONS
            .get(index)
            .map(|(_, label)| (*label).to_string())
            .unwrap_or_else(|| format!("Preset {index}")),
        IconFlagSelection::Custom => format!("Custom (0x{:04X})", custom_flag),
    }
}

pub fn selected_icon_flag_value(
    selection: IconFlagSelection,
    custom_flag: u16,
) -> Result<u16, String> {
    match selection {
        IconFlagSelection::Preset(index) => ICON_SYS_FLAG_OPTIONS
            .get(index)
            .map(|(value, _)| *value)
            .ok_or_else(|| "Invalid icon.sys flag selection".to_string()),
        IconFlagSelection::Custom => Ok(custom_flag),
    }
}

pub struct PresetSectionState<'a> {
    pub selected_preset: &'a mut Option<String>,
}

pub struct PresetPreviewData<'a> {
    pub background_colors: &'a [ColorConfig; 4],
    pub light_colors: &'a [ColorFConfig; 3],
    pub ambient_color: &'a ColorFConfig,
}

pub enum PresetSelection<'a> {
    Manual,
    Preset(&'a IconSysPreset),
}

pub struct PresetSectionResponse<'a> {
    pub changed: bool,
    pub selection: Option<PresetSelection<'a>>,
}

pub fn preset_selector<'a>(
    ui: &mut egui::Ui,
    state: PresetSectionState<'a>,
    preview: PresetPreviewData<'_>,
) -> PresetSectionResponse<'a> {
    let mut changed = false;
    let mut selection = None;

    let selected_label = match state.selected_preset.as_deref() {
        Some(id) => find_preset(id)
            .map(|preset| preset.label.to_string())
            .unwrap_or_else(|| format!("Custom ({id})")),
        None => "Manual".to_string(),
    };

    egui::ComboBox::from_id_salt("icon_sys_preset_combo")
        .selected_text(selected_label)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(state.selected_preset.is_none(), "Manual")
                .clicked()
            {
                *state.selected_preset = None;
                changed = true;
                selection = Some(PresetSelection::Manual);
            }
            for preset in ICON_SYS_PRESETS {
                let selected = state
                    .selected_preset
                    .as_deref()
                    .map(|id| id == preset.id)
                    .unwrap_or(false);
                if ui.selectable_label(selected, preset.label).clicked() {
                    *state.selected_preset = Some(preset.id.to_string());
                    changed = true;
                    selection = Some(PresetSelection::Preset(preset));
                }
            }
        });

    ui.add_space(6.0);
    preset_preview(ui, preview);

    PresetSectionResponse { changed, selection }
}

fn preset_preview(ui: &mut egui::Ui, preview: PresetPreviewData<'_>) {
    ui.vertical(|ui| {
        ui.label("Background gradient");
        ui.horizontal(|ui| {
            for color in preview.background_colors {
                let rgba = color_config_to_rgba(*color);
                draw_color_swatch(ui, color32_from_rgba_u8(rgba));
            }
        });

        ui.label("Light colors");
        ui.horizontal(|ui| {
            for color in preview.light_colors {
                let rgba = color_f_config_to_rgba(*color);
                draw_color_swatch(ui, color32_from_rgba_f32(rgba));
            }
        });

        ui.label("Ambient");
        let ambient = color_f_config_to_rgba(*preview.ambient_color);
        draw_color_swatch(ui, color32_from_rgba_f32(ambient));
    });
}

fn draw_color_swatch(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(20.0, 14.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 3.0, color);
}

pub struct BackgroundSectionState<'a> {
    pub transparency: &'a mut u32,
    pub colors: &'a mut [ColorConfig; 4],
}

pub fn background_editor(ui: &mut egui::Ui, state: BackgroundSectionState<'_>) -> SectionResponse {
    let mut changed = false;

    if ui
        .add(
            egui::DragValue::new(&mut *state.transparency)
                .range(0.0..=255.0)
                .speed(1)
                .suffix(" α"),
        )
        .changed()
    {
        changed = true;
    }

    let mut background_changed = false;
    egui::Grid::new("icon_sys_background_grid")
        .num_columns(2)
        .spacing(egui::vec2(8.0, 4.0))
        .show(ui, |ui| {
            for (index, color) in state.colors.iter_mut().enumerate() {
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
        changed = true;
    }

    SectionResponse { changed }
}

pub struct LightingSectionState<'a> {
    pub light_colors: &'a mut [ColorFConfig; 3],
    pub light_directions: &'a mut [VectorConfig; 3],
    pub ambient_color: &'a mut ColorFConfig,
}

pub fn lighting_editor(ui: &mut egui::Ui, state: LightingSectionState<'_>) -> SectionResponse {
    let mut changed = false;

    for (index, (color, direction)) in state
        .light_colors
        .iter_mut()
        .zip(state.light_directions.iter_mut())
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
                                .range(-1.0..=1.0)
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
            changed = true;
        }
        ui.add_space(4.0);
    }

    ui.label("Ambient color");
    let mut ambient = color_f_config_to_rgba(*state.ambient_color);
    if ui
        .color_edit_button_rgba_unmultiplied(&mut ambient)
        .changed()
    {
        *state.ambient_color = rgba_to_color_f_config(ambient);
        changed = true;
    }

    SectionResponse { changed }
}

fn find_preset(id: &str) -> Option<&'static IconSysPreset> {
    ICON_SYS_PRESETS.iter().find(|preset| preset.id == id)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_editor_renders() {
        let ctx = egui::Context::default();
        ctx.begin_frame(egui::RawInput::default());
        egui::CentralPanel::default().show(&ctx, |ui| {
            let mut line1 = String::from("HELLO");
            let mut line2 = String::from("WORLD");
            let ids = TitleSectionIds {
                line1: egui::Id::new("line1"),
                line2: egui::Id::new("line2"),
            };
            let state = TitleSectionState {
                line1: &mut line1,
                line2: &mut line2,
            };
            let response = title_editor(ui, ids, state);
            assert!(!response.changed);
        });
        ctx.end_frame();
    }

    #[test]
    fn flag_selector_renders() {
        let ctx = egui::Context::default();
        ctx.begin_frame(egui::RawInput::default());
        egui::CentralPanel::default().show(&ctx, |ui| {
            let mut selection = IconFlagSelection::Preset(0);
            let mut custom_flag = 0u16;
            let response = flag_selector(
                ui,
                FlagSectionState {
                    selection: &mut selection,
                    custom_flag: &mut custom_flag,
                },
            );
            assert!(!response.changed);
        });
        ctx.end_frame();
    }

    #[test]
    fn preset_selector_renders() {
        let ctx = egui::Context::default();
        ctx.begin_frame(egui::RawInput::default());
        egui::CentralPanel::default().show(&ctx, |ui| {
            let mut selected = None;
            let mut background = psu_packer::IconSysConfig::default_background_colors();
            let mut lights = psu_packer::IconSysConfig::default_light_colors();
            let mut ambient = psu_packer::IconSysConfig::default_ambient_color();
            let response = preset_selector(
                ui,
                PresetSectionState {
                    selected_preset: &mut selected,
                },
                PresetPreviewData {
                    background_colors: &background,
                    light_colors: &lights,
                    ambient_color: &ambient,
                },
            );
            assert!(!response.changed);
        });
        ctx.end_frame();
    }

    #[test]
    fn background_editor_renders() {
        let ctx = egui::Context::default();
        ctx.begin_frame(egui::RawInput::default());
        egui::CentralPanel::default().show(&ctx, |ui| {
            let mut transparency = 0u32;
            let mut colors = psu_packer::IconSysConfig::default_background_colors();
            let response = background_editor(
                ui,
                BackgroundSectionState {
                    transparency: &mut transparency,
                    colors: &mut colors,
                },
            );
            assert!(!response.changed);
        });
        ctx.end_frame();
    }

    #[test]
    fn lighting_editor_renders() {
        let ctx = egui::Context::default();
        ctx.begin_frame(egui::RawInput::default());
        egui::CentralPanel::default().show(&ctx, |ui| {
            let mut light_colors = psu_packer::IconSysConfig::default_light_colors();
            let mut light_directions = psu_packer::IconSysConfig::default_light_directions();
            let mut ambient = psu_packer::IconSysConfig::default_ambient_color();
            let response = lighting_editor(
                ui,
                LightingSectionState {
                    light_colors: &mut light_colors,
                    light_directions: &mut light_directions,
                    ambient_color: &mut ambient,
                },
            );
            assert!(!response.changed);
        });
        ctx.end_frame();
    }
}
