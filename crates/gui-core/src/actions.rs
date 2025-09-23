use egui::{Button, Context, KeyboardShortcut, Ui, WidgetText};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MetadataTarget {
    PsuToml,
    TitleCfg,
    IconSys,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    OpenProject,
    PackPsu,
    ChooseOutputDestination,
    AddFiles,
    SaveFile,
    EditMetadata(MetadataTarget),
    CreateMetadataTemplate(MetadataTarget),
    OpenSettings,
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
    debug_assert!(dispatcher.supports_action(descriptor.action));

    let mut button = Button::new(descriptor.label.clone());
    if let Some(shortcut) = descriptor.shortcut {
        button = button.shortcut_text(ui.ctx().format_shortcut(&shortcut));
    }

    let enabled = dispatcher.is_action_enabled(descriptor.action);
    let response = ui.add_enabled(enabled, button);
    if response.clicked() {
        dispatcher.trigger_action(descriptor.action);
        ui.close_menu();
    }
    response
}

pub fn handle_shortcut(
    ctx: &Context,
    dispatcher: &mut impl ActionDispatcher,
    descriptor: &ActionDescriptor,
) -> bool {
    if !dispatcher.supports_action(descriptor.action) {
        return false;
    }
    if let Some(shortcut) = descriptor.shortcut {
        if ctx.input_mut(|input| input.consume_shortcut(&shortcut))
            && dispatcher.is_action_enabled(descriptor.action)
        {
            dispatcher.trigger_action(descriptor.action);
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
