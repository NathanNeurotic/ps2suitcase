use {
    std::{env, io},
    winresource::WindowsResource,
};

fn main() -> io::Result<()> {
    println!("cargo:rustc-check-cfg=cfg(feature, values(\"eframe/glow\"))");

    if env::var_os("CARGO_FEATURE_GLOW").is_some() {
        println!("cargo:rustc-cfg=feature=\"eframe/glow\"");
    }

    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new()
            .set_icon("../suitcase/assets/icon.ico")
            .compile()?;
    }
    Ok(())
}
