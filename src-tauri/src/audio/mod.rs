use std::sync::OnceLock;

use cpvc::get_system_volume;
use regex::Regex;
use tauri::AppHandle;

use crate::audio::audio_device_enumerator::{list_audio_devices, AudioDevice, AudioDeviceType};

pub mod audio_device_enumerator;
pub mod audio_downsample;
pub mod audio_output;
pub mod gstreamer;
// Capture-side mic support, not yet wired to the AA MicChannel path.
#[allow(dead_code)]
pub mod microphone;

struct DedupedPerMac {
    deduped: Vec<AudioDevice>,
    #[allow(dead_code)]
    macs: std::collections::HashSet<String>,
}

// Bluetooth-audio dedup helpers, not yet wired into device enumeration.
#[allow(dead_code)]
const BT_COD_MAJOR_AUDIO: u8 = 0x04;
static BLUEZ_MAC_RE: OnceLock<Regex> = OnceLock::new();

#[allow(dead_code)]
fn is_bt_audio_cod(cod: u32) -> bool {
    let major_cod = ((cod >> 8) & 0x1F) as u8;
    major_cod == BT_COD_MAJOR_AUDIO
}

#[allow(dead_code)]
fn mac_to_bluez_id(mac: &str) -> Option<String> {
    mac.to_uppercase().replace(":", "_").into()
}

fn extract_bluez_mac(device_id: &str) -> Option<String> {
    let re = BLUEZ_MAC_RE.get_or_init(|| {
        Regex::new(r"^bluez_(?:output|input|sink|source)\.([0-9A-Fa-f_:]{17})").unwrap()
    });
    re.captures(device_id)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().replace('_', ":").to_uppercase())
}

fn dedupe_bt_per_mac(devices: Vec<AudioDevice>) -> DedupedPerMac {
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    for device in devices {
        let mac = extract_bluez_mac(&device.id);
        if let Some(mac) = mac {
            if seen.contains(&mac) {
                continue;
            }
            seen.insert(mac);
        }
        deduped.push(device);
    }
    DedupedPerMac {
        deduped,
        macs: seen,
    }
}

async fn mixed_audio_devices(
    app: &AppHandle,
    kind: AudioDeviceType,
) -> Result<Vec<AudioDevice>, String> {
    let local = list_audio_devices(app, kind).await?;
    let deduped = dedupe_bt_per_mac(local);
    Ok(deduped.deduped)
}

#[tauri::command]
pub async fn list_sinks(app: tauri::AppHandle) -> Result<Vec<AudioDevice>, String> {
    mixed_audio_devices(&app, AudioDeviceType::Sink).await
}

#[tauri::command]
pub async fn list_sources(app: tauri::AppHandle) -> Result<Vec<AudioDevice>, String> {
    mixed_audio_devices(&app, AudioDeviceType::Source).await
}

#[tauri::command]
pub fn get_current_volume() -> Result<u8, String> {
    Ok(get_system_volume())
}

#[tauri::command]
pub fn get_default_device_name() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        cpvc::wasapi::wasapi::get_sound_devices()
            .map_err(|e| format!("{e:?}"))?
            .into_iter()
            .next()
            .ok_or_else(|| "No default device found".to_string())
    }

    #[cfg(target_os = "macos")]
    {
        cpvc::coreaudio::coreaudio::get_sound_devices()
            .map_err(|e| format!("{e:?}"))?
            .into_iter()
            .next()
            .ok_or_else(|| "No default device found".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        cpvc::pulseaudio::pulseaudio::get_sound_devices()
            .map_err(|e| format!("{e:?}"))?
            .into_iter()
            .next()
            .ok_or_else(|| "No default device found".to_string())
    }
}

#[tauri::command]
pub fn set_current_volume(volume: u8) -> Result<bool, String> {
    Ok(cpvc::set_system_volume(volume))
}
