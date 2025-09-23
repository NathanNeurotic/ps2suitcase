use ps2_filetypes::color::Color;
use ps2_filetypes::{self, sjis, ColorF, IconSys, Vector};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IconSysConfig {
    pub flags: IconSysFlags,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linebreak_pos: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_transparency: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_colors: Option<Vec<ColorConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light_directions: Option<Vec<VectorConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light_colors: Option<Vec<ColorFConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambient_color: Option<ColorFConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedIconSysConfig {
    pub background_transparency: u32,
    pub background_colors: [ColorConfig; 4],
    pub light_directions: [VectorConfig; 3],
    pub light_colors: [ColorFConfig; 3],
    pub ambient_color: ColorFConfig,
}

impl IconSysConfig {
    pub fn to_bytes(&self) -> Result<Vec<u8>, crate::Error> {
        let icon_sys = self.build_icon_sys()?;
        icon_sys
            .to_bytes()
            .map_err(|err| crate::Error::ConfigError(err.to_string()))
    }

    pub fn build_icon_sys(&self) -> Result<IconSys, crate::Error> {
        let mut background_colors = DEFAULT_BACKGROUND_COLORS;
        if let Some(colors) = &self.background_colors {
            if colors.len() != background_colors.len() {
                return Err(crate::Error::ConfigError(format!(
                    "icon_sys.background_colors must contain exactly {} entries",
                    background_colors.len()
                )));
            }

            for (target, value) in background_colors.iter_mut().zip(colors.iter()) {
                *target = (*value).into();
            }
        }

        let mut light_directions = DEFAULT_LIGHT_DIRECTIONS;
        if let Some(directions) = &self.light_directions {
            if directions.len() != light_directions.len() {
                return Err(crate::Error::ConfigError(format!(
                    "icon_sys.light_directions must contain exactly {} entries",
                    light_directions.len()
                )));
            }

            for (target, value) in light_directions.iter_mut().zip(directions.iter()) {
                *target = (*value).into();
            }
        }

        let mut light_colors = DEFAULT_LIGHT_COLORS;
        if let Some(colors) = &self.light_colors {
            if colors.len() != light_colors.len() {
                return Err(crate::Error::ConfigError(format!(
                    "icon_sys.light_colors must contain exactly {} entries",
                    light_colors.len()
                )));
            }

            for (target, value) in light_colors.iter_mut().zip(colors.iter()) {
                *target = (*value).into();
            }
        }

        let ambient_color = self
            .ambient_color
            .map(|color| color.into())
            .unwrap_or(DEFAULT_AMBIENT_COLOR);

        let background_transparency = self
            .background_transparency
            .unwrap_or(DEFAULT_BACKGROUND_TRANSPARENCY);

        let linebreak_pos = self.linebreak_pos.unwrap_or(DEFAULT_LINEBREAK_POS);

        Ok(IconSys {
            flags: self.flags.value(),
            linebreak_pos,
            background_transparency,
            background_colors,
            light_directions,
            light_colors,
            ambient_color,
            title: self.title.clone(),
            icon_file: ICON_FILE_NAME.to_string(),
            icon_copy_file: ICON_FILE_NAME.to_string(),
            icon_delete_file: ICON_FILE_NAME.to_string(),
        })
    }
}

impl IconSysConfig {
    pub const fn default_linebreak_pos() -> u16 {
        DEFAULT_LINEBREAK_POS
    }

    pub const fn default_background_transparency() -> u32 {
        DEFAULT_BACKGROUND_TRANSPARENCY
    }

    pub const fn default_background_colors() -> [ColorConfig; 4] {
        [
            ColorConfig {
                r: DEFAULT_BACKGROUND_COLORS[0].r,
                g: DEFAULT_BACKGROUND_COLORS[0].g,
                b: DEFAULT_BACKGROUND_COLORS[0].b,
                a: DEFAULT_BACKGROUND_COLORS[0].a,
            },
            ColorConfig {
                r: DEFAULT_BACKGROUND_COLORS[1].r,
                g: DEFAULT_BACKGROUND_COLORS[1].g,
                b: DEFAULT_BACKGROUND_COLORS[1].b,
                a: DEFAULT_BACKGROUND_COLORS[1].a,
            },
            ColorConfig {
                r: DEFAULT_BACKGROUND_COLORS[2].r,
                g: DEFAULT_BACKGROUND_COLORS[2].g,
                b: DEFAULT_BACKGROUND_COLORS[2].b,
                a: DEFAULT_BACKGROUND_COLORS[2].a,
            },
            ColorConfig {
                r: DEFAULT_BACKGROUND_COLORS[3].r,
                g: DEFAULT_BACKGROUND_COLORS[3].g,
                b: DEFAULT_BACKGROUND_COLORS[3].b,
                a: DEFAULT_BACKGROUND_COLORS[3].a,
            },
        ]
    }

    pub const fn default_light_directions() -> [VectorConfig; 3] {
        [
            VectorConfig {
                x: DEFAULT_LIGHT_DIRECTIONS[0].x,
                y: DEFAULT_LIGHT_DIRECTIONS[0].y,
                z: DEFAULT_LIGHT_DIRECTIONS[0].z,
                w: DEFAULT_LIGHT_DIRECTIONS[0].w,
            },
            VectorConfig {
                x: DEFAULT_LIGHT_DIRECTIONS[1].x,
                y: DEFAULT_LIGHT_DIRECTIONS[1].y,
                z: DEFAULT_LIGHT_DIRECTIONS[1].z,
                w: DEFAULT_LIGHT_DIRECTIONS[1].w,
            },
            VectorConfig {
                x: DEFAULT_LIGHT_DIRECTIONS[2].x,
                y: DEFAULT_LIGHT_DIRECTIONS[2].y,
                z: DEFAULT_LIGHT_DIRECTIONS[2].z,
                w: DEFAULT_LIGHT_DIRECTIONS[2].w,
            },
        ]
    }

    pub const fn default_light_colors() -> [ColorFConfig; 3] {
        [
            ColorFConfig {
                r: DEFAULT_LIGHT_COLORS[0].r,
                g: DEFAULT_LIGHT_COLORS[0].g,
                b: DEFAULT_LIGHT_COLORS[0].b,
                a: DEFAULT_LIGHT_COLORS[0].a,
            },
            ColorFConfig {
                r: DEFAULT_LIGHT_COLORS[1].r,
                g: DEFAULT_LIGHT_COLORS[1].g,
                b: DEFAULT_LIGHT_COLORS[1].b,
                a: DEFAULT_LIGHT_COLORS[1].a,
            },
            ColorFConfig {
                r: DEFAULT_LIGHT_COLORS[2].r,
                g: DEFAULT_LIGHT_COLORS[2].g,
                b: DEFAULT_LIGHT_COLORS[2].b,
                a: DEFAULT_LIGHT_COLORS[2].a,
            },
        ]
    }

    pub const fn default_ambient_color() -> ColorFConfig {
        ColorFConfig {
            r: DEFAULT_AMBIENT_COLOR.r,
            g: DEFAULT_AMBIENT_COLOR.g,
            b: DEFAULT_AMBIENT_COLOR.b,
            a: DEFAULT_AMBIENT_COLOR.a,
        }
    }

    pub fn background_transparency_value(&self) -> u32 {
        self.background_transparency
            .unwrap_or(Self::default_background_transparency())
    }

    pub fn background_colors_array(&self) -> [ColorConfig; 4] {
        let mut colors = Self::default_background_colors();
        if let Some(values) = &self.background_colors {
            for (target, value) in colors.iter_mut().zip(values.iter()) {
                *target = *value;
            }
        }
        colors
    }

    pub fn light_directions_array(&self) -> [VectorConfig; 3] {
        let mut directions = Self::default_light_directions();
        if let Some(values) = &self.light_directions {
            for (target, value) in directions.iter_mut().zip(values.iter()) {
                *target = *value;
            }
        }
        directions
    }

    pub fn light_colors_array(&self) -> [ColorFConfig; 3] {
        let mut colors = Self::default_light_colors();
        if let Some(values) = &self.light_colors {
            for (target, value) in colors.iter_mut().zip(values.iter()) {
                *target = *value;
            }
        }
        colors
    }

    pub fn ambient_color_value(&self) -> ColorFConfig {
        self.ambient_color
            .unwrap_or_else(Self::default_ambient_color)
    }

    pub fn linebreak_position(&self) -> u16 {
        self.linebreak_pos.unwrap_or(Self::default_linebreak_pos())
    }

    pub fn resolved_with_fallback(
        &self,
        icon_sys_fallback: Option<&IconSys>,
    ) -> ResolvedIconSysConfig {
        let background_transparency = self
            .background_transparency
            .or_else(|| icon_sys_fallback.map(|icon_sys| icon_sys.background_transparency))
            .unwrap_or_else(Self::default_background_transparency);

        let background_colors = if self.background_colors.is_some() {
            self.background_colors_array()
        } else if let Some(icon_sys) = icon_sys_fallback {
            icon_sys.background_colors.map(Into::into)
        } else {
            Self::default_background_colors()
        };

        let light_directions = if self.light_directions.is_some() {
            self.light_directions_array()
        } else if let Some(icon_sys) = icon_sys_fallback {
            icon_sys.light_directions.map(Into::into)
        } else {
            Self::default_light_directions()
        };

        let light_colors = if self.light_colors.is_some() {
            self.light_colors_array()
        } else if let Some(icon_sys) = icon_sys_fallback {
            icon_sys.light_colors.map(Into::into)
        } else {
            Self::default_light_colors()
        };

        let ambient_color = if let Some(color) = self.ambient_color {
            color
        } else if let Some(icon_sys) = icon_sys_fallback {
            icon_sys.ambient_color.into()
        } else {
            Self::default_ambient_color()
        };

        ResolvedIconSysConfig {
            background_transparency,
            background_colors,
            light_directions,
            light_colors,
            ambient_color,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub struct ColorConfig {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<ColorConfig> for Color {
    fn from(value: ColorConfig) -> Self {
        Color {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}

impl From<Color> for ColorConfig {
    fn from(value: Color) -> Self {
        ColorConfig {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq)]
pub struct ColorFConfig {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl From<ColorFConfig> for ColorF {
    fn from(value: ColorFConfig) -> Self {
        ColorF {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}

impl From<ColorF> for ColorFConfig {
    fn from(value: ColorF) -> Self {
        ColorFConfig {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq)]
pub struct VectorConfig {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl From<VectorConfig> for Vector {
    fn from(value: VectorConfig) -> Self {
        Vector {
            x: value.x,
            y: value.y,
            z: value.z,
            w: value.w,
        }
    }
}

impl From<Vector> for VectorConfig {
    fn from(value: Vector) -> Self {
        VectorConfig {
            x: value.x,
            y: value.y,
            z: value.z,
            w: value.w,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconSysFlags(u16);

impl IconSysFlags {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}

impl From<u16> for IconSysFlags {
    fn from(value: u16) -> Self {
        Self::new(value)
    }
}

impl From<IconSysFlags> for u16 {
    fn from(value: IconSysFlags) -> Self {
        value.value()
    }
}

impl<'de> Deserialize<'de> for IconSysFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IconSysFlagsVisitor;

        impl<'de> serde::de::Visitor<'de> for IconSysFlagsVisitor {
            type Value = IconSysFlags;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an icon.sys flag value or descriptive name")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if value > u16::MAX as u64 {
                    return Err(E::custom("icon_sys.flags must be between 0 and 65535"));
                }
                Ok(IconSysFlags::new(value as u16))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if !(0..=u16::MAX as i64).contains(&value) {
                    return Err(E::custom("icon_sys.flags must be between 0 and 65535"));
                }
                Ok(IconSysFlags::new(value as u16))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                parse_flag_string(value)
                    .map(IconSysFlags::new)
                    .map_err(E::custom)
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(&value)
            }
        }

        deserializer.deserialize_any(IconSysFlagsVisitor)
    }
}

impl Serialize for IconSysFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u16(self.value())
    }
}

#[derive(Clone, Copy)]
pub struct IconSysPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub background_transparency: u32,
    pub background_colors: [ColorConfig; 4],
    pub light_directions: [VectorConfig; 3],
    pub light_colors: [ColorFConfig; 3],
    pub ambient_color: ColorFConfig,
}

pub const ICON_SYS_PRESETS: &[IconSysPreset] = &[
    IconSysPreset {
        id: "default",
        label: "Standard (PS2)",
        background_transparency: IconSysConfig::default_background_transparency(),
        background_colors: IconSysConfig::default_background_colors(),
        light_directions: IconSysConfig::default_light_directions(),
        light_colors: IconSysConfig::default_light_colors(),
        ambient_color: IconSysConfig::default_ambient_color(),
    },
    IconSysPreset {
        id: "cool_blue",
        label: "Cool Blue",
        background_transparency: 0,
        background_colors: [
            ColorConfig {
                r: 0,
                g: 32,
                b: 96,
                a: 0,
            },
            ColorConfig {
                r: 0,
                g: 48,
                b: 128,
                a: 0,
            },
            ColorConfig {
                r: 0,
                g: 64,
                b: 160,
                a: 0,
            },
            ColorConfig {
                r: 0,
                g: 16,
                b: 48,
                a: 0,
            },
        ],
        light_directions: [
            VectorConfig {
                x: 0.0,
                y: 0.0,
                z: 1.0,
                w: 0.0,
            },
            VectorConfig {
                x: -0.5,
                y: -0.5,
                z: 0.5,
                w: 0.0,
            },
            VectorConfig {
                x: 0.5,
                y: -0.5,
                z: 0.5,
                w: 0.0,
            },
        ],
        light_colors: [
            ColorFConfig {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
            ColorFConfig {
                r: 0.5,
                g: 0.5,
                b: 0.6,
                a: 1.0,
            },
            ColorFConfig {
                r: 0.3,
                g: 0.3,
                b: 0.4,
                a: 1.0,
            },
        ],
        ambient_color: ColorFConfig {
            r: 0.2,
            g: 0.2,
            b: 0.2,
            a: 1.0,
        },
    },
    IconSysPreset {
        id: "warm_sunset",
        label: "Warm Sunset",
        background_transparency: 0,
        background_colors: [
            ColorConfig {
                r: 128,
                g: 48,
                b: 16,
                a: 0,
            },
            ColorConfig {
                r: 176,
                g: 72,
                b: 32,
                a: 0,
            },
            ColorConfig {
                r: 208,
                g: 112,
                b: 48,
                a: 0,
            },
            ColorConfig {
                r: 96,
                g: 32,
                b: 16,
                a: 0,
            },
        ],
        light_directions: [
            VectorConfig {
                x: -0.2,
                y: -0.4,
                z: 0.8,
                w: 0.0,
            },
            VectorConfig {
                x: 0.0,
                y: -0.6,
                z: 0.6,
                w: 0.0,
            },
            VectorConfig {
                x: 0.3,
                y: -0.5,
                z: 0.7,
                w: 0.0,
            },
        ],
        light_colors: [
            ColorFConfig {
                r: 1.0,
                g: 0.9,
                b: 0.75,
                a: 1.0,
            },
            ColorFConfig {
                r: 0.9,
                g: 0.6,
                b: 0.3,
                a: 1.0,
            },
            ColorFConfig {
                r: 0.6,
                g: 0.3,
                b: 0.2,
                a: 1.0,
            },
        ],
        ambient_color: ColorFConfig {
            r: 0.25,
            g: 0.18,
            b: 0.12,
            a: 1.0,
        },
    },
];

pub const ICON_SYS_FLAG_OPTIONS: &[(u16, &str)] =
    &[(0, "Save Data"), (1, "System Software"), (4, "Settings")];

pub const ICON_SYS_TITLE_CHAR_LIMIT: usize = 16;

pub fn sanitize_icon_sys_line(value: &str, limit: usize) -> String {
    let mut sanitized = String::new();
    let mut accepted_chars = 0usize;
    for ch in value.chars() {
        if ch.is_control() {
            continue;
        }

        if accepted_chars >= limit {
            break;
        }

        sanitized.push(ch);
        if sjis::is_roundtrip_sjis(&sanitized) {
            accepted_chars += 1;
        } else {
            sanitized.pop();
        }
    }

    sanitized
}

pub fn split_icon_sys_title(title: &str, break_index: usize) -> (String, String) {
    const UNSUPPORTED_CHAR_PLACEHOLDER: char = '\u{FFFD}';

    let sanitized_chars: Vec<char> = title
        .chars()
        .map(|c| {
            if c.is_control() {
                UNSUPPORTED_CHAR_PLACEHOLDER
            } else {
                c
            }
        })
        .collect();

    let mut remaining_bytes = break_index;
    let mut break_in_chars = 0usize;
    if remaining_bytes > 0 {
        for ch in title.chars() {
            let mut utf8 = [0u8; 4];
            let encoded_len = sjis::encode_sjis(ch.encode_utf8(&mut utf8))
                .map(|bytes| bytes.len())
                .unwrap_or(1)
                .max(1);

            if remaining_bytes < encoded_len {
                break;
            }
            remaining_bytes -= encoded_len;
            break_in_chars += 1;
            if remaining_bytes == 0 {
                break;
            }
        }
    }

    let break_index = break_in_chars.min(sanitized_chars.len());
    let line1_count = break_index.min(ICON_SYS_TITLE_CHAR_LIMIT);
    let skip_count = line1_count;

    let line1: String = sanitized_chars.iter().take(line1_count).copied().collect();
    let line2: String = sanitized_chars
        .iter()
        .skip(skip_count)
        .take(ICON_SYS_TITLE_CHAR_LIMIT)
        .copied()
        .collect();

    (line1, line2)
}

pub fn shift_jis_byte_length(value: &str) -> Result<usize, sjis::SjisEncodeError> {
    sjis::encode_sjis(value).map(|bytes| bytes.len())
}

pub fn color_config_to_rgba(color: ColorConfig) -> [u8; 4] {
    [color.r, color.g, color.b, color.a]
}

pub fn rgba_to_color_config(rgba: [u8; 4]) -> ColorConfig {
    ColorConfig {
        r: rgba[0],
        g: rgba[1],
        b: rgba[2],
        a: rgba[3],
    }
}

pub fn color_f_config_to_rgba(color: ColorFConfig) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}

pub fn rgba_to_color_f_config(rgba: [f32; 4]) -> ColorFConfig {
    ColorFConfig {
        r: rgba[0],
        g: rgba[1],
        b: rgba[2],
        a: rgba[3],
    }
}

pub fn color_to_rgba(color: Color) -> [u8; 4] {
    [color.r, color.g, color.b, color.a]
}

pub fn rgba_to_color(rgba: [u8; 4]) -> Color {
    Color::new(rgba[0], rgba[1], rgba[2], rgba[3])
}

pub fn color_to_normalized_rgba(color: Color) -> [f32; 4] {
    let rgba = color_to_rgba(color);
    rgba.map(|component| component as f32 / 255.0)
}

pub fn normalized_rgba_to_color(values: [f32; 4]) -> Color {
    let clamp = |value: f32| -> u8 { (value.clamp(0.0, 1.0) * 255.0).round() as u8 };
    Color::new(
        clamp(values[0]),
        clamp(values[1]),
        clamp(values[2]),
        clamp(values[3]),
    )
}

pub fn color_f_to_rgba(color: ColorF) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}

pub fn rgba_to_color_f(values: [f32; 4]) -> ColorF {
    ColorF {
        r: values[0],
        g: values[1],
        b: values[2],
        a: values[3],
    }
}

fn parse_flag_string(value: &str) -> Result<u16, String> {
    let trimmed = value.trim();
    if let Some(mapped) = parse_named_flag(trimmed) {
        return Ok(mapped);
    }

    if let Some(stripped) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u16::from_str_radix(stripped, 16)
            .map_err(|_| format!("Invalid hexadecimal icon_sys flag: {trimmed}"));
    }

    trimmed
        .parse::<u16>()
        .map_err(|_| format!("Invalid icon_sys flag value: {trimmed}"))
}

fn parse_named_flag(value: &str) -> Option<u16> {
    let normalized: String = value
        .to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && *c != '_' && *c != '(' && *c != ')')
        .collect();

    match normalized.as_str() {
        "ps2savefile" | "savefile" => Some(0),
        "softwareps2" | "software" => Some(1),
        "unrecognizeddata" | "unrecognized" | "data" => Some(2),
        "softwarepocketstation" | "pocketstation" => Some(3),
        "settingsps2" | "settings" => Some(4),
        "systemdriver" | "driver" => Some(5),
        _ => None,
    }
}

const DEFAULT_LINEBREAK_POS: u16 = 0;
const DEFAULT_BACKGROUND_TRANSPARENCY: u32 = 0;
const DEFAULT_BACKGROUND_COLORS: [Color; 4] = [
    Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    },
    Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    },
    Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    },
    Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    },
];
const DEFAULT_LIGHT_DIRECTIONS: [Vector; 3] = [
    Vector {
        x: 0.0,
        y: 0.0,
        z: 1.0,
        w: 0.0,
    },
    Vector {
        x: 0.0,
        y: 0.0,
        z: 1.0,
        w: 0.0,
    },
    Vector {
        x: 0.0,
        y: 0.0,
        z: 1.0,
        w: 0.0,
    },
];
const DEFAULT_LIGHT_COLORS: [ColorF; 3] = [
    ColorF {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    },
    ColorF {
        r: 0.5,
        g: 0.5,
        b: 0.5,
        a: 1.0,
    },
    ColorF {
        r: 0.3,
        g: 0.3,
        b: 0.3,
        a: 1.0,
    },
];
const DEFAULT_AMBIENT_COLOR: ColorF = ColorF {
    r: 0.2,
    g: 0.2,
    b: 0.2,
    a: 1.0,
};
const ICON_FILE_NAME: &str = "icon.icn";

#[cfg(test)]
mod tests {
    use super::*;
    use ps2_filetypes::{color::Color, ColorF, Vector};

    #[test]
    fn sanitize_icon_sys_line_filters_control_chars_and_roundtrips() {
        let sanitized = sanitize_icon_sys_line("AB\u{0007}Cあいうえお", 8);
        assert_eq!(sanitized, "ABCあいうえお");
    }

    #[test]
    fn split_icon_sys_title_handles_multibyte_breaks() {
        let title = "セーブデータこんにちは";
        let break_bytes = shift_jis_byte_length("セーブデータ").unwrap();
        let (line1, line2) = split_icon_sys_title(title, break_bytes);
        assert_eq!(line1, "セーブデータ");
        assert_eq!(line2, "こんにちは");
    }

    #[test]
    fn resolved_with_fallback_uses_defaults_without_icon_sys() {
        let config = IconSysConfig {
            flags: IconSysFlags::new(0),
            title: "Test".to_string(),
            linebreak_pos: None,
            preset: None,
            background_transparency: None,
            background_colors: None,
            light_directions: None,
            light_colors: None,
            ambient_color: None,
        };

        let resolved = config.resolved_with_fallback(None);

        assert_eq!(
            resolved.background_transparency,
            IconSysConfig::default_background_transparency()
        );
        assert_eq!(
            resolved.background_colors,
            IconSysConfig::default_background_colors()
        );
        assert_eq!(
            resolved.light_directions,
            IconSysConfig::default_light_directions()
        );
        assert_eq!(resolved.light_colors, IconSysConfig::default_light_colors());
        assert_eq!(
            resolved.ambient_color,
            IconSysConfig::default_ambient_color()
        );
    }

    #[test]
    fn resolved_with_fallback_prefers_config_values() {
        let config = IconSysConfig {
            flags: IconSysFlags::new(0),
            title: "Test".to_string(),
            linebreak_pos: None,
            preset: None,
            background_transparency: Some(7),
            background_colors: Some(vec![
                ColorConfig {
                    r: 1,
                    g: 2,
                    b: 3,
                    a: 4,
                },
                ColorConfig {
                    r: 5,
                    g: 6,
                    b: 7,
                    a: 8,
                },
                ColorConfig {
                    r: 9,
                    g: 10,
                    b: 11,
                    a: 12,
                },
                ColorConfig {
                    r: 13,
                    g: 14,
                    b: 15,
                    a: 16,
                },
            ]),
            light_directions: Some(vec![
                VectorConfig {
                    x: 0.1,
                    y: 0.2,
                    z: 0.3,
                    w: 0.4,
                },
                VectorConfig {
                    x: 0.5,
                    y: 0.6,
                    z: 0.7,
                    w: 0.8,
                },
                VectorConfig {
                    x: 0.9,
                    y: 1.0,
                    z: 1.1,
                    w: 1.2,
                },
            ]),
            light_colors: Some(vec![
                ColorFConfig {
                    r: 1.0,
                    g: 1.1,
                    b: 1.2,
                    a: 1.3,
                },
                ColorFConfig {
                    r: 1.4,
                    g: 1.5,
                    b: 1.6,
                    a: 1.7,
                },
                ColorFConfig {
                    r: 1.8,
                    g: 1.9,
                    b: 2.0,
                    a: 2.1,
                },
            ]),
            ambient_color: Some(ColorFConfig {
                r: 0.5,
                g: 0.6,
                b: 0.7,
                a: 0.8,
            }),
        };

        let fallback = IconSys {
            flags: 0,
            linebreak_pos: 0,
            background_transparency: 13,
            background_colors: [
                Color::new(2, 3, 4, 5),
                Color::new(6, 7, 8, 9),
                Color::new(10, 11, 12, 13),
                Color::new(14, 15, 16, 17),
            ],
            light_directions: [
                Vector {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    w: 4.0,
                },
                Vector {
                    x: 5.0,
                    y: 6.0,
                    z: 7.0,
                    w: 8.0,
                },
                Vector {
                    x: 9.0,
                    y: 10.0,
                    z: 11.0,
                    w: 12.0,
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
                r: 0.2,
                g: 0.3,
                b: 0.4,
                a: 0.5,
            },
            title: "Fallback".to_string(),
            icon_file: "icon.icn".to_string(),
            icon_copy_file: "icon.icn".to_string(),
            icon_delete_file: "icon.icn".to_string(),
        };

        let resolved = config.resolved_with_fallback(Some(&fallback));

        assert_eq!(resolved.background_transparency, 7);
        assert_eq!(resolved.background_colors, config.background_colors_array());
        assert_eq!(resolved.light_directions, config.light_directions_array());
        assert_eq!(resolved.light_colors, config.light_colors_array());
        assert_eq!(resolved.ambient_color, config.ambient_color.unwrap());
    }

    #[test]
    fn resolved_with_fallback_uses_icon_sys_when_config_missing_values() {
        let config = IconSysConfig {
            flags: IconSysFlags::new(0),
            title: "Test".to_string(),
            linebreak_pos: None,
            preset: None,
            background_transparency: None,
            background_colors: None,
            light_directions: None,
            light_colors: None,
            ambient_color: None,
        };

        let fallback = IconSys {
            flags: 0,
            linebreak_pos: 0,
            background_transparency: 99,
            background_colors: [
                Color::new(1, 1, 1, 1),
                Color::new(2, 2, 2, 2),
                Color::new(3, 3, 3, 3),
                Color::new(4, 4, 4, 4),
            ],
            light_directions: [
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                Vector {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                    w: 0.0,
                },
                Vector {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                    w: 0.0,
                },
            ],
            light_colors: [
                ColorF {
                    r: 0.2,
                    g: 0.3,
                    b: 0.4,
                    a: 0.5,
                },
                ColorF {
                    r: 0.6,
                    g: 0.7,
                    b: 0.8,
                    a: 0.9,
                },
                ColorF {
                    r: 1.0,
                    g: 1.1,
                    b: 1.2,
                    a: 1.3,
                },
            ],
            ambient_color: ColorF {
                r: 0.4,
                g: 0.5,
                b: 0.6,
                a: 0.7,
            },
            title: "Fallback".to_string(),
            icon_file: "icon.icn".to_string(),
            icon_copy_file: "icon.icn".to_string(),
            icon_delete_file: "icon.icn".to_string(),
        };

        let resolved = config.resolved_with_fallback(Some(&fallback));

        assert_eq!(resolved.background_transparency, 99);
        assert_eq!(
            resolved.background_colors,
            fallback.background_colors.map(Into::into)
        );
        assert_eq!(
            resolved.light_directions,
            fallback.light_directions.map(Into::into)
        );
        assert_eq!(resolved.light_colors, fallback.light_colors.map(Into::into));
        assert_eq!(resolved.ambient_color, fallback.ambient_color.into());
    }
}
