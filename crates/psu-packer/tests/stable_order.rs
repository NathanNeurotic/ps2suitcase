use std::fs;
use std::path::Path;

use chrono::NaiveDate;
use ps2_filetypes::{PSUEntryKind, PSU};
use psu_packer::{pack_with_config, Config, IconSysConfig, IconSysFlags};
use tempfile::tempdir;

fn write_sample_files(dir: &Path) {
    fs::write(dir.join("B.DAT"), b"second").expect("write B.DAT");
    fs::write(dir.join("A.DAT"), b"first").expect("write A.DAT");
}

fn file_entry_names(archive: &PSU) -> Vec<String> {
    archive
        .entries
        .iter()
        .filter_map(|entry| {
            if matches!(entry.kind, PSUEntryKind::File) {
                Some(entry.name.clone())
            } else {
                None
            }
        })
        .collect()
}

fn build_icon_config() -> IconSysConfig {
    IconSysConfig {
        flags: IconSysFlags::new(0),
        title: "Example Save".to_string(),
        linebreak_pos: None,
        preset: None,
        background_transparency: None,
        background_colors: None,
        light_directions: None,
        light_colors: None,
        ambient_color: None,
    }
}

#[test]
fn packing_same_directory_twice_is_stable() {
    let tempdir = tempdir().expect("temp dir");
    let project = tempdir.path();
    write_sample_files(project);
    let output_dir = project.join("output");
    fs::create_dir(&output_dir).expect("create output dir");

    let timestamp = NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let output_first = output_dir.join("first.psu");
    let config_first = Config {
        name: "Stable Save".to_string(),
        timestamp: Some(timestamp),
        include: None,
        exclude: None,
        icon_sys: Some(build_icon_config()),
    };
    pack_with_config(project, &output_first, config_first).expect("first pack succeeds");

    let output_second = output_dir.join("second.psu");
    let config_second = Config {
        name: "Stable Save".to_string(),
        timestamp: Some(timestamp),
        include: None,
        exclude: None,
        icon_sys: Some(build_icon_config()),
    };
    pack_with_config(project, &output_second, config_second).expect("second pack succeeds");

    let first_bytes = fs::read(&output_first).expect("read first output");
    let second_bytes = fs::read(&output_second).expect("read second output");

    let first_archive = PSU::new(first_bytes.clone());
    let second_archive = PSU::new(second_bytes.clone());

    let first_names = file_entry_names(&first_archive);
    let second_names = file_entry_names(&second_archive);

    assert_eq!(
        first_names, second_names,
        "repeated runs should preserve entry ordering",
    );
    assert_eq!(
        first_names,
        vec!["A.DAT", "B.DAT", "icon.sys"],
        "files should be ordered case-insensitively",
    );
    assert_eq!(
        first_bytes, second_bytes,
        "packing should produce identical archives"
    );
}
