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


    let sanitize = |value: f32| -> Option<f32> {
        if value.is_finite() && value > epsilon {
            Some(value)
        } else {
            None
        }
    };

    let minimum_reasonable = ui.spacing().interact_size.x;

    let explicit_cap = explicit_max.and_then(|max| sanitize(max));
    let mut hint_values = [
        sanitize(ui.available_width()),
        sanitize(ui.max_rect().width()),
        sanitize(ui.clip_rect().width()),
        sanitize(ui.ctx().screen_rect().width()),
    ];

    let (unclamped_available, available) =
        reconcile_runtime_hints(&mut hint_values, explicit_cap, minimum_reasonable, epsilon);

    let [_primary_available, mut max_rect_available, mut clip_available, mut screen_available] =
        hint_values;

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

fn reconcile_runtime_hints(
    hints: &mut [Option<f32>],
    explicit_cap: Option<f32>,
    minimum_reasonable: f32,
    epsilon: f32,
) -> (f32, f32) {
    let mut sanitized_values: Vec<f32> = hints.iter().filter_map(|opt| *opt).collect();

    let plausibility_baseline = sanitized_values
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);

    let minimum_plausibility = (minimum_reasonable * 0.1).max(epsilon);

    let plausibility_threshold = if plausibility_baseline.is_finite() {
        (plausibility_baseline * 0.1).max(minimum_plausibility)
    } else {
        minimum_plausibility
    };

    let plausible_count = sanitized_values
        .iter()
        .filter(|&&value| value >= plausibility_threshold)
        .count();

    let allow_runtime_discard = plausible_count >= 2;
    let treat_single_outlier_as_implausible = plausible_count == 1;

    for value in hints.iter_mut() {
        if let Some(inner) = value {
            if !inner.is_finite()
                || (allow_runtime_discard && *inner < plausibility_threshold)
                || (treat_single_outlier_as_implausible && *inner >= plausibility_threshold)
            {
                *value = None;
            }
        }
    }

    sanitized_values = hints.iter().filter_map(|opt| *opt).collect();

    let mut aggregated_hint: Option<f32> = None;

    for value in sanitized_values {
        aggregated_hint = Some(match aggregated_hint {
            Some(current) => current.min(value),
            None => value,
        });
    }

    let unclamped_available = aggregated_hint
        .or(explicit_cap)
        .unwrap_or_else(|| minimum_reasonable.max(epsilon));

    let available = explicit_cap
        .map(|cap| unclamped_available.min(cap))
        .unwrap_or(unclamped_available);

    (unclamped_available, available)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_single_implausible_outlier() {
        let explicit_cap = Some(1180.0);
        let minimum_reasonable = 16.0;
        let epsilon = f32::EPSILON;

        let mut hints = [
            Some(1.5_f32),
            Some(1.5_f32),
            Some(0.5_f32),
            Some(16384.0_f32),
        ];

        let (unclamped_available, available) =
            reconcile_runtime_hints(&mut hints, explicit_cap, minimum_reasonable, epsilon);

        assert!(
            hints[3].is_none(),
            "expected the implausible outlier to be discarded"
        );
        assert!(
            available < explicit_cap.unwrap(),
            "available width should not fall back to the max-width cap"
        );
        assert!(
            unclamped_available <= 1.5,
            "unclamped width should be driven by the surviving viewport hints"
        );
    }
}
