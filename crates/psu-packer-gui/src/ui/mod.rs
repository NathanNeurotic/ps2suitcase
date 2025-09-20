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
    let epsilon = f32::EPSILON;
    let primary_available = ui.available_width();
    let available = if primary_available.is_finite() && primary_available > epsilon {
        primary_available
    } else {
        let fallback_available = ui.max_rect().width();
        if fallback_available.is_finite() && fallback_available > epsilon {
            fallback_available
        } else if max_width.is_finite() && max_width > epsilon {
            max_width
        } else {
            epsilon
        }
    };

    let safe_max_width = if max_width.is_finite() && max_width > epsilon {
        max_width
    } else {
        available
    };

    let width = available.min(safe_max_width).max(epsilon);
    let margin = ((available - width) * 0.5).max(0.0);

    let mut result = None;
    ui.horizontal(|ui| {
        if margin > epsilon {
            ui.add_space(margin);
        }

        result = Some(
            ui.scope(|ui| {
                ui.set_width(width);
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    add_contents(ui)
                })
                .inner
            })
            .inner,
        );

        if margin > epsilon {
            ui.add_space(margin);
        }
    });

    result.expect("centered_column should always produce a result")
}
