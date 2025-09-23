use crate::AppState;
use psu_packer::{self, Config as PsuConfig};
use std::path::PathBuf;

#[derive(Debug)]
pub enum ExportError {
    NoFolderSelected,
    PackError {
        folder: PathBuf,
        output: PathBuf,
        source: psu_packer::Error,
    },
}

pub fn export_psu(state: &AppState) -> Result<(), ExportError> {
    let folder = state
        .opened_folder
        .clone()
        .ok_or(ExportError::NoFolderSelected)?;

    let folder_name = folder
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| folder.to_string_lossy().into_owned());

    let config = config_from_state(state, folder_name.clone());
    let default_name = format!("{folder_name}.psu");

    if let Some(mut output_path) = rfd::FileDialog::new()
        .set_file_name(&default_name)
        .set_directory(&folder)
        .save_file()
    {
        enforce_psu_extension(&mut output_path);

        psu_packer::pack_with_config(&folder, &output_path, config).map_err(|source| {
            ExportError::PackError {
                folder: folder.clone(),
                output: output_path.clone(),
                source,
            }
        })?;
    }

    Ok(())
}

fn config_from_state(state: &AppState, name: String) -> PsuConfig {
    let include = Some(
        state
            .files
            .iter()
            .map(|file| file.name.clone())
            .collect::<Vec<_>>(),
    );

    PsuConfig {
        name,
        timestamp: None,
        include,
        exclude: None,
        icon_sys: None,
    }
}

fn enforce_psu_extension(path: &mut PathBuf) {
    let has_psu_extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("psu"))
        .unwrap_or(false);

    if !has_psu_extension {
        path.set_extension("psu");
    }
}
