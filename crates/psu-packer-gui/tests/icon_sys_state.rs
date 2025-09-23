use icon_sys_ui::IconSysState;
use ps2_filetypes::IconSys;
use psu_packer::IconSysConfig;

use psu_packer_gui::state::PackerApp;

#[test]
fn applying_icon_sys_file_matches_shared_state_conversion() {
    let icon_sys = IconSys {
        flags: 7,
        linebreak_pos: 0,
        background_transparency: 128,
        background_colors: IconSysConfig::default_background_colors().map(Into::into),
        light_directions: IconSysConfig::default_light_directions().map(Into::into),
        light_colors: IconSysConfig::default_light_colors().map(Into::into),
        ambient_color: IconSysConfig::default_ambient_color().into(),
        title: String::from("HELLOWORLD"),
        icon_file: String::from("list.icn"),
        icon_copy_file: String::from("copy.icn"),
        icon_delete_file: String::from("del.icn"),
    };

    let expected_state = IconSysState::from_icon_sys(&icon_sys);

    let mut app = PackerApp::default();
    app.apply_icon_sys_file(&icon_sys);

    assert_eq!(app.icon_sys_state(), &expected_state);
}
