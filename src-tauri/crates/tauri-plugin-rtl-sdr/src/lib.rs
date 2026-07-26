pub mod fm;
mod rtl_sdr_detection;

pub use rtl_sdr_detection::detect_rtl_sdr;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("rtl-sdr")
        .invoke_handler(tauri::generate_handler![rtl_sdr_detection::detect_rtl_sdr])
        .build()
}
