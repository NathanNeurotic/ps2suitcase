use chrono::{DateTime, Local, NaiveDateTime};
use colored::Colorize;
use ps2_filetypes::{PSUEntry, PSUEntryKind, PSUWriter, DIR_ID, FILE_ID, PSU};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod icon_sys;
pub mod sas;

pub use icon_sys::{
    color_config_to_rgba, color_f_config_to_rgba, color_f_to_rgba, color_to_normalized_rgba,
    color_to_rgba, normalized_rgba_to_color, rgba_to_color, rgba_to_color_config, rgba_to_color_f,
    rgba_to_color_f_config, sanitize_icon_sys_line, shift_jis_byte_length, split_icon_sys_title,
    ColorConfig, ColorFConfig, IconSysConfig, IconSysFlags, IconSysPreset, VectorConfig,
    ICON_SYS_FLAG_OPTIONS, ICON_SYS_PRESETS, ICON_SYS_TITLE_CHAR_LIMIT,
};

#[derive(Debug)]
pub struct Config {
    pub name: String,
    pub timestamp: Option<NaiveDateTime>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub icon_sys: Option<IconSysConfig>,
}

mod date_format {
    use chrono::NaiveDateTime;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub fn serialize<S>(value: &Option<NaiveDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serializer.serialize_some(&value.format(FORMAT).to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserialize: D) -> Result<Option<NaiveDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserialize)?;
        if let Some(s) = s {
            Ok(Some(
                NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)?,
            ))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ConfigFile {
    config: ConfigSection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon_sys: Option<IconSysConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ConfigSection {
    name: String,
    #[serde(default, with = "date_format", skip_serializing_if = "Option::is_none")]
    timestamp: Option<NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude: Option<Vec<String>>,
}

impl From<ConfigFile> for Config {
    fn from(file: ConfigFile) -> Self {
        let ConfigFile { config, icon_sys } = file;
        Self {
            name: config.name,
            timestamp: config.timestamp,
            include: config.include,
            exclude: config.exclude,
            icon_sys,
        }
    }
}

impl Config {
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        let config_section = ConfigSection {
            name: self.name.clone(),
            timestamp: self.timestamp,
            include: self.include.clone(),
            exclude: self.exclude.clone(),
        };

        let config_file = ConfigFile {
            config: config_section,
            icon_sys: self.icon_sys.clone(),
        };

        toml::to_string_pretty(&config_file)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FileTimes {
    pub created: Option<SystemTime>,
    pub modified: SystemTime,
}

pub trait MetadataReader {
    fn file_times(&self, path: &Path) -> std::io::Result<FileTimes>;
}

#[derive(Default)]
pub struct FsMetadataReader;

impl MetadataReader for FsMetadataReader {
    fn file_times(&self, path: &Path) -> std::io::Result<FileTimes> {
        let metadata = std::fs::metadata(path)?;
        let modified = metadata.modified()?;
        let created = metadata.created().ok();
        Ok(FileTimes { created, modified })
    }
}

pub fn load_config(folder: &Path) -> Result<Config, Error> {
    let config_file = folder.join("psu.toml");
    let str = std::fs::read_to_string(&config_file)?;
    let config_file =
        toml::from_str::<ConfigFile>(&str).map_err(|e| Error::ConfigError(e.to_string()))?;
    Ok(config_file.into())
}

pub fn pack_psu(folder: &Path, output: &Path) -> Result<(), Error> {
    let config = load_config(folder)?;
    pack_with_config(folder, output, config)
}

pub fn pack_with_config(folder: &Path, output: &Path, cfg: Config) -> Result<(), Error> {
    let metadata_reader = FsMetadataReader::default();
    pack_with_config_and_metadata_reader(folder, output, cfg, &metadata_reader)
}

pub fn pack_with_config_and_metadata_reader<M: MetadataReader>(
    folder: &Path,
    output: &Path,
    cfg: Config,
    metadata_reader: &M,
) -> Result<(), Error> {
    let Config {
        name,
        timestamp,
        include,
        exclude,
        icon_sys,
    } = cfg;

    if !check_name(&name) {
        return Err(Error::NameError);
    }

    let mut psu = PSU::default();

    let icon_sys_path = folder.join("icon.sys");
    if let Some(icon_config) = &icon_sys {
        let bytes = icon_config.to_bytes()?;
        std::fs::write(&icon_sys_path, bytes)?;
    }

    let raw_included_files = if let Some(include) = include {
        include
            .into_iter()
            .filter_map(|file| {
                if file.contains(|c| matches!(c, '\\' | '/')) {
                    eprintln!(
                        "{} {} {}",
                        "File".dimmed(),
                        file.dimmed(),
                        "exists in subfolder, skipping".dimmed()
                    );
                    None
                } else {
                    let candidate = folder.join(&file);
                    if !candidate.exists() {
                        eprintln!(
                            "{} {} {}",
                            "File".dimmed(),
                            file.dimmed(),
                            "does not exist, skipping".dimmed()
                        );
                        None
                    } else {
                        Some(candidate)
                    }
                }
            })
            .collect::<Vec<_>>()
    } else {
        std::fs::read_dir(folder)?
            .into_iter()
            .flatten()
            .map(|d| d.path())
            .collect::<Vec<_>>()
    };

    let mut files = filter_files(&raw_included_files);
    files.sort_by_key(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_ascii_lowercase())
    });

    if let Some(exclude) = exclude {
        let mut exclude_set = HashSet::new();

        for file in exclude {
            if file.contains(|c| matches!(c, '\\' | '/')) {
                eprintln!(
                    "{} {} {}",
                    "File".dimmed(),
                    file.dimmed(),
                    "exists in subfolder, skipping exclude".dimmed()
                );
                continue;
            }

            let candidate = folder.join(&file);
            if !candidate.exists() {
                eprintln!(
                    "{} {} {}",
                    "File".dimmed(),
                    file.dimmed(),
                    "does not exist, skipping exclude".dimmed()
                );
                continue;
            }

            exclude_set.insert(file);
        }

        if !exclude_set.is_empty() {
            files = files
                .into_iter()
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| !exclude_set.contains(name))
                        .unwrap_or(true)
                })
                .collect::<Vec<_>>();
        }
    }

    if icon_sys.is_some() {
        if !files.iter().any(|path| path == &icon_sys_path) {
            files.push(icon_sys_path);
        }
    }

    let timestamp_value = timestamp.unwrap_or_default();
    add_psu_defaults(&mut psu, &name, files.len(), timestamp_value);
    add_files_to_psu(&mut psu, &files, timestamp, metadata_reader)?;
    std::fs::write(output, PSUWriter::new(psu).to_bytes()?)?;
    Ok(())
}

fn check_name(name: &str) -> bool {
    for c in name.chars() {
        if !matches!(c, 'a'..'z'|'A'..'Z'|'0'..'9'|'_'|'-'|' ') {
            return false;
        }
    }
    true
}

fn filter_files(files: &[PathBuf]) -> Vec<PathBuf> {
    files
        .iter()
        .filter_map(|f| {
            if f.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.eq_ignore_ascii_case("psu.toml"))
                .unwrap_or(false)
            {
                None
            } else if !f.is_file() {
                println!(
                    "{} {}",
                    f.display().to_string().dimmed(),
                    "is not a file, skipping".dimmed()
                );
                None
            } else {
                Some(f.to_owned())
            }
        })
        .collect()
}

fn add_psu_defaults(psu: &mut PSU, name: &str, file_count: usize, timestamp: NaiveDateTime) {
    psu.entries.push(PSUEntry {
        id: DIR_ID,
        size: file_count as u32 + 2,
        created: timestamp,
        sector: 0,
        modified: timestamp,
        name: name.to_owned(),
        kind: PSUEntryKind::Directory,
        contents: None,
    });
    psu.entries.push(PSUEntry {
        id: DIR_ID,
        size: 0,
        created: timestamp,
        sector: 0,
        modified: timestamp,
        name: ".".to_string(),
        kind: PSUEntryKind::Directory,
        contents: None,
    });
    psu.entries.push(PSUEntry {
        id: DIR_ID,
        size: 0,
        created: timestamp,
        sector: 0,
        modified: timestamp,
        name: "..".to_string(),
        kind: PSUEntryKind::Directory,
        contents: None,
    });
}

fn add_files_to_psu<M: MetadataReader>(
    psu: &mut PSU,
    files: &[PathBuf],
    timestamp: Option<NaiveDateTime>,
    metadata_reader: &M,
) -> Result<(), Error> {
    for file in files {
        let name = file.file_name().unwrap().to_str().unwrap();

        let f = std::fs::read(file)?;
        let (created, modified) = if let Some(timestamp) = timestamp {
            (timestamp, timestamp)
        } else {
            let file_times = metadata_reader.file_times(file)?;
            let modified = convert_timestamp(file_times.modified);
            let created = file_times
                .created
                .map(convert_timestamp)
                .unwrap_or(modified);
            (created, modified)
        };

        println!("+ {} {}", "Adding", name.green());

        psu.entries.push(PSUEntry {
            id: FILE_ID,
            size: f.len() as u32,
            created,
            sector: 0,
            modified,
            name: name.to_owned(),
            kind: PSUEntryKind::File,
            contents: Some(f),
        })
    }

    Ok(())
}

fn convert_timestamp(time: SystemTime) -> NaiveDateTime {
    let duration = time.duration_since(UNIX_EPOCH).unwrap();
    DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
        .unwrap()
        .with_timezone(&Local)
        .naive_local()
}

#[derive(Debug)]
pub enum Error {
    NameError,
    IOError(std::io::Error),
    ConfigError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::NameError => write!(f, "Name must match [a-zA-Z0-9._-\\s]+"),
            Error::IOError(err) => write!(f, "{err:?}"),
            Error::ConfigError(err) => write!(f, "{err}"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IOError(err)
    }
}
