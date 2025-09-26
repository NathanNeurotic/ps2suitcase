use eframe::egui;

use crate::{
    state::PackerApp,
    ui::{self, theme},
};
use gui_core::actions::{self, Action, ActionDescriptor};
use gui_core::ActionDispatcher;

mod shell;

pub trait View<TState> {
    fn show(&mut self, ui: &mut egui::Ui, state: &mut TState);
}

#[derive(Default)]
pub struct TimestampRulesPanel;

impl View<PackerApp> for TimestampRulesPanel {
    fn show(&mut self, ui: &mut egui::Ui, state: &mut PackerApp) {
        ui::timestamps::timestamp_rules_editor(state, ui);
    }
}

impl eframe::App for PackerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_pack_job();

        if ctx.input(|i| i.viewport().close_requested()) && !self.exit_confirmed {
            self.trigger_action(Action::ShowExitConfirmation);
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        self.zoom_factor = self.zoom_factor.clamp(0.5, 2.0);
        ctx.set_pixels_per_point(self.zoom_factor);

        let source_present = self.has_source();
        if !source_present && self.packer_state.source_present_last_frame {
            self.reset_metadata_fields();
        }
        self.packer_state.source_present_last_frame = source_present;

        if let Some(progress) = self.packer_state.pack_progress_value() {
            ctx.request_repaint();
            egui::Window::new("packing_progress")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .frame(egui::Frame::popup(&ctx.style()))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Packing PSU…")
                                .font(theme::display_font(26.0))
                                .color(self.theme.neon_accent),
                        );
                        ui.add_space(8.0);
                        ui.add(
                            egui::ProgressBar::new(progress)
                                .desired_width(200.0)
                                .animate(true),
                        );
                    });
                });
        }

        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::none().fill(self.theme.background))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                theme::draw_vertical_gradient(
                    ui.painter(),
                    rect,
                    self.theme.header_top,
                    self.theme.header_bottom,
                );
                let separator_rect =
                    egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - 2.0), rect.max);
                theme::draw_separator(ui.painter(), separator_rect, self.theme.separator);
                egui::menu::bar(ui, |ui| {
                    ui::file_picker::file_menu(self, ui);
                    ui.menu_button("View", |ui| {
                        let zoom_in = ActionDescriptor::new(Action::ZoomIn, "Zoom In");
                        actions::action_button(ui, self, &zoom_in);
                        let zoom_out = ActionDescriptor::new(Action::ZoomOut, "Zoom Out");
                        actions::action_button(ui, self, &zoom_out);
                        let reset_zoom = ActionDescriptor::new(Action::ResetZoom, "Reset Zoom");
                        actions::action_button(ui, self, &reset_zoom);
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);
                        let zoom_text = format!("Zoom: {:.0}%", self.zoom_factor * 100.0);
                        ui.label(egui::RichText::new(zoom_text).color(self.theme.neon_accent));
                    });
                });
            });

        shell::show_shell(ctx, self);

        ui::dialogs::pack_confirmation(self, ctx);
        ui::dialogs::exit_confirmation(self, ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SasPrefix, TIMESTAMP_FORMAT};
    use chrono::NaiveDateTime;
    use eframe::egui;
    use gui_core::actions::{
        Action, ActionDescriptor, EditorAction, MetadataAction, TimestampAction,
        TimestampStrategyAction,
    };
    use gui_core::ActionDispatcher;
    use std::{fs, path::Path, thread, time::Duration};
    use tempfile::tempdir;

    fn write_required_files(folder: &Path) {
        for file in crate::REQUIRED_PROJECT_FILES {
            let path = folder.join(file);
            fs::write(&path, b"data").expect("write required file");
        }
    }

    fn dispatch_action(app: &mut PackerApp, descriptor: &ActionDescriptor) {
        let action = descriptor.action.clone();
        assert!(app.supports_action(action.clone()));
        assert!(app.is_action_enabled(action.clone()));
        app.trigger_action(action);
    }

    fn wait_for_pack_completion(app: &mut PackerApp) {
        while app.pack_job_active() {
            thread::sleep(Duration::from_millis(10));
            app.poll_pack_job();
        }
    }

    #[test]
    fn timestamp_panel_serializes_rules() {
        let mut app = PackerApp::default();
        let open_timestamp_editor = ActionDescriptor::new(
            Action::OpenEditor(EditorAction::TimestampAutomation),
            "Timestamp rules",
        );
        dispatch_action(&mut app, &open_timestamp_editor);
        app.packer_state
            .timestamp_rules_ui
            .set_seconds_between_items(8);
        app.packer_state
            .timestamp_rules_ui
            .apply_to_rules(&mut app.packer_state.timestamp_rules);

        let mut panel = TimestampRulesPanel::default();
        let ctx = egui::Context::default();
        let palette = app.theme.clone();
        theme::install(&ctx, &palette);
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.show(ui, &mut app));
        });

        let serialized = app
            .packer_state
            .timestamp_rules_ui
            .serialize()
            .expect("serialize timestamp rules");
        assert!(serialized.contains("\"seconds_between_items\": 8"));
    }

    #[test]
    fn pack_job_completes_after_view_interaction() {
        let workspace = tempdir().expect("temp workspace");
        let project_dir = workspace.path().join("project");
        fs::create_dir_all(&project_dir).expect("create project folder");
        write_required_files(&project_dir);

        let mut app = PackerApp::default();
        app.packer_state.folder = Some(project_dir.clone());
        app.packer_state.refresh_missing_required_project_files();
        app.packer_state.timestamp = Some(
            NaiveDateTime::parse_from_str("2024-01-01 12:00:00", TIMESTAMP_FORMAT)
                .expect("parse timestamp"),
        );

        let select_app_prefix = ActionDescriptor::new(
            Action::Metadata(MetadataAction::SelectPrefix(SasPrefix::App)),
            "Select prefix",
        );
        let set_folder_name = ActionDescriptor::new(
            Action::Metadata(MetadataAction::SetFolderBaseName("SAVE".to_string())),
            "Set folder name",
        );
        let set_psu_base = ActionDescriptor::new(
            Action::Metadata(MetadataAction::SetPsuFileBaseName("SAVE".to_string())),
            "Set PSU base",
        );
        let manual_strategy = ActionDescriptor::new(
            Action::Timestamp(TimestampAction::SelectStrategy(
                TimestampStrategyAction::Manual,
            )),
            "Manual strategy",
        );

        dispatch_action(&mut app, &select_app_prefix);
        dispatch_action(&mut app, &set_folder_name);
        dispatch_action(&mut app, &set_psu_base);
        dispatch_action(&mut app, &manual_strategy);

        let mut panel = TimestampRulesPanel::default();
        let ctx = egui::Context::default();
        let palette = app.theme.clone();
        theme::install(&ctx, &palette);
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.show(ui, &mut app));
        });

        let output_path = workspace.path().join("output.psu");
        app.packer_state.output = output_path.display().to_string();

        let pack_descriptor = ActionDescriptor::new(Action::PackPsu, "Pack");
        dispatch_action(&mut app, &pack_descriptor);
        wait_for_pack_completion(&mut app);

        assert!(output_path.exists());
        assert!(app.packer_state.error_message.is_none());
    }
}
