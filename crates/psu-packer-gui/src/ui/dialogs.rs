use eframe::egui;

use crate::PackerApp;
use gui_core::actions::Action;
use gui_core::ActionDispatcher;

pub(crate) fn pack_confirmation(app: &mut PackerApp, ctx: &egui::Context) {
    if let Some(missing) = app.packer_state.pending_pack_missing_files() {
        let message = PackerApp::format_missing_required_files_message(missing);
        egui::Window::new("Confirm Packing")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(&message);
                ui.add_space(12.0);
                ui.label("Pack anyway?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Proceed").clicked() {
                        app.trigger_action(Action::ConfirmPack);
                    }
                    if ui.button("Go Back").clicked() {
                        app.trigger_action(Action::CancelPack);
                    }
                });
            });
    }
}

pub(crate) fn exit_confirmation(app: &mut PackerApp, ctx: &egui::Context) {
    if app.show_exit_confirm {
        egui::Window::new("Confirm Exit")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Are you sure you want to exit?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let yes_clicked = ui.button("Yes").clicked();
                    let no_clicked = ui.button("No").clicked();

                    if yes_clicked {
                        app.trigger_action(Action::ConfirmExit);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    } else if no_clicked {
                        app.trigger_action(Action::CancelExit);
                    }
                });
            });
    }
}
