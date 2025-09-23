use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use crate::MissingRequiredFile;

pub(crate) struct PackJob {
    pub(crate) progress: Arc<Mutex<PackProgress>>,
    pub(crate) handle: Option<thread::JoinHandle<()>>,
}

pub(crate) enum PackProgress {
    InProgress,
    Finished(PackOutcome),
}

pub(crate) struct PackPreparation {
    pub(crate) folder: PathBuf,
    pub(crate) config: psu_packer::Config,
    pub(crate) missing_required_files: Vec<MissingRequiredFile>,
}

pub(crate) enum PackOutcome {
    Success {
        output_path: PathBuf,
    },
    Error {
        folder: PathBuf,
        output_path: PathBuf,
        error: psu_packer::Error,
    },
}

pub(crate) enum PendingPackAction {
    Pack {
        folder: PathBuf,
        output_path: PathBuf,
        config: psu_packer::Config,
        missing_required_files: Vec<MissingRequiredFile>,
    },
}

impl PendingPackAction {
    pub(crate) fn missing_files(&self) -> &[MissingRequiredFile] {
        match self {
            PendingPackAction::Pack {
                missing_required_files,
                ..
            } => missing_required_files,
        }
    }
}
