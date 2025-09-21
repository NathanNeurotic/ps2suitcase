use eframe::egui;

pub mod dialogs;
pub mod file_picker;
pub mod icon_sys;
pub mod pack_controls;
pub mod theme;
pub mod timestamps;

pub(crate) fn centered_column<R>(
    ui: &mut egui::Ui,
    max_width: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let available_width = ui.available_size_before_wrap().x;
    let content_width = available_width.min(max_width);
    let margin = (available_width - content_width).max(0.0) / 2.0;

    ui.horizontal(|ui| {
        if margin > 0.0 {
            ui.add_space(margin);
        }
        let result = ui
            .vertical(|ui| {
                ui.set_max_width(content_width);
                add_contents(ui)
            })
            .inner;
        if margin > 0.0 {
            ui.add_space(margin);
        }
        result
    })
    .inner
}
