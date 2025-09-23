use eframe::egui;

use crate::{MissingRequiredFile, ProjectRequirementStatus};

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
    let available = ui.available_width();
    let width = available.min(max_width);
    let margin = ((available - width) * 0.5).max(0.0);

    let mut result = None;
    ui.horizontal(|ui| {
        if margin > 0.0 {
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

        if margin > 0.0 {
            ui.add_space(margin);
        }
    });

    result.expect("centered_column should always produce a result")
}

fn requirement_description(file: &MissingRequiredFile) -> String {
    match file.reason.detail() {
        Some(detail) => format!("{} ({detail})", file.name),
        None => file.name.clone(),
    }
}

pub(crate) fn project_requirements_checklist(
    ui: &mut egui::Ui,
    requirements: &[ProjectRequirementStatus],
) {
    if requirements.is_empty() {
        return;
    }

    let original_spacing = ui.spacing().item_spacing.y;
    let adjusted_spacing = original_spacing.max(4.0);
    ui.spacing_mut().item_spacing.y = adjusted_spacing;

    for status in requirements {
        let description = requirement_description(&status.file);
        let mut satisfied = status.satisfied;
        let text = if status.satisfied {
            egui::RichText::new(description)
        } else {
            egui::RichText::new(description).color(egui::Color32::YELLOW)
        };
        let response = ui.add_enabled(false, egui::Checkbox::new(&mut satisfied, text));
        if status.satisfied {
            response.on_hover_text("Requirement satisfied.");
        } else {
            let mut tooltip = String::from("Missing from the selected project folder.");
            if let Some(detail) = status.file.reason.detail() {
                tooltip.push(' ');
                tooltip.push_str(detail);
                tooltip.push('.');
            }
            response.on_hover_text(tooltip);
        }
    }

    ui.spacing_mut().item_spacing.y = original_spacing;
}
