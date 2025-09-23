use crate::tabs::Tab;
use crate::{AppState, VirtualFile};
use eframe::egui;
use eframe::egui::{CornerRadius, Grid, Id, PopupCloseBehavior, Response, Ui};
use icon_sys_ui::{
    background_editor, flag_selector, lighting_editor, preset_selector, title_editor,
    BackgroundSectionState, FlagSectionState, IconFlagSelection, LightingSectionState,
    PresetPreviewData, PresetSectionState, PresetSelection, TitleSectionIds, TitleSectionState,
};
use ps2_filetypes::color::Color;
use ps2_filetypes::{ColorF, IconSys, Vector};
use psu_packer::{
    shift_jis_byte_length, split_icon_sys_title, ColorConfig, ColorFConfig, IconSysPreset,
    VectorConfig, ICON_SYS_FLAG_OPTIONS, ICON_SYS_PRESETS,
};
use relative_path::PathExt;
use std::path::PathBuf;

pub struct IconSysViewer {
    title_line1: String,
    title_line2: String,
    file: String,
    pub icon_file: String,
    pub icon_copy_file: String,
    pub icon_delete_file: String,
    pub flag_selection: IconFlagSelection,
    pub custom_flag: u16,
    pub background_transparency: u32,
    pub ambient_color: ColorFConfig,
    pub background_colors: [ColorConfig; 4],
    pub light_directions: [VectorConfig; 3],
    pub light_colors: [ColorFConfig; 3],
    pub selected_preset: Option<String>,
    pub sys: IconSys,
    pub file_path: PathBuf,
}

impl IconSysViewer {
    pub fn new(file: &VirtualFile, state: &AppState) -> Self {
        let buf = std::fs::read(&file.file_path).expect("File not found");

        let sys = IconSys::new(buf);
        let (title_line1, title_line2) =
            split_icon_sys_title(&sys.title, sys.linebreak_pos as usize);

        let (flag_selection, custom_flag) = resolve_flag_selection(sys.flags);

        let background_colors = sys.background_colors.map(|color| ColorConfig {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        });
        let light_directions = sys.light_directions.map(|direction| VectorConfig {
            x: direction.x,
            y: direction.y,
            z: direction.z,
            w: direction.w,
        });
        let light_colors = sys.light_colors.map(|color| ColorFConfig {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        });
        let ambient_color = ColorFConfig {
            r: sys.ambient_color.r,
            g: sys.ambient_color.g,
            b: sys.ambient_color.b,
            a: sys.ambient_color.a,
        };

        let selected_preset = detect_preset(
            sys.background_transparency,
            &background_colors,
            &light_directions,
            &light_colors,
            &ambient_color,
        );

        Self {
            title_line1,
            title_line2,
            icon_file: sys.icon_file.clone(),
            icon_copy_file: sys.icon_copy_file.clone(),
            icon_delete_file: sys.icon_delete_file.clone(),
            background_transparency: sys.background_transparency,
            ambient_color,
            background_colors,
            light_directions,
            light_colors,
            flag_selection,
            custom_flag,
            selected_preset,
            sys,
            file_path: file.file_path.clone(),
            file: file
                .file_path
                .relative_to(state.opened_folder.clone().unwrap())
                .unwrap()
                .to_string(),
        }
    }

    pub fn show(&mut self, ui: &mut Ui, app: &mut AppState) {
        let files: Vec<String> = app
            .files
            .iter()
            .filter_map(|file| {
                let name = file.name.clone();
                if matches!(
                    std::path::Path::new(&name)
                        .extension()
                        .and_then(|ext| ext.to_str()),
                    Some("icn") | Some("ico")
                ) {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();

        ui.vertical(|ui| {
            ui.heading("Icon Configuration");
            ui.add_space(4.0);

            ui.group(|ui| {
                ui.heading("Title");
                ui.small(
                    "Each line supports up to 16 characters that must round-trip through Shift-JIS",
                );
                title_editor(
                    ui,
                    TitleSectionIds {
                        line1: egui::Id::new("viewer_icon_sys_title_line1"),
                        line2: egui::Id::new("viewer_icon_sys_title_line2"),
                    },
                    TitleSectionState {
                        line1: &mut self.title_line1,
                        line2: &mut self.title_line2,
                    },
                );
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.heading("Flags");
                flag_selector(
                    ui,
                    FlagSectionState {
                        selection: &mut self.flag_selection,
                        custom_flag: &mut self.custom_flag,
                    },
                );
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.heading("Presets");
                ui.small("Choose a preset to populate the colors and lights automatically.");
                let response = preset_selector(
                    ui,
                    PresetSectionState {
                        selected_preset: &mut self.selected_preset,
                    },
                    PresetPreviewData {
                        background_colors: &self.background_colors,
                        light_colors: &self.light_colors,
                        ambient_color: &self.ambient_color,
                    },
                );
                if let Some(selection) = response.selection {
                    match selection {
                        PresetSelection::Manual => {
                            self.selected_preset = None;
                        }
                        PresetSelection::Preset(preset) => {
                            apply_preset(self, preset);
                        }
                    }
                }
            });

            ui.add_space(8.0);

            ui.heading("Icons");
            ui.add_space(4.0);

            Grid::new("icons").num_columns(2).show(ui, |ui| {
                ui.label("List");
                file_select(ui, "list_icon", &mut self.icon_file, &files);
                ui.end_row();
                ui.label("Copy");
                file_select(ui, "copy_icon", &mut self.icon_copy_file, &files);
                ui.end_row();
                ui.label("Delete");
                file_select(ui, "delete_icon", &mut self.icon_delete_file, &files);
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.heading("Background");
                ui.small("Adjust the gradient colors and alpha layer.");
                let response = background_editor(
                    ui,
                    BackgroundSectionState {
                        transparency: &mut self.background_transparency,
                        colors: &mut self.background_colors,
                    },
                );
                if response.changed {
                    self.selected_preset = None;
                }
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.heading("Lighting");
                ui.small("Tweak light directions, colors, and the ambient glow.");
                let response = lighting_editor(
                    ui,
                    LightingSectionState {
                        light_colors: &mut self.light_colors,
                        light_directions: &mut self.light_directions,
                        ambient_color: &mut self.ambient_color,
                    },
                );
                if response.changed {
                    self.selected_preset = None;
                }
            });

            ui.add_space(8.0);

            ui.button("Save")
                .on_hover_text("Save changes")
                .clicked()
                .then(|| {
                    self.save();
                });
        });
    }

    fn build_icon_sys(&self) -> IconSys {
        let flag_value =
            icon_sys_ui::selected_icon_flag_value(self.flag_selection, self.custom_flag)
                .unwrap_or(self.sys.flags);
        let linebreak_pos = shift_jis_byte_length(&self.title_line1)
            .map(|len| len as u16)
            .unwrap_or(self.sys.linebreak_pos);
        IconSys {
            flags: flag_value,
            linebreak_pos,
            background_transparency: self.background_transparency,
            background_colors: self.background_colors.map(Into::into),
            light_directions: self.light_directions.map(Into::into),
            light_colors: self.light_colors.map(Into::into),
            ambient_color: self.ambient_color.into(),
            title: format!("{}{}", self.title_line1, self.title_line2),
            icon_file: self.icon_file.clone(),
            icon_copy_file: self.icon_copy_file.clone(),
            icon_delete_file: self.icon_delete_file.clone(),
            ..self.sys.clone()
        }
    }
}

impl Tab for IconSysViewer {
    fn get_id(&self) -> &str {
        &self.file
    }

    fn get_title(&self) -> String {
        self.file.clone()
    }

    fn get_modified(&self) -> bool {
        let rebuilt = self.build_icon_sys();
        if self.sys.flags != rebuilt.flags
            || self.sys.linebreak_pos != rebuilt.linebreak_pos
            || self.sys.background_transparency != rebuilt.background_transparency
            || self.sys.background_colors != rebuilt.background_colors
            || self.sys.light_directions != rebuilt.light_directions
            || self.sys.light_colors != rebuilt.light_colors
            || self.sys.ambient_color != rebuilt.ambient_color
            || self.sys.title != rebuilt.title
            || self.sys.icon_file != rebuilt.icon_file
            || self.sys.icon_copy_file != rebuilt.icon_copy_file
            || self.sys.icon_delete_file != rebuilt.icon_delete_file
        {
            return true;
        }
        false
    }

    fn save(&mut self) {
        let new_sys = self.build_icon_sys();
        std::fs::write(&self.file_path, new_sys.to_bytes().unwrap()).expect("Failed to save icon");
        self.sys = new_sys;
    }
}

fn resolve_flag_selection(flags: u16) -> (IconFlagSelection, u16) {
    if let Some((index, _)) = ICON_SYS_FLAG_OPTIONS
        .iter()
        .enumerate()
        .find(|(_, (value, _))| *value == flags)
    {
        (IconFlagSelection::Preset(index), flags)
    } else {
        (IconFlagSelection::Custom, flags)
    }
}

fn detect_preset(
    transparency: u32,
    background_colors: &[ColorConfig; 4],
    light_directions: &[VectorConfig; 3],
    light_colors: &[ColorFConfig; 3],
    ambient: &ColorFConfig,
) -> Option<String> {
    ICON_SYS_PRESETS.iter().find_map(|preset| {
        if preset.background_transparency == transparency
            && &preset.background_colors == background_colors
            && &preset.light_directions == light_directions
            && &preset.light_colors == light_colors
            && &preset.ambient_color == ambient
        {
            Some(preset.id.to_string())
        } else {
            None
        }
    })
}

fn apply_preset(viewer: &mut IconSysViewer, preset: &IconSysPreset) {
    viewer.background_transparency = preset.background_transparency;
    viewer.background_colors = preset.background_colors;
    viewer.light_directions = preset.light_directions;
    viewer.light_colors = preset.light_colors;
    viewer.ambient_color = preset.ambient_color;
    viewer.selected_preset = Some(preset.id.to_string());
}

fn set_border_radius(ui: &mut Ui, radius: CornerRadius) {
    ui.style_mut().visuals.widgets.hovered.corner_radius = radius.add(CornerRadius::same(1));
    ui.style_mut().visuals.widgets.inactive.corner_radius = radius;
    ui.style_mut().visuals.widgets.active.corner_radius = radius;
}

fn file_select(ui: &mut Ui, name: impl Into<String>, value: &mut String, files: &[String]) {
    let id = Id::from(name.into());
    let layout_response = ui.horizontal(|ui| {
        ui.style_mut().spacing.item_spacing.x = 1.0;

        set_border_radius(
            ui,
            CornerRadius {
                nw: 2,
                sw: 2,
                ne: 0,
                se: 0,
            },
        );
        ui.text_edit_singleline(value);

        set_border_radius(
            ui,
            CornerRadius {
                nw: 0,
                sw: 0,
                ne: 2,
                se: 2,
            },
        );
        let response = ui.button("🔽");
        if response.clicked() {
            ui.memory_mut(|mem| {
                mem.toggle_popup(id);
            });
        }

        response
    });

    let res = Response {
        rect: layout_response.response.rect,
        ..layout_response.inner
    };

    egui::popup_below_widget(ui, id, &res, PopupCloseBehavior::CloseOnClick, |ui| {
        ui.set_min_width(200.0);
        files.iter().for_each(|file| {
            if ui.selectable_label(false, file.clone()).clicked() {
                *value = file.clone();
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tabs::Tab;
    use eframe::egui::{self, CentralPanel};
    use psu_packer::IconSysConfig;
    use tempfile::tempdir;

    #[test]
    fn icon_sys_viewer_preserves_negative_light_directions_when_loading_and_saving() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let icon_sys_path = temp_dir.path().join("icon.sys");

        let background_color = Color::new(10, 20, 30, 255);
        let light_direction = Vector {
            x: -0.5,
            y: 0.75,
            z: -1.0,
            w: 1.0,
        };

        let original_icon_sys = IconSys {
            flags: 0,
            linebreak_pos: 0,
            background_transparency: 0,
            background_colors: [
                background_color,
                background_color,
                background_color,
                background_color,
            ],
            light_directions: [light_direction, light_direction, light_direction],
            light_colors: [
                ColorF {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 1.0,
                },
                ColorF {
                    r: 0.4,
                    g: 0.5,
                    b: 0.6,
                    a: 1.0,
                },
                ColorF {
                    r: 0.7,
                    g: 0.8,
                    b: 0.9,
                    a: 1.0,
                },
            ],
            ambient_color: ColorF {
                r: 0.2,
                g: 0.3,
                b: 0.4,
                a: 1.0,
            },
            title: "Test".into(),
            icon_file: "test.icn".into(),
            icon_copy_file: "copy.icn".into(),
            icon_delete_file: "delete.icn".into(),
        };

        let bytes = original_icon_sys
            .to_bytes()
            .expect("failed to serialize icon.sys");
        std::fs::write(&icon_sys_path, &bytes).expect("failed to write icon.sys");

        let virtual_file = VirtualFile {
            name: "icon.sys".into(),
            file_path: icon_sys_path.clone(),
            size: bytes.len() as u64,
        };

        let mut app_state = AppState::new();
        app_state.opened_folder = Some(temp_dir.path().to_path_buf());

        let mut viewer = IconSysViewer::new(&virtual_file, &app_state);

        assert_eq!(viewer.light_directions[0].x, light_direction.x);
        assert_eq!(viewer.light_directions[0].y, light_direction.y);
        assert_eq!(viewer.light_directions[0].z, light_direction.z);

        viewer.save();

        let reloaded_icon_sys =
            IconSys::new(std::fs::read(icon_sys_path).expect("failed to read icon.sys"));

        assert_eq!(reloaded_icon_sys.light_directions[0].x, light_direction.x);
        assert_eq!(reloaded_icon_sys.light_directions[0].y, light_direction.y);
        assert_eq!(reloaded_icon_sys.light_directions[0].z, light_direction.z);
    }

    #[test]
    fn icon_sys_viewer_renders_default_icon() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let icon_sys_path = temp_dir.path().join("icon.sys");

        let icon_sys = IconSys {
            flags: 0,
            linebreak_pos: IconSysConfig::default_linebreak_pos(),
            background_transparency: IconSysConfig::default_background_transparency(),
            background_colors: IconSysConfig::default_background_colors().map(Into::into),
            light_directions: IconSysConfig::default_light_directions().map(Into::into),
            light_colors: IconSysConfig::default_light_colors().map(Into::into),
            ambient_color: IconSysConfig::default_ambient_color().into(),
            title: "DEFAULT".into(),
            icon_file: "list.icn".into(),
            icon_copy_file: "copy.icn".into(),
            icon_delete_file: "del.icn".into(),
        };

        let bytes = icon_sys.to_bytes().expect("failed to serialize icon.sys");
        std::fs::write(&icon_sys_path, &bytes).expect("failed to write icon.sys");

        let virtual_file = VirtualFile {
            name: "icon.sys".into(),
            file_path: icon_sys_path.clone(),
            size: bytes.len() as u64,
        };

        let mut app_state = AppState::new();
        app_state.opened_folder = Some(temp_dir.path().to_path_buf());

        let mut viewer = IconSysViewer::new(&virtual_file, &app_state);
        let ctx = egui::Context::default();
        ctx.begin_frame(egui::RawInput::default());
        CentralPanel::default().show(&ctx, |ui| {
            viewer.show(ui, &mut app_state);
        });
        let output = ctx.end_frame();

        assert!(output.shapes.iter().any(|shape| !shape.is_empty()));
        assert_eq!(
            viewer.background_transparency,
            icon_sys.background_transparency
        );
        assert_eq!(ColorF::from(viewer.ambient_color), icon_sys.ambient_color);
    }
}
