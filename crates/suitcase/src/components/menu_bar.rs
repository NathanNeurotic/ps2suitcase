use crate::components::menu_item::MenuItemComponent;
use crate::data::state::AppState;
use eframe::egui;
use eframe::egui::{menu, Context, KeyboardShortcut, Modifiers, Ui};
use gui_core::actions::{self, Action, ActionDescriptor, MetadataTarget};

const CTRL_OR_CMD: Modifiers = if cfg!(target_os = "macos") {
    Modifiers::MAC_CMD
} else {
    Modifiers::CTRL
};
const CTRL_OR_CMD_SHIFT: Modifiers = if cfg!(target_os = "macos") {
    Modifiers {
        alt: false,
        ctrl: false,
        shift: true,
        mac_cmd: true,
        command: false,
    }
} else {
    Modifiers {
        alt: false,
        ctrl: true,
        shift: true,
        mac_cmd: false,
        command: false,
    }
};

pub const OPEN_FOLDER_KEYBOARD_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(CTRL_OR_CMD, egui::Key::O);
const EXPORT_KEYBOARD_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(CTRL_OR_CMD_SHIFT, egui::Key::S);
const ADD_FILE_KEYBOARD_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(CTRL_OR_CMD, egui::Key::N);
const SAVE_KEYBOARD_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(CTRL_OR_CMD, egui::Key::S);
const OPEN_SETTINGS_KEYBOARD_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(CTRL_OR_CMD, egui::Key::Comma);

pub fn menu_bar(ui: &mut Ui, app: &mut AppState) {
    menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            let open_folder = open_folder_action_descriptor();
            actions::action_button(ui, app, &open_folder);

            let add_files = add_files_action_descriptor();
            actions::action_button(ui, app, &add_files);

            let save_file = save_file_action_descriptor();
            actions::action_button(ui, app, &save_file);

            let create_psu = create_psu_template_action_descriptor();
            actions::action_button(ui, app, &create_psu);

            let create_title = create_title_template_action_descriptor();
            actions::action_button(ui, app, &create_title);
        });
        ui.menu_button("Edit", |ui| {
            let open_settings = open_settings_action_descriptor();
            actions::action_button(ui, app, &open_settings);
        });
        ui.menu_button("Export", |ui| {
            let export_psu = export_psu_action_descriptor();
            actions::action_button(ui, app, &export_psu);
        });
        ui.menu_button("Help", |ui| {
            ui.menu_item_link("GitHub", "https://github.com/techwritescode/ps2-rust")
        })
    });
}

pub fn handle_accelerators(ctx: &Context, app: &mut AppState) {
    let descriptors = [
        open_folder_action_descriptor(),
        export_psu_action_descriptor(),
        save_file_action_descriptor(),
        add_files_action_descriptor(),
        open_settings_action_descriptor(),
    ];

    actions::handle_shortcuts(ctx, app, &descriptors);
}

fn open_folder_action_descriptor() -> ActionDescriptor {
    ActionDescriptor::new(Action::OpenProject, "Open Folder")
        .with_shortcut(OPEN_FOLDER_KEYBOARD_SHORTCUT)
}

fn add_files_action_descriptor() -> ActionDescriptor {
    ActionDescriptor::new(Action::AddFiles, "Add Files").with_shortcut(ADD_FILE_KEYBOARD_SHORTCUT)
}

fn save_file_action_descriptor() -> ActionDescriptor {
    ActionDescriptor::new(Action::SaveFile, "Save File").with_shortcut(SAVE_KEYBOARD_SHORTCUT)
}

fn create_psu_template_action_descriptor() -> ActionDescriptor {
    ActionDescriptor::new(
        Action::CreateMetadataTemplate(MetadataTarget::PsuToml),
        "Create psu.toml from template",
    )
}

fn create_title_template_action_descriptor() -> ActionDescriptor {
    ActionDescriptor::new(
        Action::CreateMetadataTemplate(MetadataTarget::TitleCfg),
        "Create title.cfg from template",
    )
}

fn export_psu_action_descriptor() -> ActionDescriptor {
    ActionDescriptor::new(Action::PackPsu, "Export PSU").with_shortcut(EXPORT_KEYBOARD_SHORTCUT)
}

fn open_settings_action_descriptor() -> ActionDescriptor {
    ActionDescriptor::new(Action::OpenSettings, "Settings")
        .with_shortcut(OPEN_SETTINGS_KEYBOARD_SHORTCUT)
}
