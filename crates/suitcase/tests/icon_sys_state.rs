use icon_sys_ui::IconSysState;
use ps2_filetypes::{color::Color, ColorF, IconSys, Vector};
use psu_packer::IconSysConfig;
use suitcase::{AppState, IconSysViewer, VirtualFile};

#[test]
fn viewer_initial_state_matches_shared_state_conversion() {
    let icon_sys = IconSys {
        flags: 5,
        linebreak_pos: IconSysConfig::default_linebreak_pos(),
        background_transparency: 64,
        background_colors: IconSysConfig::default_background_colors().map(|color| Color {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }),
        light_directions: IconSysConfig::default_light_directions().map(|direction| Vector {
            x: direction.x,
            y: direction.y,
            z: direction.z,
            w: direction.w,
        }),
        light_colors: IconSysConfig::default_light_colors().map(|color| ColorF {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }),
        ambient_color: IconSysConfig::default_ambient_color().into(),
        title: String::from("TESTTITLE"),
        icon_file: String::from("list.icn"),
        icon_copy_file: String::from("copy.icn"),
        icon_delete_file: String::from("del.icn"),
    };

    let bytes = icon_sys.to_bytes().expect("failed to serialize icon.sys");
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let icon_sys_path = temp_dir.path().join("icon.sys");
    std::fs::write(&icon_sys_path, &bytes).expect("failed to write icon.sys");

    let virtual_file = VirtualFile {
        name: "icon.sys".into(),
        file_path: icon_sys_path.clone(),
        size: bytes.len() as u64,
    };

    let mut app_state = AppState::new();
    app_state.opened_folder = Some(temp_dir.path().to_path_buf());

    let viewer = IconSysViewer::new(&virtual_file, &app_state);
    let mut expected_state = IconSysState::from_icon_sys(&icon_sys);
    expected_state.update_detected_preset();

    assert_eq!(viewer.icon_state, expected_state);
}
