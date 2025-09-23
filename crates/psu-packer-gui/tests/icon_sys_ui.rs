use eframe::egui::{self, CentralPanel};
use psu_packer::ICON_SYS_PRESETS;
use psu_packer_gui::ui::icon_sys::{icon_sys_snapshot, render_icon_sys_editor};
use psu_packer_gui::ui::theme;
use psu_packer_gui::PackerApp;

#[test]
fn icon_sys_editor_renders_default_state() {
    let mut app = PackerApp::default();

    let ctx = egui::Context::default();
    theme::install(&ctx, &theme::Palette::default());
    ctx.begin_frame(egui::RawInput::default());
    CentralPanel::default().show(&ctx, |ui| {
        render_icon_sys_editor(&mut app, ui);
    });
    let full_output = ctx.end_frame();

    assert!(full_output
        .shapes
        .iter()
        .any(|shape| !matches!(shape.shape, egui::epaint::Shape::Noop)));

    let default_preset = &ICON_SYS_PRESETS[0];
    let snapshot = icon_sys_snapshot(&app);
    assert_eq!(
        snapshot.background_transparency,
        default_preset.background_transparency
    );
    assert_eq!(snapshot.background_colors, default_preset.background_colors);
    assert_eq!(snapshot.light_directions, default_preset.light_directions);
    assert_eq!(snapshot.light_colors, default_preset.light_colors);
    assert_eq!(snapshot.ambient_color, default_preset.ambient_color);
}
