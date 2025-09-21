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

    let minimum_reasonable = ui.spacing().interact_size.x;

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
    if let Some(value) = explicit_cap {
        sanitized_hints.push(value);
    }

    let plausibility_baseline = sanitized_hints
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);

    let minimum_plausibility = (minimum_reasonable * 0.1).max(epsilon);

    let plausibility_threshold = if plausibility_baseline.is_finite() {
        (plausibility_baseline * 0.1).max(minimum_plausibility)
    } else {
        minimum_plausibility
    };

    let discard_if_implausible = |value: &mut Option<f32>| {
        if let Some(inner) = value {
            if !inner.is_finite() || *inner < plausibility_threshold {
                *value = None;
            }
        }
    };

    discard_if_implausible(&mut primary_available);
    discard_if_implausible(&mut max_rect_available);
    discard_if_implausible(&mut clip_available);
    discard_if_implausible(&mut screen_available);

    let unclamped_available = {
        let mut aggregated_hint: Option<f32> = None;

        for value in [
            primary_available,
            max_rect_available,
            clip_available,
            screen_available,
        ] {
            if let Some(inner) = value {
                aggregated_hint = Some(match aggregated_hint {
                    Some(current) => current.min(inner),
                    None => inner,
                });
            }
        }

        aggregated_hint
            .or(explicit_cap)
            .unwrap_or_else(|| minimum_reasonable.max(epsilon))
    };

    let available = explicit_cap
        .map(|cap| unclamped_available.min(cap))
        .unwrap_or(unclamped_available);

    let working_bound = if explicit_cap.is_some() {
        available
    } else {
        unclamped_available
    };

    let bound_to_working_width = |value: &mut Option<f32>| {
        if let Some(inner) = value {
            if inner.is_finite() {
                *inner = inner.min(working_bound);
            } else {
                *value = Some(working_bound);
            }
        }
    };

    bound_to_working_width(&mut clip_available);
    bound_to_working_width(&mut screen_available);
    bound_to_working_width(&mut max_rect_available);

    let viewport_width = clip_available
        .or(screen_available)
        .or(max_rect_available)
        .or(Some(working_bound));

    let bounded_viewport_width = viewport_width.map(|view| view.min(working_bound));

    let effective_floor = bounded_viewport_width
        .map(|view| view.min(minimum_reasonable))
        .unwrap_or(minimum_reasonable);

    let inner_max_width = explicit_cap.unwrap_or(f32::INFINITY);
    let width = available
        .max(effective_floor)
        .min(inner_max_width)
        .max(epsilon);
    let margin = bounded_viewport_width
        .map(|view| ((view - width) * 0.5).max(0.0))
        .unwrap_or(0.0);

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
