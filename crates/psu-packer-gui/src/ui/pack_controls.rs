use eframe::egui;

use crate::{
    ui::{project_requirements_checklist, theme},
    PackerApp, SasPrefix, REQUIRED_PROJECT_FILES,
};
use gui_core::{
    actions::{self, Action, ActionDescriptor, FileListAction, FileListKind, MetadataAction},
    ActionDispatcher,
};

pub(crate) fn metadata_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.set_width(ui.available_width());
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Metadata"));
        ui.small("Edit PSU metadata before or after selecting a folder.");

        egui::Grid::new("metadata_grid")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 6.0))
            .show(ui, |ui| {
                ui.label("PREFIX CATEGORY");
                let mut selected_prefix = app.packer_state.selected_prefix;
                let prefix_changed = egui::ComboBox::from_id_source("metadata_prefix_combo")
                    .selected_text(selected_prefix.label())
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        for prefix in SasPrefix::iter_with_unprefixed() {
                            let response =
                                ui.selectable_value(&mut selected_prefix, prefix, prefix.label());
                            if response.changed() {
                                changed = true;
                            }
                        }
                        changed
                    })
                    .inner
                    .unwrap_or(false);
                if prefix_changed && selected_prefix != app.packer_state.selected_prefix {
                    app.trigger_action(Action::Metadata(MetadataAction::SelectPrefix(
                        selected_prefix,
                    )));
                }
                ui.end_row();

                let folder_preview = app.packer_state.folder_name();
                ui.vertical(|ui| {
                    ui.label("psuPaste Folder Name");
                    let trimmed = folder_preview.trim();
                    if trimmed.is_empty() {
                        ui.small("Used when exporting to psuPaste folders.");
                    } else {
                        ui.small(format!("Creates folder: {folder_preview}"));
                    }
                });
                let mut folder_base_name = app.packer_state.folder_base_name.clone();
                if ui.text_edit_singleline(&mut folder_base_name).changed() {
                    app.trigger_action(Action::Metadata(MetadataAction::SetFolderBaseName(
                        folder_base_name,
                    )));
                }
                ui.end_row();

                let output_preview = app.packer_state.default_output_file_name();
                ui.vertical(|ui| {
                    ui.label("PSU filename");
                    match &output_preview {
                        Some(file_name) => {
                            ui.small(format!("Saves archive as: {file_name}"));
                        }
                        None => {
                            ui.small("Base name updates the .psu archive.");
                        }
                    }
                });
                let mut psu_base = app.packer_state.psu_file_base_name.clone();
                let psu_response = ui
                    .horizontal(|ui| {
                        let response = ui.text_edit_singleline(&mut psu_base);
                        ui.monospace(".psu");
                        response
                    })
                    .inner;
                if psu_response.changed() {
                    app.trigger_action(Action::Metadata(MetadataAction::SetPsuFileBaseName(
                        psu_base,
                    )));
                }
                ui.end_row();

                ui.label("Timestamp");
                crate::ui::timestamps::metadata_timestamp_section(app, ui);
                ui.end_row();

                ui.label("icon.sys");
                let mut label = "Configure icon.sys metadata in the dedicated tab.".to_string();
                if app.icon_sys_enabled {
                    if app.icon_sys_use_existing {
                        label.push_str(" Existing icon.sys file will be reused.");
                    } else {
                        label.push_str(" A new icon.sys will be generated.");
                    }
                }
                ui.small(label);
                ui.end_row();
            });
        #[cfg(feature = "psu-toml-editor")]
        if app.packer_state.folder.is_some() && app.psu_toml_sync_blocked {
            ui.add_space(6.0);
            ui.colored_label(
                egui::Color32::YELLOW,
                "psu.toml has manual edits; automatic metadata syncing is paused.",
            );
        }
    });
}

pub(crate) fn file_filters_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.set_width(ui.available_width());
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "File filters"));
        ui.small("Manage which files to include or exclude before creating the archive.");
        let folder_selected = app.packer_state.folder.is_some();
        if !folder_selected {
            ui.small("No folder selected. Enter file names manually or choose a folder to browse.");
        }
        ui.columns(2, |columns| {
            file_list_ui(app, &mut columns[0], ListKind::Include);
            file_list_ui(app, &mut columns[1], ListKind::Exclude);
        });
    });
}

pub(crate) fn output_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.set_width(ui.available_width());
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Output"));
        ui.small("Choose where the packed PSU file will be saved.");
        let choose_destination_descriptor =
            ActionDescriptor::new(Action::ChooseOutputDestination, "Choose destination");
        actions::handle_shortcuts(ui.ctx(), app, &[choose_destination_descriptor.clone()]);
        egui::Grid::new("output_grid")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 6.0))
            .show(ui, |ui| {
                ui.label("Packed PSU path");
                let trimmed_output = app.packer_state.output.trim();
                if trimmed_output.is_empty() {
                    ui.weak("No destination selected");
                } else {
                    ui.label(egui::RichText::new(trimmed_output).monospace());
                }
                ui.end_row();

                ui.label("");
                actions::action_button(ui, app, &choose_destination_descriptor)
                    .on_hover_text("Pick where the PSU file will be created or updated.");
                ui.end_row();
            });
    });
}

pub(crate) fn packaging_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.set_width(ui.available_width());
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Packaging"));
        ui.small("Validate the configuration and generate the PSU archive.");
        let pack_in_progress = app.is_pack_running();
        let missing_requirements = !app.packer_state.missing_required_project_files.is_empty();
        let missing_summary = if missing_requirements {
            Some(
                app.packer_state
                    .missing_required_project_files
                    .iter()
                    .map(|entry| entry.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        } else {
            None
        };
        let requirement_statuses = app.packer_state.project_requirement_statuses();
        let required_asset_list = REQUIRED_PROJECT_FILES.join(", ");

        if let Some(ref statuses) = requirement_statuses {
            if missing_requirements {
                let details = missing_summary
                    .as_ref()
                    .filter(|summary| !summary.trim().is_empty())
                    .cloned()
                    .unwrap_or_else(|| required_asset_list.clone());
                ui.colored_label(
                    egui::Color32::YELLOW,
                    format!("Add the missing project assets before packing: {details}."),
                );
            } else {
                ui.weak(format!(
                    "All required project assets detected ({required_asset_list})."
                ));
            }
            ui.add_space(4.0);
            project_requirements_checklist(ui, statuses);
        } else {
            ui.weak("Select a project folder to verify the required assets.");
        }
        let pack_descriptor = ActionDescriptor::new(Action::PackPsu, "Pack PSU");
        let update_descriptor = ActionDescriptor::new(Action::UpdatePsu, "Update PSU");
        let export_descriptor =
            ActionDescriptor::new(Action::ExportPsuToFolder, "Save as Folder with contents");
        actions::handle_shortcuts(
            ui.ctx(),
            app,
            &[
                pack_descriptor.clone(),
                update_descriptor.clone(),
                export_descriptor.clone(),
            ],
        );
        ui.horizontal_wrapped(|ui| {
            let pack_response = actions::action_button(ui, app, &pack_descriptor);
            if pack_in_progress {
                pack_response.on_hover_text("Packing in progress…");
            } else if missing_requirements {
                let details = missing_summary
                    .as_ref()
                    .filter(|summary| !summary.trim().is_empty())
                    .cloned()
                    .unwrap_or_else(|| required_asset_list.clone());
                pack_response.on_hover_text(format!(
                    "Add the missing project assets before packing: {details}."
                ));
            } else {
                pack_response.on_hover_text("Create the PSU archive using the settings above.");
            }

            let update_response = actions::action_button(ui, app, &update_descriptor);
            if pack_in_progress {
                update_response.on_hover_text("Packing in progress…");
            } else if missing_requirements {
                let details = missing_summary
                    .as_ref()
                    .filter(|summary| !summary.trim().is_empty())
                    .cloned()
                    .unwrap_or_else(|| required_asset_list.clone());
                update_response.on_hover_text(format!(
                    "Add the missing project assets before updating: {details}."
                ));
            } else {
                update_response
                    .on_hover_text("Repack the current project into the existing PSU file.");
            }

            let export_response = actions::action_button(ui, app, &export_descriptor);
            if pack_in_progress {
                export_response.on_hover_text("Packing in progress…");
            } else if missing_requirements {
                let details = missing_summary
                    .as_ref()
                    .filter(|summary| !summary.trim().is_empty())
                    .cloned()
                    .unwrap_or_else(|| required_asset_list.clone());
                export_response.on_hover_text(format!(
                    "Add the missing project assets before exporting: {details}."
                ));
            } else {
                export_response
                    .on_hover_text("Export the contents of the current PSU archive to a folder.");
            }
        });

        if pack_in_progress {
            ui.label("Packing in progress…");
        }

        if let Some(error) = &app.packer_state.error_message {
            ui.colored_label(egui::Color32::RED, error);
        }
        if !app.packer_state.status.is_empty() {
            ui.label(&app.packer_state.status);
        }
    });
}

#[derive(Copy, Clone)]
pub(crate) enum ListKind {
    Include,
    Exclude,
}

impl ListKind {
    fn label(self) -> &'static str {
        match self {
            ListKind::Include => "Include files",
            ListKind::Exclude => "Exclude files",
        }
    }

    fn as_file_list_kind(self) -> FileListKind {
        match self {
            ListKind::Include => FileListKind::Include,
            ListKind::Exclude => FileListKind::Exclude,
        }
    }
}

impl From<FileListKind> for ListKind {
    fn from(kind: FileListKind) -> Self {
        match kind {
            FileListKind::Include => ListKind::Include,
            FileListKind::Exclude => ListKind::Exclude,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use gui_core::actions::{
        Action, ActionDescriptor, FileListAction, FileListKind, IconSysAction, TimestampAction,
        TimestampStrategyAction,
    };
    use ps2_filetypes::sjis;
    use std::path::PathBuf;

    fn dispatch_action(app: &mut PackerApp, descriptor: &ActionDescriptor) {
        let action = descriptor.action.clone();
        assert!(app.supports_action(action.clone()));
        assert!(app.is_action_enabled(action.clone()));
        app.trigger_action(action);
    }

    fn set_manual_entry(app: &mut PackerApp, kind: FileListKind, value: &str) {
        let manual_entry = app.packer_state_mut().manual_entry_mut(kind);
        manual_entry.clear();
        manual_entry.push_str(value);
    }

    fn app_with_prefix(prefix: SasPrefix) -> PackerApp {
        let mut app = PackerApp::default();
        app.packer_state.set_folder_base_name("SAVE".to_string());
        app.packer_state.set_psu_file_base_name("SAVE".to_string());
        app.packer_state.set_selected_prefix(prefix);
        app
    }

    #[test]
    fn config_from_state_appends_psu_toml_once() {
        let base_app = app_with_prefix(SasPrefix::App);
        let config = base_app.build_config().expect("configuration should build");
        assert_eq!(config.exclude, Some(vec!["psu.toml".to_string()]));
        assert!(
            base_app.packer_state.exclude_files.is_empty(),
            "building the configuration should not modify the exclude list"
        );

        let manual_add_exclude = ActionDescriptor::new(
            Action::FileList(FileListAction::ManualAdd(FileListKind::Exclude)),
            "Add",
        );

        let mut manual_entry_app = app_with_prefix(SasPrefix::App);
        set_manual_entry(&mut manual_entry_app, FileListKind::Exclude, "DATA.BIN");
        dispatch_action(&mut manual_entry_app, &manual_add_exclude);
        let config_with_manual_entry = manual_entry_app
            .build_config()
            .expect("configuration should include manual exclude");
        assert_eq!(
            config_with_manual_entry.exclude,
            Some(vec!["DATA.BIN".to_string(), "psu.toml".to_string()])
        );

        let mut duplicate_app = app_with_prefix(SasPrefix::App);
        set_manual_entry(&mut duplicate_app, FileListKind::Exclude, "psu.toml");
        dispatch_action(&mut duplicate_app, &manual_add_exclude);
        let config_with_duplicate = duplicate_app
            .build_config()
            .expect("configuration should handle duplicate entries");
        assert_eq!(
            config_with_duplicate.exclude,
            Some(vec!["psu.toml".to_string()])
        );
    }

    #[test]
    fn build_config_uses_loaded_psu_edits() {
        let mut app = app_with_prefix(SasPrefix::Emu);
        app.packer_state.loaded_psu_path = Some(PathBuf::from("input.psu"));
        let timestamp = NaiveDate::from_ymd_opt(2023, 11, 14)
            .and_then(|date| date.and_hms_opt(12, 34, 56))
            .expect("valid timestamp");
        app.packer_state.set_manual_timestamp(Some(timestamp));

        let select_manual_strategy = ActionDescriptor::new(
            Action::Timestamp(TimestampAction::SelectStrategy(
                TimestampStrategyAction::Manual,
            )),
            "Manual",
        );
        dispatch_action(&mut app, &select_manual_strategy);

        let include_manual_add = ActionDescriptor::new(
            Action::FileList(FileListAction::ManualAdd(FileListKind::Include)),
            "Add include",
        );
        let exclude_manual_add = ActionDescriptor::new(
            Action::FileList(FileListAction::ManualAdd(FileListKind::Exclude)),
            "Add exclude",
        );

        set_manual_entry(&mut app, FileListKind::Include, "FILE.BIN");
        dispatch_action(&mut app, &include_manual_add);
        set_manual_entry(&mut app, FileListKind::Exclude, "SKIP.DAT");
        dispatch_action(&mut app, &exclude_manual_add);

        let config = app.build_config().expect("config builds successfully");
        assert_eq!(config.name, "EMU_SAVE");
        assert_eq!(config.timestamp, Some(timestamp));
        assert_eq!(config.include, Some(vec!["FILE.BIN".to_string()]));
        assert_eq!(
            config.exclude,
            Some(vec!["SKIP.DAT".to_string(), "psu.toml".to_string()])
        );
    }

    #[test]
    fn manual_filter_entries_allowed_without_folder() {
        let mut app = app_with_prefix(SasPrefix::App);
        let include_manual_add = ActionDescriptor::new(
            Action::FileList(FileListAction::ManualAdd(FileListKind::Include)),
            "Add include",
        );
        let exclude_manual_add = ActionDescriptor::new(
            Action::FileList(FileListAction::ManualAdd(FileListKind::Exclude)),
            "Add exclude",
        );

        set_manual_entry(&mut app, FileListKind::Include, "BOOT.ELF");
        dispatch_action(&mut app, &include_manual_add);
        set_manual_entry(&mut app, FileListKind::Exclude, "THUMBS.DB");
        dispatch_action(&mut app, &exclude_manual_add);

        let config = app.build_config().expect("config builds successfully");
        assert_eq!(config.include, Some(vec!["BOOT.ELF".to_string()]));
        assert_eq!(
            config.exclude,
            Some(vec!["THUMBS.DB".to_string(), "psu.toml".to_string()])
        );
    }

    #[test]
    fn manual_filter_entries_trim_and_reject_duplicates() {
        let mut app = app_with_prefix(SasPrefix::App);
        let include_manual_add = ActionDescriptor::new(
            Action::FileList(FileListAction::ManualAdd(FileListKind::Include)),
            "Add include",
        );

        set_manual_entry(&mut app, FileListKind::Include, "  DATA.BIN  ");
        dispatch_action(&mut app, &include_manual_add);
        assert_eq!(app.packer_state.include_files, vec!["DATA.BIN"]);

        let initial_len = app.packer_state.include_files.len();
        set_manual_entry(&mut app, FileListKind::Include, "DATA.BIN");
        dispatch_action(&mut app, &include_manual_add);
        assert_eq!(app.packer_state.include_files.len(), initial_len);
        assert_eq!(app.packer_state.include_files, vec!["DATA.BIN"]);
        assert!(app.packer_state.error_message.is_some());
    }

    #[test]
    fn config_from_state_uses_shift_jis_byte_linebreaks() {
        let mut app = app_with_prefix(SasPrefix::App);
        let enable_descriptor =
            ActionDescriptor::new(Action::IconSys(IconSysAction::Enable), "Enable");
        dispatch_action(&mut app, &enable_descriptor);

        app.icon_sys_title_line1 = "メモ".to_string();
        app.icon_sys_title_line2 = "リーカード".to_string();

        let config = app.build_config().expect("configuration should build");
        let icon_sys = config.icon_sys.expect("icon_sys configuration present");
        let expected_break = sjis::encode_sjis(&app.icon_sys_title_line1).unwrap().len() as u16;

        assert_eq!(icon_sys.linebreak_pos, Some(expected_break));
    }
}

fn file_list_ui(app: &mut PackerApp, ui: &mut egui::Ui, kind: ListKind) {
    let file_list_kind = kind.as_file_list_kind();
    let label = kind.label();

    let browse_descriptor = ActionDescriptor::new(
        Action::FileList(FileListAction::Browse(file_list_kind)),
        "📁",
    );
    let manual_add_descriptor = ActionDescriptor::new(
        Action::FileList(FileListAction::ManualAdd(file_list_kind)),
        "Add",
    );
    let remove_descriptor = ActionDescriptor::new(
        Action::FileList(FileListAction::RemoveSelected(file_list_kind)),
        "➖",
    );

    let shortcut_descriptors = [
        browse_descriptor.clone(),
        manual_add_descriptor.clone(),
        remove_descriptor.clone(),
    ];
    actions::handle_shortcuts(ui.ctx(), app, &shortcut_descriptors);

    ui.horizontal(|ui| {
        ui.label(label);
        ui.add_space(ui.spacing().item_spacing.x);

        actions::action_button(ui, app, &browse_descriptor)
            .on_hover_text("Browse for files in the selected folder.");

        actions::action_button(ui, app, &remove_descriptor)
            .on_hover_text("Remove the selected file from this list.");
    });

    ui.horizontal(|ui| {
        let response = {
            let manual_entry = app.packer_state_mut().manual_entry_mut(file_list_kind);
            ui.add(egui::TextEdit::singleline(manual_entry).hint_text("Add file by name"))
        };

        actions::action_button(ui, app, &manual_add_descriptor)
            .on_hover_text("Add the typed entry to this list.");

        let enter_pressed =
            response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if enter_pressed {
            let action = manual_add_descriptor.action.clone();
            if app.is_action_enabled(action.clone()) {
                app.trigger_action(action);
            }
        }
    });

    let files = app
        .packer_state()
        .file_list_entries(file_list_kind)
        .to_vec();
    let selected_index = app.packer_state().file_list_selection(file_list_kind);

    egui::ScrollArea::vertical()
        .max_height(150.0)
        .show(ui, |ui| {
            for (idx, file) in files.iter().enumerate() {
                ui.horizontal(|ui| {
                    let is_selected = Some(idx) == selected_index;
                    if ui.selectable_label(is_selected, file).clicked() {
                        app.trigger_action(Action::FileList(FileListAction::SelectEntry(
                            file_list_kind,
                            Some(idx),
                        )));
                    }

                    ui.add_space(ui.spacing().item_spacing.x);

                    let response = ui
                        .small_button("✖")
                        .on_hover_text("Remove this file from the list.");
                    if response.clicked() {
                        app.trigger_action(Action::FileList(FileListAction::SelectEntry(
                            file_list_kind,
                            Some(idx),
                        )));
                        app.trigger_action(remove_descriptor.action.clone());
                    }
                });
            }
        });
}
