use std::sync::OnceLock;

use regex::Regex;
use tauri::AppHandle;

use crate::audio::audio_device_enumerator::{list_audio_devices, AudioDevice, AudioDeviceType};

pub mod audio_device_enumerator;
pub mod audio_downsample;
pub mod audio_output;
pub mod gstreamer;
pub mod microphone;

struct DedupedPerMac {
    deduped: Vec<AudioDevice>,
    macs: std::collections::HashSet<String>,
}

const BT_COD_MAJOR_AUDIO: u8 = 0x04;
static BLUEZ_MAC_RE: OnceLock<Regex> = OnceLock::new();

fn is_bt_audio_cod(cod: u32) -> bool {
    let major_cod = ((cod >> 8) & 0x1F) as u8;
    major_cod == BT_COD_MAJOR_AUDIO
}

fn mac_to_bluez_id(mac: &str) -> Option<String> {
    return mac.to_uppercase().replace(":", "_").into();
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
