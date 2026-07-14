use std::env;
use std::path::Path;

const COMMANDS: &[&str] = &["detect_rtl_sdr"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();

    // rtl_sdr_detection.rs and fm_radio.rs link `rtlsdr` directly
    // (`#[link(name = "rtlsdr")]`). There's no vcpkg port and upstream ships no
    // Windows binaries, so a prebuilt DLL + a generated import lib are vendored
    // under vendor/windows-x64 (see the README there).
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let vendor_dir = Path::new(&manifest_dir).join("vendor/windows-x64");
        println!("cargo:rustc-link-search=native={}", vendor_dir.display());
    }
}
