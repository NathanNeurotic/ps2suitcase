use std::borrow::Cow;

use egui::{self};

use crate::actions::{Action, ActionDispatcher};

#[derive(Clone)]
pub struct ShellStyle {
    pub nav_width: f32,
    pub navigation_panel_frame: egui::Frame,
    pub navigation_item_frame: egui::Frame,
    pub navigation_selected_item_frame: egui::Frame,
    pub navigation_text: egui::Color32,
    pub navigation_selected_text: egui::Color32,
    pub navigation_alert_text: egui::Color32,
    pub content_frame: egui::Frame,
    pub context_panel_frame: egui::Frame,
}

impl Default for ShellStyle {
    fn default() -> Self {
        Self {
            nav_width: 220.0,
            navigation_panel_frame: egui::Frame::default(),
            navigation_item_frame: egui::Frame::default(),
            navigation_selected_item_frame: egui::Frame::default(),
            navigation_text: egui::Color32::WHITE,
            navigation_selected_text: egui::Color32::BLACK,
            navigation_alert_text: egui::Color32::RED,
            content_frame: egui::Frame::default(),
            context_panel_frame: egui::Frame::default(),
        }
    }
}

pub struct NavigationItem<'a, Intent> {
    pub id: Cow<'a, str>,
    pub label: Cow<'a, str>,
    pub description: Option<Cow<'a, str>>,
    pub action: Action,
    pub intent: Intent,
    pub enabled: bool,
    pub alert: bool,
}

impl<'a, Intent> NavigationItem<'a, Intent> {
    pub fn new(
        id: impl Into<Cow<'a, str>>,
        label: impl Into<Cow<'a, str>>,
        intent: Intent,
        action: Action,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            action,
            intent,
            enabled: true,
            alert: false,
        }
    }

    pub fn with_description(mut self, description: impl Into<Cow<'a, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn alert(mut self, alert: bool) -> Self {
        self.alert = alert;
        self
    }
}

pub struct ContextPanel<'a, TState> {
    pub id: Cow<'a, str>,
    pub width: f32,
    pub frame: egui::Frame,
    pub show: Box<dyn FnMut(&mut egui::Ui, &mut TState) + 'a>,
}

impl<'a, TState> ContextPanel<'a, TState> {
    pub fn new<F>(id: impl Into<Cow<'a, str>>, width: f32, frame: egui::Frame, show: F) -> Self
    where
        F: FnMut(&mut egui::Ui, &mut TState) + 'a,
    {
        Self {
            id: id.into(),
            width,
            frame,
            show: Box::new(show),
        }
    }
}

pub struct ContentPane<'a, TState, Intent> {
    pub intent: Intent,
    pub show: Box<dyn FnMut(&mut egui::Ui, &mut TState) + 'a>,
}

impl<'a, TState, Intent> ContentPane<'a, TState, Intent> {
    pub fn new<F>(intent: Intent, show: F) -> Self
    where
        F: FnMut(&mut egui::Ui, &mut TState) + 'a,
    {
        Self {
            intent,
            show: Box::new(show),
        }
    }
}

pub fn show_shell<TState, Intent>(
    ctx: &egui::Context,
    state: &mut TState,
    style: &ShellStyle,
    navigation: &[NavigationItem<'_, Intent>],
    leading_panels: &mut [ContextPanel<'_, TState>],
    trailing_panels: &mut [ContextPanel<'_, TState>],
    active_intent: &Intent,
    content_panes: &mut [ContentPane<'_, TState, Intent>],
) -> bool
where
    TState: ActionDispatcher,
    Intent: PartialEq,
{
    for panel in leading_panels.iter_mut() {
        egui::SidePanel::left(panel.id.as_ref().to_owned())
            .resizable(false)
            .exact_width(panel.width)
            .frame(panel.frame.clone())
            .show(ctx, |ui| {
                (panel.show)(ui, state);
            });
    }

    egui::SidePanel::left("shell_navigation")
        .resizable(false)
        .exact_width(style.nav_width)
        .frame(style.navigation_panel_frame.clone())
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                for item in navigation {
                    let is_active = &item.intent == active_intent;
                    let frame = if is_active {
                        style.navigation_selected_item_frame.clone()
                    } else {
                        style.navigation_item_frame.clone()
                    };
                    let action = item.action.clone();
                    let action_enabled = item.enabled && state.is_action_enabled(action.clone());

                    let inner_response = frame.show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let content = ui.add_enabled_ui(action_enabled, |ui| {
                            let mut title = egui::RichText::new(item.label.as_ref())
                                .color(if is_active {
                                    style.navigation_selected_text
                                } else if item.alert {
                                    style.navigation_alert_text
                                } else {
                                    style.navigation_text
                                })
                                .strong();

                            if is_active {
                                title = title.underline();
                            }

                            ui.label(title);
                            if let Some(description) = &item.description {
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(description.as_ref())
                                        .color(style.navigation_text.gamma_multiply(0.8))
                                        .small(),
                                );
                            }
                        });
                        content.response
                    });

                    let response = inner_response.response;
                    if response.clicked() && action_enabled {
                        state.trigger_action(action);
                    }

                    ui.add_space(8.0);
                }
            });
        });

    for panel in trailing_panels.iter_mut() {
        egui::SidePanel::right(panel.id.as_ref().to_owned())
            .resizable(false)
            .exact_width(panel.width)
            .frame(panel.frame.clone())
            .show(ctx, |ui| {
                (panel.show)(ui, state);
            });
    }

    let mut content_displayed = false;
    egui::CentralPanel::default()
        .frame(style.content_frame.clone())
        .show(ctx, |ui| {
            if let Some(pane) = content_panes
                .iter_mut()
                .find(|pane| pane.intent == *active_intent)
            {
                (pane.show)(ui, state);
                content_displayed = true;
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Select an item from the navigation to begin.");
                });
            }
        });

    content_displayed
}
