#[link(name = "rtlsdr")]
unsafe extern "C" {
    fn rtlsdr_get_device_count() -> u32;
}

#[tauri::command]
pub fn detect_rtl_sdr() -> bool {
    unsafe { rtlsdr_get_device_count() > 0 }
}
