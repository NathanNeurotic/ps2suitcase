use std::path::PathBuf;

use crate::actions::{FileListKind, MetadataTarget};
use crate::state::{MissingRequiredFile, VirtualFile};

/// High-level side effects requested by the GUI state machine.
///
/// Each variant corresponds to a platform specific operation that cannot be performed inside the
/// pure state layer. Downstream handlers should match on the event, perform the requested side
/// effect, and feed any results back into the state through the relevant APIs.
#[derive(Clone)]
pub enum AppEvent {
    /// Prompt the user to select the root project folder.
    OpenFolder,
    /// Open the given virtual file in an editor or viewer.
    ///
    /// The `VirtualFile` payload contains the file name, absolute path, and size so the handler can
    /// decide how to present the contents.
    OpenFile(VirtualFile),
    /// Update the native window title to the provided value.
    SetTitle(String),
    /// Present a file picker that allows the user to add arbitrary files to the project.
    AddFiles,
    /// Start the PSU export flow using the currently selected project state.
    ExportPSU,
    /// Save the current project to its existing location without prompting the user.
    SaveFile,
    /// Display a dialog that lets the user choose a PSU file to open.
    OpenSave,
    /// Show the UI for creating ICN assets.
    CreateICN,
    /// Create the `psu.toml` metadata file from its template or load the template into the editor.
    CreatePsuToml,
    /// Create the `title.cfg` metadata file from its template or load the template into the editor.
    CreateTitleCfg,
    /// Open the application settings view.
    OpenSettings,
    /// Launch PCSX2 using the configured executable path.
    StartPCSX2,
    /// Launch PCSX2 with the provided ELF path.
    StartPCSX2Elf(PathBuf),
    /// Run validation against the currently selected project folder.
    Validate,
    /// Request a "save as" dialog for the packed PSU output.
    ///
    /// * `default_directory` - Optional directory that should be pre-selected when the dialog is
    ///   opened.
    /// * `default_file_name` - Optional file name suggestion that should populate the save dialog.
    ChooseOutputDestination {
        default_directory: Option<PathBuf>,
        default_file_name: Option<String>,
    },
    /// Request a multi-select file dialog for include/exclude lists.
    ///
    /// * `project_root` - Folder the selections must reside in. The dialog should restrict
    ///   navigation to this directory so the returned files remain relative to the project.
    /// * `kind` - Indicates whether the chosen files should be appended to the include or exclude
    ///   list.
    BrowseFileListEntries {
        project_root: PathBuf,
        kind: FileListKind,
    },
    /// Prompt the user for the destination folder when exporting PSU contents to disk.
    ///
    /// * `default_directory` - Optional directory to open the dialog in.
    ChooseExportFolder { default_directory: Option<PathBuf> },
    /// Create or load a metadata template for the requested target.
    ///
    /// * `target` - Identifies which metadata file should be produced.
    /// * `template` - The textual contents that should be written to disk or loaded into the
    ///   editor.
    /// * `destination` - Optional folder where the template should be created. When `None`, the
    ///   handler should keep the template in-memory and display it in the appropriate editor.
    CreateMetadataTemplate {
        target: MetadataTarget,
        template: String,
        destination: Option<PathBuf>,
    },
    /// Ask the user to confirm packing despite missing required files.
    ///
    /// The `missing_required_files` payload lists the files that were not found so the UI can show a
    /// meaningful warning. If the user confirms, the handler should call
    /// [`crate::state::PackerState::confirm_pending_pack_action`] to retrieve the prepared pack
    /// inputs.
    ShowPackConfirmation {
        missing_required_files: Vec<MissingRequiredFile>,
    },
}
