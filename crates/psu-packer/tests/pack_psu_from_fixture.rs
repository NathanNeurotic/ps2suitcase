use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use ps2_filetypes::{PSUEntryKind, PSU};
use psu_packer::pack_psu;
use tempfile::tempdir;

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/psu.toml")
}

#[test]
fn pack_psu_consumes_repository_fixture() {
    let tempdir = tempdir().expect("temp dir");
    let project = tempdir.path();

    let config_destination = project.join("psu.toml");
    fs::copy(fixture_path(), &config_destination).expect("copy psu.toml fixture");
    fs::write(project.join("DATA.BIN"), b"payload").expect("write data file");

    let output = project.join("output.psu");
    pack_psu(project, &output).expect("pack psu using fixture config");

    let archive = PSU::new(fs::read(&output).expect("read packed archive"));

    assert!(
        archive.entries.iter().any(|entry| {
            matches!(entry.kind, PSUEntryKind::File) && entry.name.eq_ignore_ascii_case("DATA.BIN")
        }),
        "expected DATA.BIN to be included"
    );

    assert!(
        archive.entries.iter().all(|entry| {
            !matches!(entry.kind, PSUEntryKind::File)
                || !entry.name.eq_ignore_ascii_case("psu.toml")
        }),
        "psu.toml should not be packaged"
    );

    let expected_timestamp = NaiveDate::from_ymd_opt(2024, 10, 10)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();

    let root_entry = archive
        .entries
        .iter()
        .find(|entry| {
            matches!(entry.kind, PSUEntryKind::Directory) && entry.name != "." && entry.name != ".."
        })
        .expect("root directory entry present");

    assert_eq!(root_entry.name, "Test PSU");

    for entry in archive.entries.iter() {
        assert_eq!(
            entry.created, expected_timestamp,
            "created timestamp should match config for {}",
            entry.name
        );
        assert_eq!(
            entry.modified, expected_timestamp,
            "modified timestamp should match config for {}",
            entry.name
        );
    }
}
