use crate::data::files::Files;
use crate::data::virtual_file::VirtualFile;
use gui_core::actions::{Action, ActionDispatcher, MetadataTarget};
use std::path::PathBuf;

#[derive(Clone)]
pub enum AppEvent {
    OpenFolder,
    OpenFile(VirtualFile),
    SetTitle(String),
    AddFiles,
    ExportPSU,
    SaveFile,
    OpenSave,
    CreateICN,
    CreatePsuToml,
    CreateTitleCfg,
    OpenSettings,
    StartPCSX2,
    StartPCSX2Elf(PathBuf),
    Validate,
}

pub struct AppState {
    pub opened_folder: Option<PathBuf>,
    pub files: Files,
    pub events: Vec<AppEvent>,
    pub pcsx2_path: String,
}

impl AppState {}

impl AppState {
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
        match action {
            Action::OpenProject
            | Action::PackPsu
            | Action::AddFiles
            | Action::SaveFile
            | Action::CreateMetadataTemplate(MetadataTarget::PsuToml)
            | Action::CreateMetadataTemplate(MetadataTarget::TitleCfg)
            | Action::OpenSettings => true,
            _ => false,
        }
    }
}
