use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use chrono::NaiveDateTime;
use eframe::egui;
use eframe::App;
use gui_core::actions::{
    Action, IconSysAction, MetadataAction, TimestampAction, TimestampStrategyAction,
};
use gui_core::ActionDispatcher;
use ps2_filetypes::{IconSys, PSUEntry, PSUEntryKind, PSU};
use psu_packer::{shift_jis_byte_length, IconSysConfig, IconSysPreset, ICON_SYS_PRESETS};
use psu_packer_gui::ui::theme;
use psu_packer_gui::{PackerApp, SasPrefix, TIMESTAMP_FORMAT};
use tempfile::tempdir;

const MAX_PACK_FRAMES: usize = 200;

#[test]
fn unified_pack_flow_applies_preset_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = tempdir()?;
    let project_dir = workspace.path().join("project");
    write_project_fixture(&project_dir)?;

    let (mut app, ctx, mut frame) = new_app_harness();
    app.load_project_from_path(&project_dir);

    dispatch_action(
        &mut app,
        Action::Metadata(MetadataAction::SelectPrefix(SasPrefix::App)),
    );
    dispatch_action(
        &mut app,
        Action::Metadata(MetadataAction::SetFolderBaseName("COOLSAVE".to_string())),
    );
    dispatch_action(
        &mut app,
        Action::Metadata(MetadataAction::SetPsuFileBaseName("cool_flow".to_string())),
    );

    let timestamp = NaiveDateTime::parse_from_str("2024-03-15 10:45:00", TIMESTAMP_FORMAT)?;
    dispatch_action(
        &mut app,
        Action::Timestamp(TimestampAction::SelectStrategy(
            TimestampStrategyAction::Manual,
        )),
    );
    dispatch_action(
        &mut app,
        Action::Timestamp(TimestampAction::SetManualTimestamp(Some(timestamp))),
    );

    let custom_icon = icon_sys_for_titles("COOL", "SAVE");
    app.apply_icon_sys_file(&custom_icon);
    dispatch_action(&mut app, Action::IconSys(IconSysAction::GenerateNew));
    dispatch_action(
        &mut app,
        Action::IconSys(IconSysAction::ApplyPreset("cool_blue".to_string())),
    );

    let preset = ICON_SYS_PRESETS
        .iter()
        .find(|preset| preset.id == "cool_blue")
        .expect("cool_blue preset available");
    assert_eq!(
        app.icon_sys_state().selected_preset.as_deref(),
        Some(preset.id)
    );

    let output_path = workspace.path().join("out").join("cool_flow.psu");
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    app.set_output_destination(&output_path);
    assert!(app.is_action_enabled(Action::PackPsu));

    dispatch_action(&mut app, Action::PackPsu);
    wait_for_pack_completion(&mut app, &ctx, &mut frame);

    assert!(output_path.exists(), "packed PSU should be created");

    let psu = read_psu(&output_path);
    let root_directory = find_root_directory(&psu).expect("root directory entry");
    assert_eq!(root_directory.name, "APP_COOLSAVE");

    let icon_bytes = read_psu_entry_bytes(&psu, "icon.sys").expect("icon.sys entry");
    let icon = IconSys::new(icon_bytes);
    assert_eq!(icon.title, "COOLSAVE");
    assert_eq!(
        icon.linebreak_pos,
        shift_jis_byte_length("COOL").expect("encode title") as u16
    );
    assert_eq!(icon.flags, custom_icon.flags);
    assert_icon_matches_preset(&icon, preset);

    let title_bytes = read_psu_entry_bytes(&psu, "title.cfg").expect("title.cfg entry");
    let title_text = String::from_utf8(title_bytes)?;
    assert!(title_text.contains("Example Game"));
    assert!(title_text.contains("Release=2024"));

    Ok(())
}

fn new_app_harness() -> (PackerApp, egui::Context, eframe::Frame) {
    let ctx = egui::Context::default();
    theme::install(&ctx, &theme::Palette::default());
    let creation = eframe::CreationContext::_new_kittest(ctx.clone());
    let app = PackerApp::new(&creation);
    let frame = eframe::Frame::_new_kittest();
    (app, ctx, frame)
}

fn dispatch_action(app: &mut PackerApp, action: Action) {
    assert!(app.supports_action(action.clone()));
    assert!(app.is_action_enabled(action.clone()));
    app.trigger_action(action);
}

fn pump_frame(app: &mut PackerApp, ctx: &egui::Context, frame: &mut eframe::Frame) {
    ctx.begin_frame(egui::RawInput::default());
    app.update(ctx, frame);
    let _ = ctx.end_frame();
}

fn wait_for_pack_completion(app: &mut PackerApp, ctx: &egui::Context, frame: &mut eframe::Frame) {
    for _ in 0..MAX_PACK_FRAMES {
        pump_frame(app, ctx, frame);
        if app.is_action_enabled(Action::PackPsu) {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }

    panic!("pack job did not complete in time");
}

fn write_project_fixture(project_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(project_root)?;
    fs::write(project_root.join("list.icn"), b"LISTDATA")?;
    fs::write(project_root.join("copy.icn"), b"COPYDATA")?;
    fs::write(project_root.join("del.icn"), b"DELDATA")?;

    let title_template = load_title_template();
    fs::write(project_root.join("title.cfg"), title_template)?;

    let icon = icon_sys_for_titles("DEFAULT", "SAVE");
    fs::write(project_root.join("icon.sys"), icon.to_bytes()?)?;

    Ok(())
}

fn load_title_template() -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|dir| dir.parent())
        .expect("workspace root");
    let template_path = workspace_root
        .join("assets")
        .join("templates")
        .join("title.cfg");
    fs::read_to_string(template_path).expect("load title.cfg template")
}

fn icon_sys_for_titles(line1: &str, line2: &str) -> IconSys {
    let linebreak = shift_jis_byte_length(line1).expect("encode icon.sys title") as u16;
    let combined = format!("{line1}{line2}");
    IconSys {
        flags: 1,
        linebreak_pos: linebreak,
        background_transparency: IconSysConfig::default_background_transparency(),
        background_colors: IconSysConfig::default_background_colors().map(Into::into),
        light_directions: IconSysConfig::default_light_directions().map(Into::into),
        light_colors: IconSysConfig::default_light_colors().map(Into::into),
        ambient_color: IconSysConfig::default_ambient_color().into(),
        title: combined,
        icon_file: "list.icn".to_string(),
        icon_copy_file: "copy.icn".to_string(),
        icon_delete_file: "del.icn".to_string(),
    }
}

fn read_psu(path: &Path) -> PSU {
    let data = fs::read(path).expect("read PSU output");
    PSU::new(data)
}

fn read_psu_entry_bytes(psu: &PSU, name: &str) -> Option<Vec<u8>> {
    psu.entries()
        .into_iter()
        .find(|entry| entry.name.eq_ignore_ascii_case(name))
        .and_then(|entry| entry.contents)
}

fn find_root_directory(psu: &PSU) -> Option<PSUEntry> {
    psu.entries().into_iter().find(|entry| {
        matches!(entry.kind, PSUEntryKind::Directory) && entry.name != "." && entry.name != ".."
    })
}

fn assert_icon_matches_preset(icon: &IconSys, preset: &IconSysPreset) {
    assert_eq!(icon.background_transparency, preset.background_transparency);

    for (actual, expected) in icon
        .background_colors
        .iter()
        .zip(preset.background_colors.iter())
    {
        assert_eq!(actual.r, expected.r);
        assert_eq!(actual.g, expected.g);
        assert_eq!(actual.b, expected.b);
        assert_eq!(actual.a, expected.a);
    }

    for (actual, expected) in icon
        .light_directions
        .iter()
        .zip(preset.light_directions.iter())
    {
        assert!(approx_eq(actual.x, expected.x));
        assert!(approx_eq(actual.y, expected.y));
        assert!(approx_eq(actual.z, expected.z));
        assert!(approx_eq(actual.w, expected.w));
    }

    for (actual, expected) in icon.light_colors.iter().zip(preset.light_colors.iter()) {
        assert!(approx_eq(actual.r, expected.r));
        assert!(approx_eq(actual.g, expected.g));
        assert!(approx_eq(actual.b, expected.b));
        assert!(approx_eq(actual.a, expected.a));
    }

    assert!(approx_eq(icon.ambient_color.r, preset.ambient_color.r));
    assert!(approx_eq(icon.ambient_color.g, preset.ambient_color.g));
    assert!(approx_eq(icon.ambient_color.b, preset.ambient_color.b));
    assert!(approx_eq(icon.ambient_color.a, preset.ambient_color.a));
}

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < f32::EPSILON * 16.0
}
