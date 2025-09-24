use std::{
    collections::HashSet,
    ffi::OsStr,
    fs, io,
    ops::Index,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use crate::actions::{Action, ActionDispatcher, FileListKind, MetadataTarget};
use crate::commands::AppEvent;
use crate::validation::{sanitize_seconds_between_items, timestamp_rules_equal};
use psu_packer::sas::{
    canonical_aliases_for_category, planned_timestamp_for_folder, planned_timestamp_for_name,
    CategoryRule, TimestampRules,
};
use tempfile::{tempdir, TempDir};

use chrono::NaiveDateTime;
use ps2_filetypes::{templates, PSUEntryKind, PSU};

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

#[derive(Debug)]
pub struct PackErrorMessage {
    message: String,
    failed_files: Vec<String>,
}

impl From<String> for PackErrorMessage {
    fn from(message: String) -> Self {
        Self {
            message,
            failed_files: Vec::new(),
        }
    }
}

impl From<&str> for PackErrorMessage {
    fn from(message: &str) -> Self {
        Self {
            message: message.to_owned(),
            failed_files: Vec::new(),
        }
    }
}

impl<S> From<(S, Vec<String>)> for PackErrorMessage
where
    S: Into<String>,
{
    fn from((message, failed_files): (S, Vec<String>)) -> Self {
        Self {
            message: message.into(),
            failed_files,
        }
    }
}

pub struct PackJob {
    pub progress: Arc<Mutex<PackProgress>>,
    pub handle: Option<thread::JoinHandle<()>>,
}

pub enum PackProgress {
    InProgress,
    Finished(PackOutcome),
}

pub struct PackPreparation {
    pub folder: PathBuf,
    pub config: psu_packer::Config,
    pub missing_required_files: Vec<MissingRequiredFile>,
}

pub enum PackOutcome {
    Success {
        output_path: PathBuf,
    },
    Error {
        folder: PathBuf,
        output_path: PathBuf,
        error: psu_packer::Error,
    },
}

pub enum PendingPackAction {
    Pack {
        folder: PathBuf,
        output_path: PathBuf,
        config: psu_packer::Config,
        missing_required_files: Vec<MissingRequiredFile>,
    },
}

impl PendingPackAction {
    pub fn missing_files(&self) -> &[MissingRequiredFile] {
        match self {
            PendingPackAction::Pack {
                missing_required_files,
                ..
            } => missing_required_files,
        }
    }
}

pub struct PackerState {
    pub folder: Option<PathBuf>,
    pub output: String,
    pub status: String,
    pub error_message: Option<String>,
    pub selected_prefix: SasPrefix,
    pub folder_base_name: String,
    pub psu_file_base_name: String,
    pub timestamp: Option<NaiveDateTime>,
    pub timestamp_strategy: TimestampStrategy,
    pub timestamp_from_rules: bool,
    pub source_timestamp: Option<NaiveDateTime>,
    pub manual_timestamp: Option<NaiveDateTime>,
    pub timestamp_rules: TimestampRules,
    pub timestamp_rules_loaded_from_file: bool,
    pub timestamp_rules_modified: bool,
    pub timestamp_rules_error: Option<String>,
    pub timestamp_rules_ui: TimestampRulesUiState,
    pub include_files: Vec<String>,
    pub exclude_files: Vec<String>,
    pub include_manual_entry: String,
    pub exclude_manual_entry: String,
    pub selected_include: Option<usize>,
    pub selected_exclude: Option<usize>,
    pub missing_required_project_files: Vec<MissingRequiredFile>,
    pub pending_pack_action: Option<PendingPackAction>,
    pub loaded_psu_path: Option<PathBuf>,
    pub loaded_psu_files: Vec<String>,
    pub source_present_last_frame: bool,
    pub pack_job: Option<PackJob>,
    pub temp_workspace: Option<TempDir>,
    pub events: Vec<AppEvent>,
}

impl Default for PackerState {
    fn default() -> Self {
        let timestamp_rules = TimestampRules::default();
        let timestamp_rules_ui = TimestampRulesUiState::from_rules(&timestamp_rules);
        Self {
            folder: None,
            output: String::new(),
            status: String::new(),
            error_message: None,
            selected_prefix: SasPrefix::default(),
            folder_base_name: String::new(),
            psu_file_base_name: String::new(),
            timestamp: None,
            timestamp_strategy: TimestampStrategy::default(),
            timestamp_from_rules: false,
            source_timestamp: None,
            manual_timestamp: None,
            timestamp_rules,
            timestamp_rules_loaded_from_file: false,
            timestamp_rules_modified: false,
            timestamp_rules_error: None,
            timestamp_rules_ui,
            include_files: Vec::new(),
            exclude_files: Vec::new(),
            include_manual_entry: String::new(),
            exclude_manual_entry: String::new(),
            selected_include: None,
            selected_exclude: None,
            missing_required_project_files: Vec::new(),
            pending_pack_action: None,
            loaded_psu_path: None,
            loaded_psu_files: Vec::new(),
            source_present_last_frame: false,
            pack_job: None,
            temp_workspace: None,
            events: Vec::new(),
        }
    }
}

impl PackerState {
    fn timestamp_rules_path_from(folder: &Path) -> PathBuf {
        folder.join(TIMESTAMP_RULES_FILE)
    }

    pub fn timestamp_rules_path(&self) -> Option<PathBuf> {
        self.folder
            .as_ref()
            .map(|folder| Self::timestamp_rules_path_from(folder))
    }

    fn active_project_requirements(&self) -> Vec<MissingRequiredFile> {
        let mut requirements = REQUIRED_PROJECT_FILES
            .iter()
            .map(|name| MissingRequiredFile::always(name))
            .collect::<Vec<_>>();

        if self.include_requires_file("BOOT.ELF") {
            requirements.push(MissingRequiredFile::included("BOOT.ELF"));
        }

        if self.uses_timestamp_rules_file() {
            requirements.push(MissingRequiredFile::timestamp_rules());
        }

        requirements
    }

    pub fn missing_required_project_files_for(&self, folder: &Path) -> Vec<MissingRequiredFile> {
        let mut missing = Vec::new();

        for requirement in self.active_project_requirements() {
            let candidate = folder.join(&requirement.name);
            if !candidate.is_file() {
                missing.push(requirement);
            }
        }

        missing
    }

    fn include_requires_file(&self, file_name: &str) -> bool {
        self.include_files
            .iter()
            .any(|entry| entry.eq_ignore_ascii_case(file_name))
    }

    fn uses_timestamp_rules_file(&self) -> bool {
        matches!(self.timestamp_strategy, TimestampStrategy::SasRules)
            && (self.timestamp_rules_loaded_from_file || self.timestamp_rules_modified)
    }

    pub fn refresh_missing_required_project_files(&mut self) {
        if let Some(folder) = self.folder.clone() {
            self.missing_required_project_files = self.missing_required_project_files_for(&folder);
        } else {
            self.missing_required_project_files.clear();
        }
    }

    pub fn project_requirement_statuses(&self) -> Option<Vec<ProjectRequirementStatus>> {
        self.folder.as_ref()?;

        let missing_names: HashSet<&str> = self
            .missing_required_project_files
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();

        let statuses = self
            .active_project_requirements()
            .into_iter()
            .map(|file| {
                let satisfied = !missing_names.contains(file.name.as_str());
                ProjectRequirementStatus { file, satisfied }
            })
            .collect::<Vec<_>>();

        Some(statuses)
    }

    pub fn pending_pack_missing_files(&self) -> Option<&[MissingRequiredFile]> {
        self.pending_pack_action
            .as_ref()
            .map(|action| action.missing_files())
    }

    pub fn take_events(&mut self) -> Vec<AppEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn request_output_destination_dialog(&mut self) {
        let default_directory = self.default_output_directory(None);
        let default_file_name = self.default_output_file_name();
        self.events.push(AppEvent::ChooseOutputDestination {
            default_directory,
            default_file_name,
        });
    }

    pub fn request_file_list_entries(&mut self, kind: FileListKind) {
        if let Some(folder) = self.folder.clone() {
            self.events.push(AppEvent::BrowseFileListEntries {
                project_root: folder,
                kind,
            });
        }
    }

    pub fn request_export_folder_dialog(&mut self) {
        let default_directory = self.default_output_directory(None);
        self.events
            .push(AppEvent::ChooseExportFolder { default_directory });
    }

    pub fn request_metadata_template(&mut self, target: MetadataTarget) {
        let template = match target {
            MetadataTarget::PsuToml => Some(templates::PSU_TOML_TEMPLATE.to_string()),
            MetadataTarget::TitleCfg => Some(templates::TITLE_CFG_TEMPLATE.to_string()),
            MetadataTarget::IconSys => None,
        };

        if let Some(template) = template {
            let destination = self.folder.clone();
            self.events.push(AppEvent::CreateMetadataTemplate {
                target,
                template,
                destination,
            });
        }
    }

    pub fn request_pack_confirmation(&mut self) {
        if let Some(missing_required_files) = self
            .pending_pack_missing_files()
            .map(|files| files.to_vec())
        {
            self.events.push(AppEvent::ShowPackConfirmation {
                missing_required_files,
            });
        }
    }

    pub fn confirm_pending_pack_action(
        &mut self,
    ) -> Option<(PathBuf, PathBuf, psu_packer::Config)> {
        let action = self.pending_pack_action.take()?;
        match action {
            PendingPackAction::Pack {
                folder,
                output_path,
                config,
                ..
            } => Some((folder, output_path, config)),
        }
    }

    pub fn cancel_pending_pack_action(&mut self) {
        self.pending_pack_action = None;
    }

    pub fn load_timestamp_rules_from_folder(&mut self, folder: &Path) {
        let path = Self::timestamp_rules_path_from(folder);
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<TimestampRules>(&content) {
                Ok(mut rules) => {
                    rules.sanitize();
                    self.timestamp_rules = rules;
                    self.timestamp_rules_error = None;
                    self.timestamp_rules_loaded_from_file = true;
                }
                Err(err) => {
                    self.timestamp_rules = TimestampRules::default();
                    self.timestamp_rules_error =
                        Some(format!("Failed to parse {}: {err}", path.display()));
                    self.timestamp_rules_loaded_from_file = true;
                }
            },
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    self.timestamp_rules = TimestampRules::default();
                    self.timestamp_rules_error = None;
                    self.timestamp_rules_loaded_from_file = false;
                } else {
                    self.timestamp_rules = TimestampRules::default();
                    self.timestamp_rules_error =
                        Some(format!("Failed to read {}: {err}", path.display()));
                    self.timestamp_rules_loaded_from_file = true;
                }
            }
        }

        self.timestamp_rules_ui = TimestampRulesUiState::from_rules(&self.timestamp_rules);
        self.timestamp_rules_ui
            .apply_to_rules(&mut self.timestamp_rules);
        self.timestamp_rules_modified = false;
    }

    pub fn save_timestamp_rules(&mut self) -> Result<PathBuf, String> {
        let Some(folder) = self.folder.as_ref() else {
            return Err("Select a folder before saving timestamp rules.".to_string());
        };

        self.timestamp_rules_ui
            .apply_to_rules(&mut self.timestamp_rules);
        let serialized = self
            .timestamp_rules_ui
            .serialize()
            .map_err(|err| format!("Failed to serialize timestamp rules: {err}"))?;

        let path = Self::timestamp_rules_path_from(folder);
        fs::write(&path, serialized)
            .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;

        self.timestamp_rules_ui = TimestampRulesUiState::from_rules(&self.timestamp_rules);
        self.timestamp_rules_modified = false;
        self.timestamp_rules_error = None;
        self.timestamp_rules_loaded_from_file = true;
        Ok(path)
    }

    pub fn set_timestamp_strategy(&mut self, strategy: TimestampStrategy) -> bool {
        if self.timestamp_strategy == strategy {
            return false;
        }

        self.timestamp_strategy = strategy;

        if matches!(self.timestamp_strategy, TimestampStrategy::Manual)
            && self.manual_timestamp.is_none()
        {
            if let Some(source) = self.source_timestamp {
                self.manual_timestamp = Some(source);
            } else if let Some(planned) = self.planned_timestamp_for_current_source() {
                self.manual_timestamp = Some(planned);
            }
        }

        self.refresh_timestamp_from_strategy()
    }

    pub fn refresh_timestamp_from_strategy(&mut self) -> bool {
        let new_timestamp = match self.timestamp_strategy {
            TimestampStrategy::None => None,
            TimestampStrategy::InheritSource => self.source_timestamp,
            TimestampStrategy::SasRules => self.planned_timestamp_for_current_source(),
            TimestampStrategy::Manual => self.manual_timestamp,
        };

        let changed = self.timestamp != new_timestamp;
        self.timestamp = new_timestamp;
        self.timestamp_from_rules = matches!(self.timestamp_strategy, TimestampStrategy::SasRules)
            && self.timestamp.is_some();
        changed
    }

    pub fn sync_timestamp_after_source_update(&mut self) -> bool {
        let planned = self.planned_timestamp_for_current_source();

        if matches!(self.timestamp_strategy, TimestampStrategy::None) {
            if self.source_timestamp.is_some() {
                self.timestamp_strategy = TimestampStrategy::InheritSource;
            } else if planned.is_some() {
                self.timestamp_strategy = TimestampStrategy::SasRules;
            }
        }

        if matches!(self.timestamp_strategy, TimestampStrategy::Manual)
            && self.manual_timestamp.is_none()
        {
            if let Some(source) = self.source_timestamp {
                self.manual_timestamp = Some(source);
            } else if let Some(planned) = planned {
                self.manual_timestamp = Some(planned);
            }
        }

        self.refresh_timestamp_from_strategy()
    }

    pub fn mark_timestamp_rules_modified(&mut self) {
        self.timestamp_rules_ui
            .apply_to_rules(&mut self.timestamp_rules);
        self.timestamp_rules_modified = true;
        self.recompute_timestamp_from_rules();
    }

    fn recompute_timestamp_from_rules(&mut self) {
        if !matches!(self.timestamp_strategy, TimestampStrategy::SasRules) {
            return;
        }

        self.refresh_timestamp_from_strategy();
    }

    pub fn apply_planned_timestamp(&mut self) {
        self.set_timestamp_strategy(TimestampStrategy::SasRules);
    }

    pub fn planned_timestamp_for_current_source(&self) -> Option<NaiveDateTime> {
        if let Some(folder) = self.folder.as_ref() {
            return planned_timestamp_for_folder(folder.as_path(), &self.timestamp_rules);
        }

        let name = self.folder_name();
        if name.trim().is_empty() {
            return None;
        }

        planned_timestamp_for_name(&name, &self.timestamp_rules)
    }

    pub fn reset_timestamp_rules_to_default(&mut self) {
        self.timestamp_rules = TimestampRules::default();
        self.timestamp_rules_error = None;
        self.timestamp_rules_ui = TimestampRulesUiState::from_rules(&self.timestamp_rules);
        self.timestamp_rules_ui
            .apply_to_rules(&mut self.timestamp_rules);
        self.timestamp_rules_loaded_from_file = false;
        self.mark_timestamp_rules_modified();
    }

    pub fn set_error_message<M>(&mut self, message: M)
    where
        M: Into<PackErrorMessage>,
    {
        let message = message.into();
        let mut text = message.message;
        if !message.failed_files.is_empty() {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str("Failed files: ");
            text.push_str(&message.failed_files.join(", "));
        }
        self.error_message = Some(text);
        self.status.clear();
    }

    pub fn format_missing_required_files_message(missing: &[MissingRequiredFile]) -> String {
        crate::validation::format_missing_required_files_message(missing)
    }

    pub fn clear_error_message(&mut self) {
        self.error_message = None;
    }

    pub fn reset_metadata_fields(&mut self) {
        self.selected_prefix = SasPrefix::default();
        self.folder_base_name.clear();
        self.psu_file_base_name.clear();
        self.timestamp = None;
        self.timestamp_strategy = TimestampStrategy::None;
        self.timestamp_from_rules = false;
        self.source_timestamp = None;
        self.manual_timestamp = None;
        self.include_files.clear();
        self.exclude_files.clear();
        self.include_manual_entry.clear();
        self.exclude_manual_entry.clear();
        self.selected_include = None;
        self.selected_exclude = None;
    }

    pub fn folder_name(&self) -> String {
        let mut name = String::from(self.selected_prefix.as_str());
        name.push_str(&self.folder_base_name);
        name
    }

    fn effective_psu_file_base_name(&self) -> Option<String> {
        let trimmed_file = self.psu_file_base_name.trim();
        if !trimmed_file.is_empty() {
            return Some(trimmed_file.to_string());
        }

        let trimmed_folder = self.folder_base_name.trim();
        if trimmed_folder.is_empty() {
            None
        } else {
            Some(trimmed_folder.to_string())
        }
    }

    fn existing_output_directory(&self) -> Option<PathBuf> {
        let trimmed_output = self.output.trim();
        if trimmed_output.is_empty() {
            return None;
        }

        let path = Path::new(trimmed_output);
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(|parent| parent.to_path_buf())
    }

    fn loaded_psu_directory(&self) -> Option<PathBuf> {
        self.loaded_psu_path
            .as_ref()
            .and_then(|path| path.parent())
            .map(|parent| parent.to_path_buf())
    }

    pub fn default_output_directory(&self, fallback_dir: Option<&Path>) -> Option<PathBuf> {
        if let Some(existing) = self.existing_output_directory() {
            return Some(existing);
        }

        if let Some(dir) = fallback_dir {
            return Some(dir.to_path_buf());
        }

        if let Some(folder) = self.folder.as_ref() {
            return Some(folder.clone());
        }

        self.loaded_psu_directory()
    }

    pub fn default_output_path(&self) -> Option<PathBuf> {
        self.default_output_path_with(None)
    }

    pub fn default_output_path_with(&self, fallback_dir: Option<&Path>) -> Option<PathBuf> {
        let file_name = self.default_output_file_name()?;
        let directory = self.default_output_directory(fallback_dir);
        Some(match directory {
            Some(dir) => dir.join(file_name),
            None => PathBuf::from(file_name),
        })
    }

    pub fn default_output_file_name(&self) -> Option<String> {
        let base_name = self.effective_psu_file_base_name()?;
        let mut stem = String::from(self.selected_prefix.as_str());
        stem.push_str(&base_name);
        if stem.is_empty() {
            None
        } else {
            Some(format!("{stem}.psu"))
        }
    }

    fn update_output_if_matches_default(&mut self, previous_default_output: Option<String>) {
        let should_update = if self.output.trim().is_empty() {
            true
        } else if let Some(previous_default) = previous_default_output {
            Path::new(&self.output)
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name == previous_default)
                .unwrap_or(false)
        } else {
            false
        };

        if should_update {
            match self.default_output_path() {
                Some(path) => {
                    self.output = path.display().to_string();
                }
                None => self.output.clear(),
            }
        }
    }

    pub fn metadata_inputs_changed(&mut self, previous_default_output: Option<String>) {
        if self.psu_file_base_name.trim().is_empty() {
            let trimmed_folder = self.folder_base_name.trim();
            if !trimmed_folder.is_empty() {
                self.psu_file_base_name = trimmed_folder.to_string();
            }
        }

        self.update_output_if_matches_default(previous_default_output);
        self.ensure_timestamp_strategy_default();
        if matches!(self.timestamp_strategy, TimestampStrategy::SasRules) {
            self.refresh_timestamp_from_strategy();
        }
    }

    pub fn set_selected_prefix(&mut self, prefix: SasPrefix) -> bool {
        if self.selected_prefix == prefix {
            return false;
        }

        let previous_default = self.default_output_file_name();
        self.selected_prefix = prefix;
        self.metadata_inputs_changed(previous_default);
        true
    }

    pub fn set_folder_base_name<S>(&mut self, base_name: S) -> bool
    where
        S: Into<String>,
    {
        let base_name = base_name.into();
        if self.folder_base_name == base_name {
            return false;
        }

        let previous_default = self.default_output_file_name();
        self.folder_base_name = base_name;
        self.metadata_inputs_changed(previous_default);
        true
    }

    pub fn set_psu_file_base_name<S>(&mut self, base_name: S) -> bool
    where
        S: Into<String>,
    {
        let base_name = base_name.into();
        if self.psu_file_base_name == base_name {
            return false;
        }

        let previous_default = self.default_output_file_name();
        self.psu_file_base_name = base_name;
        self.metadata_inputs_changed(previous_default);
        true
    }

    pub fn set_manual_timestamp(&mut self, timestamp: Option<NaiveDateTime>) -> bool {
        if self.manual_timestamp == timestamp {
            return false;
        }

        self.manual_timestamp = timestamp;
        if matches!(self.timestamp_strategy, TimestampStrategy::Manual) {
            self.refresh_timestamp_from_strategy();
        }
        true
    }

    pub fn ensure_manual_timestamp(&mut self, default: NaiveDateTime) -> bool {
        if self.manual_timestamp.is_some() {
            return false;
        }

        self.set_manual_timestamp(Some(default))
    }

    pub fn file_list_parts_mut(
        &mut self,
        kind: FileListKind,
    ) -> (&mut Vec<String>, &mut Option<usize>, &mut String) {
        match kind {
            FileListKind::Include => (
                &mut self.include_files,
                &mut self.selected_include,
                &mut self.include_manual_entry,
            ),
            FileListKind::Exclude => (
                &mut self.exclude_files,
                &mut self.selected_exclude,
                &mut self.exclude_manual_entry,
            ),
        }
    }

    pub fn file_list_entries(&self, kind: FileListKind) -> &[String] {
        match kind {
            FileListKind::Include => &self.include_files,
            FileListKind::Exclude => &self.exclude_files,
        }
    }

    pub fn file_list_selection(&self, kind: FileListKind) -> Option<usize> {
        match kind {
            FileListKind::Include => self.selected_include,
            FileListKind::Exclude => self.selected_exclude,
        }
    }

    pub fn select_file_list_entry(&mut self, kind: FileListKind, selection: Option<usize>) {
        let (files, selected, _) = self.file_list_parts_mut(kind);
        if let Some(index) = selection {
            if index < files.len() {
                *selected = Some(index);
            } else if files.is_empty() {
                *selected = None;
            } else {
                *selected = Some(files.len() - 1);
            }
        } else {
            *selected = None;
        }
    }

    pub fn set_file_list_entries(&mut self, kind: FileListKind, entries: Vec<String>) {
        let (files, selected, _) = self.file_list_parts_mut(kind);
        *files = entries;
        *selected = None;
    }

    pub fn add_file_list_entry(&mut self, kind: FileListKind, entry: String) -> usize {
        let (files, selected, _) = self.file_list_parts_mut(kind);
        files.push(entry);
        let index = files.len() - 1;
        *selected = Some(index);
        index
    }

    pub fn remove_file_list_entry(&mut self, kind: FileListKind, index: usize) -> Option<String> {
        let (files, selected, _) = self.file_list_parts_mut(kind);
        if index >= files.len() {
            return None;
        }

        let removed = files.remove(index);
        if files.is_empty() {
            *selected = None;
        } else if index >= files.len() {
            *selected = Some(files.len() - 1);
        } else {
            *selected = Some(index);
        }
        Some(removed)
    }

    pub fn clear_file_list_selection(&mut self, kind: FileListKind) {
        let (_, selected, _) = self.file_list_parts_mut(kind);
        *selected = None;
    }

    pub fn manual_entry_mut(&mut self, kind: FileListKind) -> &mut String {
        let (_, _, manual) = self.file_list_parts_mut(kind);
        manual
    }

    pub fn clear_manual_entry(&mut self, kind: FileListKind) {
        self.manual_entry_mut(kind).clear();
    }

    fn ensure_timestamp_strategy_default(&mut self) {
        if !matches!(self.timestamp_strategy, TimestampStrategy::None) {
            return;
        }

        let recommended = if self.source_timestamp.is_some() {
            Some(TimestampStrategy::InheritSource)
        } else if self.planned_timestamp_for_current_source().is_some() {
            Some(TimestampStrategy::SasRules)
        } else {
            Some(TimestampStrategy::Manual)
        };

        if let Some(strategy) = recommended {
            self.set_timestamp_strategy(strategy);
        }
    }

    pub fn set_folder_name_from_full(&mut self, name: &str) {
        let (prefix, remainder) = SasPrefix::split_from_name(name);
        self.selected_prefix = prefix;
        self.folder_base_name = remainder.to_string();
    }

    pub fn set_psu_file_base_from_full(&mut self, file_stem: &str) {
        let (prefix, remainder) = SasPrefix::split_from_name(file_stem);
        if prefix == SasPrefix::None || prefix == self.selected_prefix {
            self.psu_file_base_name = remainder.to_string();
        } else {
            self.psu_file_base_name = file_stem.to_string();
        }
    }

    pub fn missing_include_files(&self, folder: &Path) -> Vec<String> {
        if self.include_files.is_empty() {
            return Vec::new();
        }

        self.include_files
            .iter()
            .filter_map(|file| {
                let candidate = folder.join(file);
                if candidate.is_file() {
                    None
                } else {
                    Some(file.clone())
                }
            })
            .collect()
    }

    pub fn format_pack_error(
        &self,
        folder: &Path,
        output_path: &Path,
        err: psu_packer::Error,
    ) -> String {
        match err {
            psu_packer::Error::NameError => {
                "PSU name can only contain letters, numbers, spaces, underscores, and hyphens."
                    .to_string()
            }
            psu_packer::Error::ConfigError(message) => {
                format!("Configuration error: {message}")
            }
            psu_packer::Error::IOError(io_err) => {
                let missing_files = self.missing_include_files(folder);
                if !missing_files.is_empty() {
                    let formatted = missing_files
                        .into_iter()
                        .map(|name| format!("• {name}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    return format!(
                        "The following files referenced in the configuration are missing from {}:\n{}",
                        folder.display(),
                        formatted
                    );
                }

                match io_err.kind() {
                    io::ErrorKind::NotFound => {
                        if let Some(parent) = output_path.parent() {
                            if !parent.exists() {
                                return format!(
                                    "Cannot write the PSU file because the destination folder {} does not exist.",
                                    parent.display()
                                );
                            }
                        }
                        format!("A required file or folder could not be found: {io_err}")
                    }
                    io::ErrorKind::PermissionDenied => {
                        format!("Permission denied while accessing the file system: {io_err}")
                    }
                    _ => format!("File system error: {io_err}"),
                }
            }
        }
    }

    pub fn determine_update_destination(&self) -> Result<PathBuf, String> {
        if let Some(path) = &self.loaded_psu_path {
            return Ok(path.clone());
        }

        let trimmed = self.output.trim();
        if trimmed.is_empty() {
            Err("Load a PSU file or set the output path before updating.".to_string())
        } else {
            Ok(PathBuf::from(trimmed))
        }
    }

    pub fn determine_export_source_path(&self) -> Result<PathBuf, String> {
        if let Some(path) = &self.loaded_psu_path {
            return Ok(path.clone());
        }

        let trimmed = self.output.trim();
        if trimmed.is_empty() {
            Err("Load a PSU file or select a packed PSU before exporting its contents.".to_string())
        } else {
            Ok(PathBuf::from(trimmed))
        }
    }

    pub fn export_psu_to_folder(
        &self,
        source_path: &Path,
        destination_parent: &Path,
    ) -> Result<PathBuf, String> {
        if !source_path.is_file() {
            return Err(format!(
                "Cannot export because {} does not exist.",
                source_path.display()
            ));
        }

        let data = fs::read(source_path)
            .map_err(|err| format!("Failed to read {}: {err}", source_path.display()))?;

        let parsed = std::panic::catch_unwind(|| PSU::new(data))
            .map_err(|_| format!("Failed to parse PSU file {}", source_path.display()))?;

        let entries = parsed.entries();
        let root_name = entries
            .iter()
            .find(|entry| {
                matches!(entry.kind, PSUEntryKind::Directory)
                    && entry.name != "."
                    && entry.name != ".."
            })
            .map(|entry| entry.name.clone())
            .ok_or_else(|| format!("{} does not contain PSU metadata", source_path.display()))?;

        if root_name.trim().is_empty() {
            return Err(format!(
                "{} does not contain a valid root directory entry.",
                source_path.display()
            ));
        }

        let export_root = destination_parent.join(&root_name);
        fs::create_dir_all(&export_root)
            .map_err(|err| format!("Failed to create {}: {err}", export_root.display()))?;

        for entry in entries {
            match entry.kind {
                PSUEntryKind::Directory => {
                    if entry.name == "." || entry.name == ".." {
                        continue;
                    }

                    let target = if entry.name == root_name {
                        export_root.clone()
                    } else {
                        export_root.join(&entry.name)
                    };

                    fs::create_dir_all(&target)
                        .map_err(|err| format!("Failed to create {}: {err}", target.display()))?;
                }
                PSUEntryKind::File => {
                    let Some(contents) = entry.contents else {
                        return Err(format!(
                            "{} is missing file data in the PSU archive.",
                            entry.name
                        ));
                    };

                    let target = export_root.join(&entry.name);
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent).map_err(|err| {
                            format!("Failed to create {}: {err}", parent.display())
                        })?;
                    }

                    fs::write(&target, contents)
                        .map_err(|err| format!("Failed to write {}: {err}", target.display()))?;
                }
            }
        }

        Ok(export_root)
    }

    pub fn prepare_loaded_psu_workspace(&self) -> Result<(TempDir, PathBuf), String> {
        let source_path = self
            .loaded_psu_path
            .as_ref()
            .ok_or_else(|| "No PSU file is currently loaded.".to_string())?;
        let temp_dir =
            tempdir().map_err(|err| format!("Failed to create temporary workspace: {err}"))?;
        let export_root = self
            .export_psu_to_folder(source_path, temp_dir.path())
            .map_err(|err| format!("Failed to export loaded PSU: {err}"))?;
        Ok((temp_dir, export_root))
    }

    pub fn is_pack_running(&self) -> bool {
        self.pack_job.is_some()
    }

    pub fn start_pack_job(
        &mut self,
        folder: PathBuf,
        output_path: PathBuf,
        config: psu_packer::Config,
    ) {
        if self.pack_job.is_some() {
            return;
        }

        let progress = Arc::new(Mutex::new(PackProgress::InProgress));
        let thread_progress = Arc::clone(&progress);

        let handle = thread::spawn(move || {
            let result =
                psu_packer::pack_with_config(folder.as_path(), output_path.as_path(), config);

            let outcome = match result {
                Ok(_) => PackOutcome::Success {
                    output_path: output_path.clone(),
                },
                Err(error) => PackOutcome::Error {
                    folder: folder.clone(),
                    output_path: output_path.clone(),
                    error,
                },
            };

            let mut guard = thread_progress
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            *guard = PackProgress::Finished(outcome);
        });

        self.status = "Packing…".to_string();
        self.clear_error_message();
        self.pack_job = Some(PackJob {
            progress,
            handle: Some(handle),
        });
    }

    pub fn pack_progress_value(&self) -> Option<f32> {
        let job = self.pack_job.as_ref()?;
        let guard = job.progress.lock().ok()?;
        Some(match &*guard {
            PackProgress::InProgress => 0.0,
            PackProgress::Finished(_) => 1.0,
        })
    }

    pub fn poll_pack_job(&mut self) -> Option<PackOutcome> {
        let Some(mut job) = self.pack_job.take() else {
            return None;
        };

        let outcome = match job.progress.lock() {
            Ok(mut guard) => {
                if let PackProgress::Finished(_) = &*guard {
                    if let PackProgress::Finished(outcome) =
                        std::mem::replace(&mut *guard, PackProgress::InProgress)
                    {
                        Some(outcome)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Err(poison) => {
                let mut guard = poison.into_inner();
                if let PackProgress::Finished(_) = &*guard {
                    if let PackProgress::Finished(outcome) =
                        std::mem::replace(&mut *guard, PackProgress::InProgress)
                    {
                        Some(outcome)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        if let Some(outcome) = outcome {
            if let Some(handle) = job.handle.take() {
                let _ = handle.join();
            }

            self.temp_workspace = None;
            Some(outcome)
        } else {
            self.pack_job = Some(job);
            None
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

    pub fn choose_output_destination(&mut self) {
        self.events.push(AppEvent::ChooseOutputDestination {
            default_directory: self.opened_folder.clone(),
            default_file_name: None,
        });
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
            | Action::ChooseOutputDestination
            | Action::AddFiles
            | Action::SaveFile
            | Action::CreateMetadataTemplate(MetadataTarget::PsuToml)
            | Action::CreateMetadataTemplate(MetadataTarget::TitleCfg)
            | Action::EditMetadata(_)
            | Action::OpenEditor(_)
            | Action::Metadata(_)
            | Action::Timestamp(_)
            | Action::FileList(_)
            | Action::IconSys(_) => self.opened_folder.is_some(),
            _ => true,
        }
    }

    fn trigger_action(&mut self, action: Action) {
        match action {
            Action::OpenProject => self.open_folder(),
            Action::PackPsu => self.export_psu(),
            Action::AddFiles => self.add_files(),
            Action::SaveFile => self.save_file(),
            Action::ChooseOutputDestination => self.choose_output_destination(),
            Action::CreateMetadataTemplate(MetadataTarget::PsuToml) => self.create_psu_toml(),
            Action::CreateMetadataTemplate(MetadataTarget::TitleCfg) => self.create_title_cfg(),
            Action::OpenSettings => self.open_settings(),
            Action::OpenEditor(_)
            | Action::Metadata(_)
            | Action::Timestamp(_)
            | Action::FileList(_)
            | Action::IconSys(_) => {}
            _ => {}
        }
    }

    fn supports_action(&self, action: Action) -> bool {
        matches!(
            action,
            Action::OpenProject
                | Action::PackPsu
                | Action::ChooseOutputDestination
                | Action::AddFiles
                | Action::SaveFile
                | Action::CreateMetadataTemplate(MetadataTarget::PsuToml)
                | Action::CreateMetadataTemplate(MetadataTarget::TitleCfg)
                | Action::OpenSettings
                | Action::OpenEditor(_)
                | Action::Metadata(_)
                | Action::Timestamp(_)
                | Action::FileList(_)
                | Action::IconSys(_)
        )
    }
}
