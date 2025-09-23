use crate::IconFlagSelection;
use ps2_filetypes::IconSys;
use psu_packer::{
    ColorConfig, ColorFConfig, IconSysConfig, IconSysPreset, VectorConfig, ICON_SYS_FLAG_OPTIONS,
    ICON_SYS_PRESETS,
};

#[derive(Clone, Debug, PartialEq)]
pub struct IconSysState {
    pub flag_selection: IconFlagSelection,
    pub custom_flag: u16,
    pub background_transparency: u32,
    pub background_colors: [ColorConfig; 4],
    pub light_directions: [VectorConfig; 3],
    pub light_colors: [ColorFConfig; 3],
    pub ambient_color: ColorFConfig,
    pub selected_preset: Option<String>,
}

impl Default for IconSysState {
    fn default() -> Self {
        Self {
            flag_selection: IconFlagSelection::Preset(0),
            custom_flag: ICON_SYS_FLAG_OPTIONS[0].0,
            background_transparency: IconSysConfig::default_background_transparency(),
            background_colors: IconSysConfig::default_background_colors(),
            light_directions: IconSysConfig::default_light_directions(),
            light_colors: IconSysConfig::default_light_colors(),
            ambient_color: IconSysConfig::default_ambient_color(),
            selected_preset: None,
        }
    }
}

impl IconSysState {
    pub fn from_icon_sys(icon_sys: &IconSys) -> Self {
        let mut state = Self::default();
        state.apply_icon_sys(icon_sys);
        state
    }

    pub fn set_flag_value(&mut self, flag_value: u16) {
        self.custom_flag = flag_value;
        if let Some(index) = ICON_SYS_FLAG_OPTIONS
            .iter()
            .position(|(value, _)| *value == flag_value)
        {
            self.flag_selection = IconFlagSelection::Preset(index);
        } else {
            self.flag_selection = IconFlagSelection::Custom;
        }
    }

    pub fn apply_preset(&mut self, preset: &IconSysPreset) {
        self.background_transparency = preset.background_transparency;
        self.background_colors = preset.background_colors;
        self.light_directions = preset.light_directions;
        self.light_colors = preset.light_colors;
        self.ambient_color = preset.ambient_color;
        self.selected_preset = Some(preset.id.to_string());
    }

    pub fn clear_preset(&mut self) {
        self.selected_preset = None;
    }

    pub fn detect_preset(&self) -> Option<String> {
        ICON_SYS_PRESETS.iter().find_map(|preset| {
            if preset.background_transparency == self.background_transparency
                && preset.background_colors == self.background_colors
                && preset.light_directions == self.light_directions
                && preset.light_colors == self.light_colors
                && preset.ambient_color == self.ambient_color
            {
                Some(preset.id.to_string())
            } else {
                None
            }
        })
    }

    pub fn update_detected_preset(&mut self) {
        self.selected_preset = self.detect_preset();
    }

    pub fn apply_icon_sys_config(
        &mut self,
        icon_cfg: &IconSysConfig,
        icon_sys_fallback: Option<&IconSys>,
    ) {
        self.set_flag_value(icon_cfg.flags.value());

        let resolved = icon_cfg.resolved_with_fallback(icon_sys_fallback);

        self.background_transparency = resolved.background_transparency;
        self.background_colors = resolved.background_colors;
        self.light_directions = resolved.light_directions;
        self.light_colors = resolved.light_colors;
        self.ambient_color = resolved.ambient_color;

        self.selected_preset = icon_cfg.preset.clone();
    }

    pub fn apply_icon_sys(&mut self, icon_sys: &IconSys) {
        self.set_flag_value(icon_sys.flags);
        self.background_transparency = icon_sys.background_transparency;
        self.background_colors = background_colors_from_icon_sys(icon_sys);
        self.light_directions = light_directions_from_icon_sys(icon_sys);
        self.light_colors = light_colors_from_icon_sys(icon_sys);
        self.ambient_color = ambient_color_from_icon_sys(icon_sys);
        self.clear_preset();
    }
}

fn background_colors_from_icon_sys(icon_sys: &IconSys) -> [ColorConfig; 4] {
    let mut colors = IconSysConfig::default_background_colors();
    for (target, color) in colors.iter_mut().zip(icon_sys.background_colors.iter()) {
        *target = ColorConfig {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        };
    }
    colors
}

fn light_directions_from_icon_sys(icon_sys: &IconSys) -> [VectorConfig; 3] {
    let mut directions = IconSysConfig::default_light_directions();
    for (target, direction) in directions.iter_mut().zip(icon_sys.light_directions.iter()) {
        *target = VectorConfig {
            x: direction.x,
            y: direction.y,
            z: direction.z,
            w: direction.w,
        };
    }
    directions
}

fn light_colors_from_icon_sys(icon_sys: &IconSys) -> [ColorFConfig; 3] {
    let mut colors = IconSysConfig::default_light_colors();
    for (target, color) in colors.iter_mut().zip(icon_sys.light_colors.iter()) {
        *target = ColorFConfig {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        };
    }
    colors
}

fn ambient_color_from_icon_sys(icon_sys: &IconSys) -> ColorFConfig {
    ColorFConfig {
        r: icon_sys.ambient_color.r,
        g: icon_sys.ambient_color.g,
        b: icon_sys.ambient_color.b,
        a: icon_sys.ambient_color.a,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps2_filetypes::{color::Color, ColorF, Vector};

    #[test]
    fn detect_preset_matches_known_configuration() {
        let preset = &ICON_SYS_PRESETS[0];
        let mut state = IconSysState::default();
        state.apply_preset(preset);
        assert_eq!(state.detect_preset(), Some(preset.id.to_string()));
    }

    #[test]
    fn apply_icon_sys_populates_fields() {
        let icon_sys = IconSys {
            flags: 123,
            linebreak_pos: 0,
            background_transparency: 42,
            background_colors: [
                Color::new(1, 2, 3, 4),
                Color::new(5, 6, 7, 8),
                Color::new(9, 10, 11, 12),
                Color::new(13, 14, 15, 16),
            ],
            light_directions: [
                Vector {
                    x: 0.1,
                    y: 0.2,
                    z: 0.3,
                    w: 0.4,
                },
                Vector {
                    x: 0.5,
                    y: 0.6,
                    z: 0.7,
                    w: 0.8,
                },
                Vector {
                    x: 0.9,
                    y: 1.0,
                    z: 1.1,
                    w: 1.2,
                },
            ],
            light_colors: [
                ColorF {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 0.4,
                },
                ColorF {
                    r: 0.5,
                    g: 0.6,
                    b: 0.7,
                    a: 0.8,
                },
                ColorF {
                    r: 0.9,
                    g: 1.0,
                    b: 1.1,
                    a: 1.2,
                },
            ],
            ambient_color: ColorF {
                r: 0.3,
                g: 0.4,
                b: 0.5,
                a: 0.6,
            },
            title: String::new(),
            icon_file: String::new(),
            icon_copy_file: String::new(),
            icon_delete_file: String::new(),
        };

        let mut state = IconSysState::default();
        state.apply_icon_sys(&icon_sys);

        assert_eq!(state.custom_flag, 123);
        assert_eq!(state.background_transparency, 42);
        assert_eq!(state.background_colors[0].r, 1);
        assert_eq!(state.light_directions[1].z, 0.7);
        assert!((state.light_colors[2].b - 1.1).abs() < f32::EPSILON);
        assert!((state.ambient_color.g - 0.4).abs() < f32::EPSILON);
        assert_eq!(state.selected_preset, None);
    }
}
