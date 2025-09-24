use chrono::NaiveDateTime;
use egui::{Button, Context, KeyboardShortcut, Ui, WidgetText};

use crate::state::SasPrefix;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MetadataTarget {
    PsuToml,
    TitleCfg,
    IconSys,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditorAction {
    PsuSettings,
    PsuToml,
    TitleCfg,
    IconSys,
    TimestampAutomation,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MetadataAction {
    ResetFields,
    SelectPrefix(SasPrefix),
    SetFolderBaseName(String),
    SetPsuFileBaseName(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimestampStrategyAction {
    None,
    InheritSource,
    SasRules,
    Manual,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TimestampRulesAction {
    SetSecondsBetweenItems(u32),
    SetSlotsPerCategory(u32),
    MoveCategoryUp(usize),
    MoveCategoryDown(usize),
    SetAliasSelected {
        category_index: usize,
        alias: String,
        selected: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TimestampAction {
    SelectStrategy(TimestampStrategyAction),
    RefreshFromStrategy,
    SyncAfterSourceUpdate,
    ApplyPlannedTimestamp,
    ResetRulesToDefault,
    SetManualTimestamp(Option<NaiveDateTime>),
    Rules(TimestampRulesAction),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileListKind {
    Include,
    Exclude,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileListAction {
    Browse(FileListKind),
    ManualAdd(FileListKind),
    RemoveSelected(FileListKind),
    SelectEntry(FileListKind, Option<usize>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum IconSysAction {
    Enable,
    Disable,
    UseExisting,
    GenerateNew,
    ClearPreset,
    ResetFields,
    ApplyPreset(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    OpenProject,
    SelectProjectFolder,
    PackPsu,
    UpdatePsu,
    ExportPsuToFolder,
    ChooseOutputDestination,
    AddFiles,
    SaveFile,
    EditMetadata(MetadataTarget),
    CreateMetadataTemplate(MetadataTarget),
    OpenSettings,
    OpenEditor(EditorAction),
    ZoomIn,
    ZoomOut,
    ResetZoom,
    ConfirmPack,
    CancelPack,
    ShowExitConfirmation,
    ConfirmExit,
    CancelExit,
    Metadata(MetadataAction),
    Timestamp(TimestampAction),
    FileList(FileListAction),
    IconSys(IconSysAction),
}

pub trait ActionDispatcher {
    fn is_action_enabled(&self, action: Action) -> bool;
    fn trigger_action(&mut self, action: Action);
    fn supports_action(&self, _action: Action) -> bool {
        true
    }
}

#[derive(Clone)]
pub struct ActionDescriptor {
    pub action: Action,
    pub label: WidgetText,
    pub shortcut: Option<KeyboardShortcut>,
}

impl ActionDescriptor {
    pub fn new(action: Action, label: impl Into<WidgetText>) -> Self {
        Self {
            action,
            label: label.into(),
            shortcut: None,
        }
    }

    pub fn with_shortcut(mut self, shortcut: KeyboardShortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }
}

pub fn action_button(
    ui: &mut Ui,
    dispatcher: &mut impl ActionDispatcher,
    descriptor: &ActionDescriptor,
) -> egui::Response {
    let action = descriptor.action.clone();
    debug_assert!(dispatcher.supports_action(action.clone()));

    let mut button = Button::new(descriptor.label.clone());
    if let Some(shortcut) = descriptor.shortcut {
        button = button.shortcut_text(ui.ctx().format_shortcut(&shortcut));
    }

    let enabled = dispatcher.is_action_enabled(action.clone());
    let response = ui.add_enabled(enabled, button);
    if response.clicked() {
        dispatcher.trigger_action(action);
        ui.close_menu();
    }
    response
}

pub fn handle_shortcut(
    ctx: &Context,
    dispatcher: &mut impl ActionDispatcher,
    descriptor: &ActionDescriptor,
) -> bool {
    let action = descriptor.action.clone();
    if !dispatcher.supports_action(action.clone()) {
        return false;
    }
    if let Some(shortcut) = descriptor.shortcut {
        if ctx.input_mut(|input| input.consume_shortcut(&shortcut))
            && dispatcher.is_action_enabled(action.clone())
        {
            dispatcher.trigger_action(action);
            return true;
        }
    }
    false
}

pub fn handle_shortcuts(
    ctx: &Context,
    dispatcher: &mut impl ActionDispatcher,
    descriptors: &[ActionDescriptor],
) {
    for descriptor in descriptors {
        if handle_shortcut(ctx, dispatcher, descriptor) {
            break;
        }
    }
}
