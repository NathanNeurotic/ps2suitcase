use crate::state::MissingRequiredFile;
use psu_packer::sas::TimestampRules;

pub fn sanitize_seconds_between_items(value: u32) -> u32 {
    let adjusted = u64::from(value.max(2));
    let next_even = ((adjusted + 1) / 2) * 2;
    let max_value = u64::from(u32::MAX);
    let max_even = max_value - (max_value % 2);
    next_even.min(max_even) as u32
}

pub fn timestamp_rules_equal(left: &TimestampRules, right: &TimestampRules) -> bool {
    if left.seconds_between_items != right.seconds_between_items
        || left.slots_per_category != right.slots_per_category
        || left.categories.len() != right.categories.len()
    {
        return false;
    }

    left.categories
        .iter()
        .zip(right.categories.iter())
        .all(|(lhs, rhs)| lhs.key == rhs.key && lhs.aliases == rhs.aliases)
}

pub fn format_missing_required_files_message(missing: &[MissingRequiredFile]) -> String {
    let formatted = missing
        .iter()
        .map(|entry| match entry.reason.detail() {
            Some(detail) => format!("• {} ({detail})", entry.name),
            None => format!("• {}", entry.name),
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "The selected folder is missing files needed to pack the PSU:\n{}",
        formatted
    )
}
