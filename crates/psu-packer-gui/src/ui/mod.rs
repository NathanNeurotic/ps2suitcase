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
    let explicit_max = if max_width.is_finite() && max_width > epsilon {
        Some(max_width)
    } else {
        None
    };

    let sanitize = |value: f32| -> Option<f32> {
        if value.is_finite() && value > epsilon {
            Some(value)
        } else {
            None
        }
    };

    let mut sanitized_hints: Vec<f32> = Vec::new();

    let mut primary_available = sanitize(ui.available_width());
    if let Some(value) = primary_available {
        sanitized_hints.push(value);
    }

    let mut max_rect_available = sanitize(ui.max_rect().width());
    if let Some(value) = max_rect_available {
        sanitized_hints.push(value);
    }

    let mut clip_available = sanitize(ui.clip_rect().width());
    if let Some(value) = clip_available {
        sanitized_hints.push(value);
    }

    let mut screen_available = sanitize(ui.ctx().screen_rect().width());
    if let Some(value) = screen_available {
        sanitized_hints.push(value);
    }

    let explicit_cap = explicit_max.and_then(|max| sanitize(max));

    let viewport_floor = sanitized_hints
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);

    let plausibility_floor = if viewport_floor.is_finite() {
        viewport_floor.mul_add(0.1, 0.0)
    } else {
        0.0
    }
    .clamp(0.0, ui.spacing().interact_size.x);

    let discard_if_implausible = |value: &mut Option<f32>| {
        if let Some(inner) = value {
            if !inner.is_finite() || *inner <= plausibility_floor.max(epsilon) {
                *value = None;
            }
        }
    };

    discard_if_implausible(&mut primary_available);
    discard_if_implausible(&mut max_rect_available);
    discard_if_implausible(&mut clip_available);
    discard_if_implausible(&mut screen_available);

    let available = if let Some(primary) = primary_available {
        let mut candidate = primary;

        if let Some(max_rect) = max_rect_available {
            candidate = candidate.min(max_rect);
        }

        if let Some(clip) = clip_available {
            candidate = candidate.min(clip);
        }

        if let Some(screen) = screen_available {
            candidate = candidate.min(screen);
        }

        if let Some(cap) = explicit_cap {
            candidate = candidate.min(cap);
        }

        candidate
    } else if let Some(max_rect) = max_rect_available {
        let mut candidate = max_rect;

        if let Some(clip) = clip_available {
            candidate = candidate.min(clip);
        }

        if let Some(screen) = screen_available {
            candidate = candidate.min(screen);
        }

        if let Some(cap) = explicit_cap {
            candidate = candidate.min(cap);
        }

        candidate
    } else if let Some(cap) = explicit_cap {
        cap
    } else {
        epsilon
    };

    let safe_max_width = explicit_cap.unwrap_or(available);

    let viewport_width = clip_available
        .or(screen_available)
        .or(max_rect_available)
        .or(Some(available));

    let minimum_reasonable = ui.spacing().interact_size.x;
    let effective_floor = viewport_width
        .map(|view| view.min(minimum_reasonable))
        .unwrap_or(minimum_reasonable)
        .min(safe_max_width);

    let clamped_available = available.max(effective_floor);

    let width = clamped_available.min(safe_max_width).max(epsilon);
    let margin = ((clamped_available - width) * 0.5).max(0.0);

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
