use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub master_volume: u8,
    pub default_sound_device: String,
    pub default_input_device: String,
    pub brightness: u8,
    pub theme: String,
    pub seed: String,
    pub scale: i8,
    pub ao_resolution: String,
    pub ao_framerate: String,
    pub ao_dpi: f64,
    pub ao_view_area_top: u32,
    pub ao_view_area_bottom: u32,
    pub ao_view_area_left: u32,
    pub ao_view_area_right: u32,
    pub ao_safe_area_top: u32,
    pub ao_safe_area_bottom: u32,
    pub ao_safe_area_left: u32,
    pub ao_safe_area_right: u32,
    pub kiosk: bool,
    // "" = host/panel default. See `window::display_mode`.
    pub display_mode: String,
}

pub struct SettingsState(pub Mutex<AppSettings>);