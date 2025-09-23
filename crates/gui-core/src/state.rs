use std::{
    collections::HashSet,
    ffi::OsStr,
    fs, io,
    ops::Index,
    path::{Path, PathBuf},
};

use crate::actions::{Action, ActionDispatcher, MetadataTarget};
use crate::commands::AppEvent;
use crate::validation::{sanitize_seconds_between_items, timestamp_rules_equal};
use psu_packer::sas::{canonical_aliases_for_category, CategoryRule, TimestampRules};

pub const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
pub const TIMESTAMP_RULES_FILE: &str = "timestamp_rules.json";
pub const REQUIRED_PROJECT_FILES: &[&str] =
    &["list.icn", "copy.icn", "del.icn", "title.cfg", "icon.sys"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MissingFileReason {
    AlwaysRequired,
    ExplicitlyIncluded,
    TimestampAutomation,
}

impl MissingFileReason {
    pub fn detail(&self) -> Option<&'static str> {
        match self {
            MissingFileReason::AlwaysRequired => None,
            MissingFileReason::ExplicitlyIncluded => Some("listed in Include files"),
            MissingFileReason::TimestampAutomation => Some("needed for SAS timestamp automation"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingRequiredFile {
    pub name: String,
    pub reason: MissingFileReason,
}

impl MissingRequiredFile {
    pub fn always(name: &str) -> Self {
        Self {
            name: name.to_string(),
            reason: MissingFileReason::AlwaysRequired,
        }
    }

    pub fn included(name: &str) -> Self {
        Self {
            name: name.to_string(),
            reason: MissingFileReason::ExplicitlyIncluded,
        }
    }

    pub fn timestamp_rules() -> Self {
        Self {
            name: TIMESTAMP_RULES_FILE.to_string(),
            reason: MissingFileReason::TimestampAutomation,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectRequirementStatus {
    pub file: MissingRequiredFile,
    pub satisfied: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SasPrefix {
    None,
    App,
    Apps,
    Ps1,
    Emu,
    Gme,
    Dst,
    Dbg,
    Raa,
    Rte,
    Default,
    Sys,
    Zzy,
    Zzz,
}

const SAS_PREFIXES: [SasPrefix; 13] = [
    SasPrefix::App,
    SasPrefix::Apps,
    SasPrefix::Ps1,
    SasPrefix::Emu,
    SasPrefix::Gme,
    SasPrefix::Dst,
    SasPrefix::Dbg,
    SasPrefix::Raa,
    SasPrefix::Rte,
    SasPrefix::Default,
    SasPrefix::Sys,
    SasPrefix::Zzy,
    SasPrefix::Zzz,
];

impl Default for SasPrefix {
    fn default() -> Self {
        SasPrefix::App
    }
}

impl SasPrefix {
    pub const fn as_str(self) -> &'static str {
        match self {
            SasPrefix::None => "",
            SasPrefix::App => "APP_",
            SasPrefix::Apps => "APPS",
            SasPrefix::Ps1 => "PS1_",
            SasPrefix::Emu => "EMU_",
            SasPrefix::Gme => "GME_",
            SasPrefix::Dst => "DST_",
            SasPrefix::Dbg => "DBG_",
            SasPrefix::Raa => "RAA_",
            SasPrefix::Rte => "RTE_",
            SasPrefix::Default => "DEFAULT",
            SasPrefix::Sys => "SYS_",
            SasPrefix::Zzy => "ZZY_",
            SasPrefix::Zzz => "ZZZ_",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            SasPrefix::None => "(none)",
            SasPrefix::Default => "DEFAULT",
            _ => self.as_str(),
        }
    }

    pub fn iter_prefixed() -> impl Iterator<Item = SasPrefix> {
        SAS_PREFIXES.into_iter()
    }

    pub fn iter_with_unprefixed() -> impl Iterator<Item = SasPrefix> {
        std::iter::once(SasPrefix::None).chain(Self::iter_prefixed())
    }

    pub fn split_from_name(name: &str) -> (SasPrefix, &str) {
        for prefix in SAS_PREFIXES {
            let remainder = match prefix {
                SasPrefix::Default => name
                    .strip_prefix("DEFAULT_")
                    .or_else(|| name.strip_prefix(prefix.as_str())),
                _ => name.strip_prefix(prefix.as_str()),
            };
            if let Some(remainder) = remainder {
                return (prefix, remainder);
            }
        }
        (SasPrefix::None, name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimestampStrategy {
    None,
    InheritSource,
    SasRules,
    Manual,
}

impl Default for TimestampStrategy {
    fn default() -> Self {
        TimestampStrategy::None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimestampRulesUiState {
    seconds_between_items: u32,
    slots_per_category: u32,
    categories: Vec<CategoryUiState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryUiState {
    key: String,
    available_aliases: Vec<String>,
    selected_aliases: HashSet<String>,
}

impl TimestampRulesUiState {
    pub fn from_rules(rules: &TimestampRules) -> Self {
        let mut sanitized = rules.clone();
        sanitized.sanitize();

        let categories = sanitized
            .categories
            .iter()
            .map(|category| {
                let available_aliases = canonical_aliases_for_category(category.key.as_str())
                    .iter()
                    .map(|alias| (*alias).to_string())
                    .collect::<Vec<_>>();
                let selected_aliases = category
                    .aliases
                    .iter()
                    .filter(|alias| {
                        available_aliases
                            .iter()
                            .any(|candidate| candidate == *alias)
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                CategoryUiState::new(category.key.clone(), available_aliases, selected_aliases)
            })
            .collect();

        Self {
            seconds_between_items: sanitize_seconds_between_items(sanitized.seconds_between_items),
            slots_per_category: sanitized.slots_per_category.max(1),
            categories,
        }
    }

    pub fn ensure_matches(&mut self, rules: &TimestampRules) {
        let refreshed = Self::from_rules(rules);
        if *self != refreshed {
            *self = refreshed;
        }
    }

    pub fn len(&self) -> usize {
        self.categories.len()
    }

    pub fn seconds_between_items(&self) -> u32 {
        self.seconds_between_items
    }

    pub fn slots_per_category(&self) -> u32 {
        self.slots_per_category
    }

    pub fn set_seconds_between_items(&mut self, value: u32) -> bool {
        let sanitized = sanitize_seconds_between_items(value);
        if sanitized != self.seconds_between_items {
            self.seconds_between_items = sanitized;
            true
        } else {
            false
        }
    }

    pub fn set_slots_per_category(&mut self, value: u32) -> bool {
        let sanitized = value.max(1);
        if sanitized != self.slots_per_category {
            self.slots_per_category = sanitized;
            true
        } else {
            false
        }
    }

    pub fn category(&self, index: usize) -> Option<&CategoryUiState> {
        self.categories.get(index)
    }

    pub fn category_mut(&mut self, index: usize) -> Option<&mut CategoryUiState> {
        self.categories.get_mut(index)
    }

    pub fn move_category_up(&mut self, index: usize) -> bool {
        if index == 0 || index >= self.categories.len() {
            return false;
        }
        self.categories.swap(index - 1, index);
        true
    }

    pub fn move_category_down(&mut self, index: usize) -> bool {
        if index + 1 >= self.categories.len() {
            return false;
        }
        self.categories.swap(index, index + 1);
        true
    }

    pub fn set_alias_selected(&mut self, index: usize, alias: &str, selected: bool) -> bool {
        self.category_mut(index)
            .map(|category| category.set_alias_selected(alias, selected))
            .unwrap_or(false)
    }

    pub fn alias_warning(&self, index: usize) -> Option<String> {
        self.category(index).and_then(|category| category.warning())
    }

    pub fn apply_to_rules(&self, rules: &mut TimestampRules) -> bool {
        let new_rules = self.to_rules();
        let changed = !timestamp_rules_equal(rules, &new_rules);
        if changed {
            *rules = new_rules;
        } else {
            // ensure sanitized values propagate even when unchanged
            *rules = new_rules;
        }
        changed
    }

    pub fn serialize(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.to_rules())
    }

    fn to_rules(&self) -> TimestampRules {
        let mut rules = TimestampRules {
            seconds_between_items: sanitize_seconds_between_items(self.seconds_between_items),
            slots_per_category: self.slots_per_category.max(1),
            categories: self
                .categories
                .iter()
                .map(|category| CategoryRule {
                    key: category.key.clone(),
                    aliases: category.sorted_aliases(),
                })
                .collect(),
        };
        rules.sanitize();
        rules.seconds_between_items = sanitize_seconds_between_items(self.seconds_between_items);
        rules.slots_per_category = self.slots_per_category.max(1);
        rules
    }
}

impl CategoryUiState {
    fn new(key: String, available_aliases: Vec<String>, selected_aliases: Vec<String>) -> Self {
        let selected_aliases = selected_aliases.into_iter().collect::<HashSet<_>>();
        Self {
            key,
            available_aliases,
            selected_aliases,
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn available_aliases(&self) -> &[String] {
        &self.available_aliases
    }

    pub fn alias_count(&self) -> usize {
        self.selected_aliases.len()
    }

    pub fn is_alias_selected(&self, alias: &str) -> bool {
        self.selected_aliases.contains(alias)
    }

    fn set_alias_selected(&mut self, alias: &str, selected: bool) -> bool {
        if !self
            .available_aliases
            .iter()
            .any(|candidate| candidate == alias)
        {
            return false;
        }

        if selected {
            if self.selected_aliases.insert(alias.to_string()) {
                return true;
            }
        } else if self.selected_aliases.remove(alias) {
            return true;
        }
        false
    }

    fn sorted_aliases(&self) -> Vec<String> {
        self.available_aliases
            .iter()
            .filter(|alias| self.selected_aliases.contains(alias.as_str()))
            .cloned()
            .collect()
    }

    fn warning(&self) -> Option<String> {
        if self.available_aliases.is_empty() || !self.selected_aliases.is_empty() {
            None
        } else {
            let alias_list = self.available_aliases.join(", ");
            Some(format!(
                "No aliases selected. Unprefixed names ({alias_list}) will fall back to DEFAULT scheduling."
            ))
        }
    }
}

#[derive(Default, Clone)]
pub struct Files(pub Vec<VirtualFile>, pub u64);

impl Files {
    pub fn from(files: Vec<VirtualFile>) -> io::Result<Self> {
        let mut slf = Self(files, 0);
        slf.recalculate_size()?;
        slf.sort();
        Ok(slf)
    }

    pub fn add_file<P: AsRef<Path>>(&mut self, file_path: P) -> io::Result<()> {
        let name = file_path
            .as_ref()
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidFilename, "Invalid file name"))?
            .to_string();
        let size = fs::metadata(&file_path)?.len();

        self.0.push(VirtualFile {
            name,
            file_path: file_path.as_ref().into(),
            size,
        });
        self.recalculate_size()?;

        Ok(())
    }

    fn sort(&mut self) {
        self.0.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());
    }

    fn recalculate_size(&mut self) -> io::Result<()> {
        let total = self
            .0
            .iter()
            .map(|file| 512 + aligned_file_size(file.size))
            .sum::<u64>();
        self.1 = (512 * 3) + total;
        Ok(())
    }

    pub fn calculated_size(&self) -> u64 {
        self.1
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &VirtualFile> {
        self.0.iter()
    }
}

impl Index<usize> for Files {
    type Output = VirtualFile;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

fn aligned_file_size(size: u64) -> u64 {
    ((size + 1023) as i64 & -1024) as u64
}

#[derive(Clone)]
pub struct VirtualFile {
    pub name: String,
    pub file_path: PathBuf,
    pub size: u64,
}

pub struct AppState {
    pub opened_folder: Option<PathBuf>,
    pub files: Files,
    pub events: Vec<AppEvent>,
    pub pcsx2_path: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            opened_folder: None,
            files: Files::default(),
            events: vec![],
            pcsx2_path: String::new(),
        }
    }

    pub fn open_file(&mut self, file: VirtualFile) {
        self.events.push(AppEvent::OpenFile(file));
    }

    pub fn set_title(&mut self, title: String) {
        self.events.push(AppEvent::SetTitle(title));
    }

    pub fn add_files(&mut self) {
        self.events.push(AppEvent::AddFiles);
    }

    pub fn open_folder(&mut self) {
        self.events.push(AppEvent::OpenFolder);
    }

    pub fn open_save(&mut self) {
        self.events.push(AppEvent::OpenSave);
    }

    pub fn export_psu(&mut self) {
        self.events.push(AppEvent::ExportPSU);
    }

    pub fn save_file(&mut self) {
        self.events.push(AppEvent::SaveFile);
    }

    pub fn create_icn(&mut self) {
        self.events.push(AppEvent::CreateICN);
    }

    pub fn create_psu_toml(&mut self) {
        self.events.push(AppEvent::CreatePsuToml);
    }

    pub fn create_title_cfg(&mut self) {
        self.events.push(AppEvent::CreateTitleCfg);
    }

    pub fn open_settings(&mut self) {
        self.events.push(AppEvent::OpenSettings);
    }

    pub fn start_pcsx2(&mut self) {
        self.events.push(AppEvent::StartPCSX2);
    }

    pub fn start_pcsx2_elf(&mut self, path: PathBuf) {
        self.events.push(AppEvent::StartPCSX2Elf(path));
    }

    pub fn validate(&mut self) {
        self.events.push(AppEvent::Validate);
    }
}

impl ActionDispatcher for AppState {
    fn is_action_enabled(&self, action: Action) -> bool {
        match action {
            Action::PackPsu
            | Action::AddFiles
            | Action::SaveFile
            | Action::CreateMetadataTemplate(MetadataTarget::PsuToml)
            | Action::CreateMetadataTemplate(MetadataTarget::TitleCfg)
            | Action::EditMetadata(_) => self.opened_folder.is_some(),
            _ => true,
        }
    }

    fn trigger_action(&mut self, action: Action) {
        match action {
            Action::OpenProject => self.open_folder(),
            Action::PackPsu => self.export_psu(),
            Action::AddFiles => self.add_files(),
            Action::SaveFile => self.save_file(),
            Action::CreateMetadataTemplate(MetadataTarget::PsuToml) => self.create_psu_toml(),
            Action::CreateMetadataTemplate(MetadataTarget::TitleCfg) => self.create_title_cfg(),
            Action::OpenSettings => self.open_settings(),
            _ => {}
        }
    }

    fn supports_action(&self, action: Action) -> bool {
        matches!(
            action,
            Action::OpenProject
                | Action::PackPsu
                | Action::AddFiles
                | Action::SaveFile
                | Action::CreateMetadataTemplate(MetadataTarget::PsuToml)
                | Action::CreateMetadataTemplate(MetadataTarget::TitleCfg)
                | Action::OpenSettings
        )
    }
}
