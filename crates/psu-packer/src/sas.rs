use std::{collections::HashSet, path::Path};

use chrono::{
    DateTime, Duration, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone,
    Timelike, Utc,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::from_str;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct SharedSasData {
    charset: String,
    defaults: SharedSasDefaults,
    categories: Vec<SharedCategory>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct SharedSasDefaults {
    seconds_between_items: u32,
    slots_per_category: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct SharedCategory {
    key: String,
    #[serde(default)]
    aliases: Vec<String>,
}

static SHARED_SAS_DATA: Lazy<SharedSasData> =
    Lazy::new(|| from_str(include_str!("sas_data.json")).expect("valid SAS shared data JSON"));

fn shared_sas_data() -> &'static SharedSasData {
    &SHARED_SAS_DATA
}

fn canonical_category_aliases() -> &'static [SharedCategory] {
    &shared_sas_data().categories
}

fn is_supported_alias(key: &str, alias: &str) -> bool {
    canonical_category_aliases()
        .iter()
        .find(|group| group.key == key)
        .map(|group| group.aliases.iter().any(|candidate| candidate == alias))
        .unwrap_or(false)
}

fn charset() -> &'static str {
    &shared_sas_data().charset
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimestampRules {
    #[serde(default = "TimestampRules::default_seconds_between_items")]
    pub seconds_between_items: u32,
    #[serde(default = "TimestampRules::default_slots_per_category")]
    pub slots_per_category: u32,
    #[serde(default = "TimestampRules::default_categories")]
    pub categories: Vec<CategoryRule>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryRule {
    pub key: String,
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl CategoryRule {
    fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            aliases: Vec::new(),
        }
    }
}

impl TimestampRules {
    fn default_seconds_between_items() -> u32 {
        shared_sas_data().defaults.seconds_between_items
    }

    fn default_slots_per_category() -> u32 {
        shared_sas_data().defaults.slots_per_category
    }

    fn default_categories() -> Vec<CategoryRule> {
        canonical_category_aliases()
            .iter()
            .map(|group| {
                let mut category = CategoryRule::new(&group.key);
                if !group.aliases.is_empty() {
                    category.aliases = group.aliases.clone();
                }
                category
            })
            .collect()
    }

    pub fn sanitize(&mut self) {
        if self.seconds_between_items == 0 {
            self.seconds_between_items = Self::default_seconds_between_items();
        }
        let adjusted = u64::from(self.seconds_between_items.max(2));
        let next_even = ((adjusted + 1) / 2) * 2;
        let max_even = {
            let max_value = u64::from(u32::MAX);
            max_value - (max_value % 2)
        };
        self.seconds_between_items = next_even.min(max_even) as u32;
        // Slots per category are pinned to a single local day worth of two-second slots so
        // each category can occupy its own date on the timeline.
        self.slots_per_category = Self::default_slots_per_category();

        if self.categories.is_empty() {
            *self = Self::default();
            return;
        }

        let mut sanitized = Vec::with_capacity(self.categories.len());
        let mut seen_keys: HashSet<String> = HashSet::new();

        for category in self.categories.drain(..) {
            let key = category.key.trim().to_ascii_uppercase();
            if key.is_empty() {
                continue;
            }
            if !seen_keys.insert(key.clone()) {
                continue;
            }

            let mut aliases: Vec<String> = category
                .aliases
                .into_iter()
                .filter_map(|alias| sanitize_alias(alias, &key))
                .collect();

            let mut seen_aliases = HashSet::new();
            aliases.retain(|alias| seen_aliases.insert(alias.clone()));

            sanitized.push(CategoryRule { key, aliases });
        }

        if !sanitized.iter().any(|category| category.key == "DEFAULT") {
            sanitized.push(CategoryRule {
                key: "DEFAULT".to_string(),
                aliases: Vec::new(),
            });
        }

        self.categories = sanitized;
    }

    pub fn seconds_between_items_i64(&self) -> i64 {
        i64::from(self.seconds_between_items)
    }

    pub fn slots_per_category_i64(&self) -> i64 {
        i64::from(self.slots_per_category)
    }
}

impl Default for TimestampRules {
    fn default() -> Self {
        Self {
            seconds_between_items: Self::default_seconds_between_items(),
            slots_per_category: Self::default_slots_per_category(),
            categories: Self::default_categories(),
        }
    }
}

fn sanitize_alias(alias: String, key: &str) -> Option<String> {
    let mut value = alias.trim().to_ascii_uppercase();
    if value.is_empty() {
        return None;
    }

    if key != "APPS" && key != "DEFAULT" && value.starts_with(key) {
        value = value[key.len()..].to_string();
    }

    if value.is_empty() {
        return None;
    }

    if !is_supported_alias(key, &value) {
        return None;
    }

    Some(value)
}

pub fn planned_timestamp_for_folder(path: &Path, rules: &TimestampRules) -> Option<NaiveDateTime> {
    let name = path.file_name()?.to_str()?;
    planned_timestamp_for_name(name, rules)
}

pub fn planned_timestamp_for_name(name: &str, rules: &TimestampRules) -> Option<NaiveDateTime> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Each category begins at ANCHOR_START + category_index days at local midnight and then
    // advances forward in two-second slots within that day.
    let (category_index, slot_offset_seconds) = deterministic_offset_seconds(trimmed, rules)?;
    let anchor_date = anchor_naive_date()?;
    let midnight = NaiveTime::from_hms_opt(0, 0, 0)?;
    let category_start_date =
        anchor_date.checked_add_signed(Duration::days(category_index as i64))?;
    let category_start = NaiveDateTime::new(category_start_date, midnight);
    let local_midnight = match Local.from_local_datetime(&category_start) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(dt, alt) => dt.min(alt),
        LocalResult::None => return None,
    };

    let planned_local = local_midnight + Duration::seconds(slot_offset_seconds);
    let planned_utc = planned_local.with_timezone(&Utc);
    let snapped = snap_even_second(planned_utc);
    let local = snapped.with_timezone(&Local);
    Some(local.naive_local())
}

fn deterministic_offset_seconds(name: &str, rules: &TimestampRules) -> Option<(usize, i64)> {
    let effective = normalize_name_for_rules(name, rules)?;
    let category_index = category_priority_index(&effective, rules)?;
    let slot = slot_index_within_category(&effective, rules);
    let name_offset = slot * rules.seconds_between_items_i64();
    Some((category_index, name_offset))
}

fn normalize_name_for_rules(name: &str, rules: &TimestampRules) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    let upper = trimmed.to_ascii_uppercase();

    for category in &rules.categories {
        if category.aliases.iter().any(|alias| *alias == upper) {
            return Some(match category.key.as_str() {
                "APPS" => String::from("APPS"),
                "DEFAULT" => upper,
                key => format!("{key}{upper}"),
            });
        }
    }

    Some(upper)
}

fn category_priority_index(effective: &str, rules: &TimestampRules) -> Option<usize> {
    find_category(effective, rules).map(|(index, _)| index)
}

fn find_category<'a>(
    effective: &str,
    rules: &'a TimestampRules,
) -> Option<(usize, &'a CategoryRule)> {
    let mut fallback: Option<(usize, &'a CategoryRule)> = None;

    for (index, category) in rules.categories.iter().enumerate() {
        match category.key.as_str() {
            "DEFAULT" => fallback = Some((index, category)),
            "APPS" => {
                if effective == "APPS" {
                    return Some((index, category));
                }
            }
            key => {
                if effective.starts_with(key) {
                    return Some((index, category));
                }
            }
        }
    }

    fallback
}

fn slot_index_within_category(effective: &str, rules: &TimestampRules) -> i64 {
    let payload = payload_for_effective(effective, rules);

    let mut total = 0.0f64;
    let mut scale = 1.0f64;
    let charset = charset();

    for ch in payload.chars().take(128) {
        scale *= charset.len() as f64;
        let index = match charset.find(ch.to_ascii_uppercase()) {
            Some(idx) => idx + 1,
            None => charset.len(),
        } as f64;
        total += index / scale;
    }

    let slots_per_category = rules.slots_per_category_i64();
    let mut slot = (total * slots_per_category as f64).floor() as i64;
    if slot >= slots_per_category {
        slot = slots_per_category - 1;
    }
    slot
}

fn payload_for_effective(effective: &str, rules: &TimestampRules) -> String {
    if let Some((_, category)) = find_category(effective, rules) {
        match category.key.as_str() {
            "APPS" => "APPS".to_string(),
            "DEFAULT" => effective.replace('-', ""),
            key => effective
                .strip_prefix(key)
                .unwrap_or(effective)
                .replace('-', ""),
        }
    } else {
        effective.replace('-', "")
    }
}

/// Returns the local anchor date (December 31, 2098) used as the forward-moving baseline
/// for SAS timestamps.
fn anchor_naive_date() -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(2098, 12, 31)
}

fn snap_even_second(dt: DateTime<Utc>) -> DateTime<Utc> {
    let mut snapped = dt.with_nanosecond(0).unwrap_or(dt);
    if snapped.second() % 2 == 1 {
        snapped += Duration::seconds(1);
    }
    snapped
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::to_string_pretty;
    use std::path::PathBuf;

    #[test]
    fn produces_even_seconds() {
        let path = PathBuf::from("APP_SAMPLE");
        let rules = TimestampRules::default();
        let timestamp = planned_timestamp_for_folder(&path, &rules).expect("timestamp");
        assert_eq!(timestamp.second() % 2, 0);
        assert_eq!(timestamp.nanosecond(), 0);
    }

    #[test]
    fn handles_aliases() {
        let mut rules = TimestampRules::default();
        rules.sanitize();
        let path = PathBuf::from("boot");
        let ts_boot = planned_timestamp_for_folder(&path, &rules).expect("timestamp");
        let sys_path = PathBuf::from("SYS_BOOT");
        let ts_sys = planned_timestamp_for_folder(&sys_path, &rules).expect("timestamp");
        assert_eq!(ts_boot, ts_sys);
    }

    #[test]
    fn canonical_aliases_match_prefixed_names() {
        let mut rules = TimestampRules::default();
        rules.sanitize();

        let alias_path = PathBuf::from("osdxmb");
        let prefixed_path = PathBuf::from("APP_OSDXMB");

        let alias_timestamp =
            planned_timestamp_for_folder(&alias_path, &rules).expect("alias timestamp");
        let prefixed_timestamp =
            planned_timestamp_for_folder(&prefixed_path, &rules).expect("prefixed timestamp");

        assert_eq!(alias_timestamp, prefixed_timestamp);
    }

    #[test]
    fn unsupported_aliases_are_removed() {
        let mut rules = TimestampRules::default();
        if let Some(category) = rules
            .categories
            .iter_mut()
            .find(|category| category.key == "RAA_")
        {
            category.aliases = vec!["INVALID".to_string()];
        }

        rules.sanitize();

        let aliases = rules
            .categories
            .iter()
            .find(|category| category.key == "RAA_")
            .expect("category");

        assert!(aliases.aliases.is_empty());
    }

    #[test]
    fn odd_spacing_produces_distinct_consecutive_timestamps() {
        let mut rules = TimestampRules {
            seconds_between_items: 3,
            slots_per_category: 32,
            categories: vec![CategoryRule {
                key: "DEFAULT".to_string(),
                aliases: Vec::new(),
            }],
        };
        rules.sanitize();
        assert!(rules.seconds_between_items >= 2);
        assert_eq!(rules.seconds_between_items % 2, 0);
        assert_eq!(
            rules.slots_per_category,
            TimestampRules::default_slots_per_category()
        );

        let first_name = "A";
        let second_name = "B";
        let first_effective =
            normalize_name_for_rules(first_name, &rules).expect("first effective name");
        let second_effective =
            normalize_name_for_rules(second_name, &rules).expect("second effective name");
        let first_slot = slot_index_within_category(&first_effective, &rules);
        let second_slot = slot_index_within_category(&second_effective, &rules);
        assert!(
            second_slot >= first_slot,
            "expected ordering to be monotonic"
        );

        let first_timestamp =
            planned_timestamp_for_name(first_name, &rules).expect("first timestamp");
        let second_timestamp =
            planned_timestamp_for_name(second_name, &rules).expect("second timestamp");

        assert!(second_timestamp > first_timestamp);
    }

    #[test]
    fn shared_data_round_trip_matches_defaults() {
        let shared = shared_sas_data();
        let rules = TimestampRules::default();

        assert_eq!(
            shared.defaults.seconds_between_items,
            rules.seconds_between_items
        );
        assert_eq!(shared.defaults.slots_per_category, rules.slots_per_category);

        let categories_from_rules: Vec<SharedCategory> = rules
            .categories
            .iter()
            .map(|category| SharedCategory {
                key: category.key.clone(),
                aliases: category.aliases.clone(),
            })
            .collect();

        assert_eq!(shared.categories, categories_from_rules);

        let json = to_string_pretty(shared).expect("serialize shared data");
        let reparsed: SharedSasData = serde_json::from_str(&json).expect("reparse shared data");
        assert_eq!(*shared, reparsed);
    }
}
