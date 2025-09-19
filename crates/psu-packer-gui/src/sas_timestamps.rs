use std::{collections::HashSet, convert::TryFrom, path::Path};

use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
pub(crate) struct CanonicalCategoryAliases {
    pub(crate) key: &'static str,
    pub(crate) aliases: &'static [&'static str],
}

const CANONICAL_CATEGORY_ALIASES: &[CanonicalCategoryAliases] = &[
    CanonicalCategoryAliases {
        key: "APP_",
        aliases: &["OSDXMB", "XEBPLUS"],
    },
    CanonicalCategoryAliases {
        key: "APPS",
        aliases: &[],
    },
    CanonicalCategoryAliases {
        key: "PS1_",
        aliases: &[],
    },
    CanonicalCategoryAliases {
        key: "EMU_",
        aliases: &[],
    },
    CanonicalCategoryAliases {
        key: "GME_",
        aliases: &[],
    },
    CanonicalCategoryAliases {
        key: "DST_",
        aliases: &[],
    },
    CanonicalCategoryAliases {
        key: "DBG_",
        aliases: &[],
    },
    CanonicalCategoryAliases {
        key: "RAA_",
        aliases: &["RESTART", "POWEROFF"],
    },
    CanonicalCategoryAliases {
        key: "RTE_",
        aliases: &["NEUTRINO"],
    },
    CanonicalCategoryAliases {
        key: "DEFAULT",
        aliases: &[],
    },
    CanonicalCategoryAliases {
        key: "SYS_",
        aliases: &["BOOT"],
    },
    CanonicalCategoryAliases {
        key: "ZZY_",
        aliases: &["EXPLOITS"],
    },
    CanonicalCategoryAliases {
        key: "ZZZ_",
        aliases: &["BM", "MATRIXTEAM", "OPL"],
    },
];

pub(crate) fn canonical_category_aliases() -> &'static [CanonicalCategoryAliases] {
    CANONICAL_CATEGORY_ALIASES
}

pub(crate) fn canonical_aliases_for_category(key: &str) -> &'static [&'static str] {
    for group in CANONICAL_CATEGORY_ALIASES {
        if group.key == key {
            return group.aliases;
        }
    }
    &[]
}

fn is_supported_alias(key: &str, alias: &str) -> bool {
    canonical_aliases_for_category(key)
        .iter()
        .any(|candidate| *candidate == alias)
}

const CHARSET: &str = " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_-.";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct TimestampRules {
    #[serde(default = "TimestampRules::default_seconds_between_items")]
    pub(crate) seconds_between_items: u32,
    #[serde(default = "TimestampRules::default_slots_per_category")]
    pub(crate) slots_per_category: u32,
    #[serde(default = "TimestampRules::default_categories")]
    pub(crate) categories: Vec<CategoryRule>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CategoryRule {
    pub(crate) key: String,
    #[serde(default)]
    pub(crate) aliases: Vec<String>,
}

impl CategoryRule {
    fn new(key: &'static str) -> Self {
        Self {
            key: key.to_string(),
            aliases: Vec::new(),
        }
    }

    fn with_aliases(mut self, aliases: &'static [&'static str]) -> Self {
        self.aliases = aliases.iter().map(|alias| alias.to_string()).collect();
        self
    }
}

impl TimestampRules {
    const fn default_seconds_between_items() -> u32 {
        1
    }

    const fn default_slots_per_category() -> u32 {
        86_400
    }

    fn default_categories() -> Vec<CategoryRule> {
        canonical_category_aliases()
            .iter()
            .map(|group| {
                let mut category = CategoryRule::new(group.key);
                if !group.aliases.is_empty() {
                    category = category.with_aliases(group.aliases);
                }
                category
            })
            .collect()
    }

    pub(crate) fn sanitize(&mut self) {
        if self.seconds_between_items == 0 {
            self.seconds_between_items = Self::default_seconds_between_items();
        }
        self.seconds_between_items = self.seconds_between_items.max(1);

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

    pub(crate) fn seconds_between_items_i64(&self) -> i64 {
        i64::from(self.seconds_between_items)
    }

    pub(crate) fn slots_per_category_i64(&self) -> i64 {
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

pub(crate) fn planned_timestamp_for_folder(
    path: &Path,
    rules: &TimestampRules,
) -> Option<NaiveDateTime> {
    let name = path.file_name()?.to_str()?;
    planned_timestamp_for_name(name, rules)
}

pub(crate) fn planned_timestamp_for_name(
    name: &str,
    rules: &TimestampRules,
) -> Option<NaiveDateTime> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    let offsets = deterministic_offset_seconds(trimmed, rules)?;
    let base = fixed_base_datetime_utc()?;
    let planned_utc = base.checked_sub_signed(Duration::seconds(offsets.total_seconds))?;
    Some(planned_utc.with_timezone(&Local).naive_local())
}

struct DeterministicOffsets {
    #[cfg(test)]
    nudge: i64,
    total_seconds: i64,
}

fn deterministic_offset_seconds(
    name: &str,
    rules: &TimestampRules,
) -> Option<DeterministicOffsets> {
    let effective = normalize_name_for_rules(name, rules)?;
    let category_index = category_priority_index(&effective, rules)?;
    let slot_index = slot_index_within_category(&effective, rules);
    let seconds_between_items = rules.seconds_between_items_i64();
    let slots_per_category = rules.slots_per_category_i64();
    let category_block = slots_per_category.checked_mul(seconds_between_items)?;
    let category_index_i64 = i64::try_from(category_index).ok()?;
    let category_offset = category_block.checked_mul(category_index_i64)?;
    let slot_offset = slot_index.checked_mul(seconds_between_items)?;
    let nudge = stable_hash01(&effective);
    let total_seconds = category_offset
        .checked_add(slot_offset)?
        .checked_add(nudge)?;

    Some(DeterministicOffsets {
        #[cfg(test)]
        nudge,
        total_seconds,
    })
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

    for ch in payload.chars().take(128) {
        scale *= CHARSET.len() as f64;
        let index = match CHARSET.find(ch.to_ascii_uppercase()) {
            Some(idx) => idx + 1,
            None => CHARSET.len(),
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

fn fixed_base_datetime_utc() -> Option<DateTime<Utc>> {
    let date = NaiveDate::from_ymd_opt(2099, 1, 1)?;
    let time = NaiveTime::from_hms_opt(7, 59, 59)?;
    Some(DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDateTime::new(date, time),
        Utc,
    ))
}

fn stable_hash01(value: &str) -> i64 {
    let mut hash: u32 = 2_166_136_261;
    for byte in value.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    i64::from(hash & 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn planned_timestamp_uses_fixed_anchor_and_offsets() {
        let mut rules = TimestampRules::default();
        rules.sanitize();

        let name = "APP_SAMPLE";
        let offsets = deterministic_offset_seconds(name, &rules).expect("offsets");
        let base = fixed_base_datetime_utc().expect("base timestamp");
        let expected_utc = base
            .checked_sub_signed(Duration::seconds(offsets.total_seconds))
            .expect("expected utc");
        let expected_local = expected_utc.with_timezone(&Local).naive_local();
        let planned = planned_timestamp_for_name(name, &rules).expect("planned timestamp");

        assert_eq!(planned, expected_local);
        assert_eq!(offsets.nudge, 1);
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
    fn sanitize_preserves_custom_spacing_and_ordering() {
        let mut rules = TimestampRules {
            seconds_between_items: 3,
            slots_per_category: 32,
            categories: vec![CategoryRule {
                key: "DEFAULT".to_string(),
                aliases: Vec::new(),
            }],
        };
        rules.sanitize();
        assert_eq!(rules.seconds_between_items, 3);
        assert_eq!(rules.slots_per_category, 32);

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

        assert!(second_timestamp < first_timestamp);
    }

    #[test]
    fn sanitize_defaults_to_one_second() {
        let mut rules = TimestampRules {
            seconds_between_items: 0,
            slots_per_category: 10,
            categories: vec![CategoryRule {
                key: "DEFAULT".to_string(),
                aliases: Vec::new(),
            }],
        };

        rules.sanitize();

        assert_eq!(rules.seconds_between_items, 1);
        assert_eq!(rules.slots_per_category, 10);
    }
}
