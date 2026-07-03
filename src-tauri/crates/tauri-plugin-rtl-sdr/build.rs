const COMMANDS: &[&str] = &["detect_rtl_sdr"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
