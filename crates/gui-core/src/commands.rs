use std::path::PathBuf;

use crate::state::VirtualFile;

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
