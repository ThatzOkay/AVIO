use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use evno::{Bus, Emit};
use serde::{Deserialize, Serialize};
use tauri::Manager;
use tauri_plugin_rtl_sdr::fm::fm_radio::{
    fm_close, fm_get_rds, fm_open, fm_read, fm_set_frequency, fm_set_gain, fm_set_sample_rate,
    fm_stop, RdsInfo,
};
use tokio::sync::mpsc;

use crate::{
    audio::audio_output::{AudioOutput, AudioOutputMode, AudioOutputOptions},
    radio::radio_service::RadioConfig,
};

pub const DEFAULT_FREQUENCY_KHZ: u32 = 100_000;
const FAVORITES_SLOTS: usize = 5;

const SAMPLE_RATE: u32 = 2048000;
const OUTPUT_RATE: u32 = 48000;
pub const FM_BAND_MIN_KHZ: u32 = 87_000;
pub const FM_BAND_MAX_KHZ: u32 = 108_000;
const FM_STEP_KHZ: u32 = 50;
const FM_FAST_STEP_KHZ: u32 = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationInfo {
    program_id: u16,
    genre: String,
    name: Option<String>,
    text: Option<String>,
}

pub struct RadioEvent {
    #[allow(dead_code)]
    event_type: String,
    #[allow(dead_code)]
    message: Option<String>,
}

pub struct FMRadioService {
    device_open: bool,
    pub running: bool,
    pub frequency: u32,
    audio_output: Option<Arc<tokio::sync::Mutex<AudioOutput>>>,
    // Shared with the background task draining the FFI read callback, which
    // updates this from outside any &mut self call.
    pub station_info: Arc<Mutex<Option<StationInfo>>>,
    pub favorites: Option<Vec<u32>>,
    app: tauri::AppHandle,
}

pub struct FMState {
    pub running: bool,
    pub frequency: u32,
    pub station_info: Option<StationInfo>,
    pub favorites: Option<Vec<u32>>,
}

impl FMRadioService {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            device_open: false,
            running: false,
            frequency: DEFAULT_FREQUENCY_KHZ,
            audio_output: None,
            station_info: Arc::new(Mutex::new(None)),
            favorites: None,
            app,
        }
    }

    pub fn init(&mut self, radio: Option<RadioConfig>) {
        if radio.is_none() {
            return;
        }
        let radio_config = radio.unwrap();
        self.frequency = self.clamp_frequency(radio_config.last_frequency);
        self.favorites = radio_config.favorites;
    }

    pub fn get_state(&self) -> FMState {
        FMState {
            running: self.running,
            frequency: self.frequency,
            station_info: self.station_info.lock().unwrap().clone(),
            favorites: self.favorites.clone(),
        }
    }

    pub async fn set_favorite(&mut self, slot: usize) {
        if slot < FAVORITES_SLOTS {
            self.favorites
                .get_or_insert_with(|| vec![0; FAVORITES_SLOTS])[slot] = self.frequency;
            self.app
                .state::<Bus>()
                .emit(RadioEvent {
                    event_type: "change".to_string(),
                    message: None,
                })
                .await;
        }
    }

    pub async fn recall_favorite(&mut self, slot: usize) {
        if let Some(favorites) = &self.favorites {
            let freq = favorites
                .get(slot)
                .copied()
                .unwrap_or(DEFAULT_FREQUENCY_KHZ);
            if !self.running {
                self.start(freq).await;
            } else {
                self.tune(self.clamp_frequency(freq)).await;
            }
        }
    }

    pub async fn start(&mut self, frequency: u32) {
        let freq = self.clamp_frequency(frequency);

        if self.running {
            self.tune(freq).await;
            return;
        }

        if !self.device_open {
            let open_result = fm_open(0).await.map_err(|e| e.to_string());
            if let Err(e) = open_result {
                eprintln!("Failed to open device: {e}");
                return;
            }
            self.device_open = true;
            fm_set_sample_rate(SAMPLE_RATE).await;
            fm_set_gain(200).await;
        }

        fm_set_frequency(freq * 1000).await;

        let options = AudioOutputOptions {
            sample_rate: OUTPUT_RATE as i32,
            channels: 1,
            mode: Some(AudioOutputMode::Music),
            device: None,
        };
        let mut audio_output = AudioOutput::new(options);
        audio_output.start(&self.app.clone()).await;
        let audio_output = Arc::new(tokio::sync::Mutex::new(audio_output));
        self.audio_output = Some(audio_output.clone());

        // rtlsdr_read_async invokes this callback synchronously from its own
        // dedicated thread; blocking it on the gstreamer stdin write or a Bus
        // emit would stall USB buffer draining and drop samples. So it only
        // forwards the buffer over a channel, and a spawned task does the
        // actual (async) writing and RDS bookkeeping.
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<f32>>();
        let send_failed_once = Arc::new(AtomicBool::new(false));
        let send_failed_once_cb = send_failed_once.clone();
        if let Err(e) = fm_read(
            Box::new(move |buff: Vec<f32>| {
                if tx.send(buff).is_err() && !send_failed_once_cb.swap(true, Ordering::Relaxed) {
                    eprintln!("FM audio channel send failed (receiver dropped)");
                }
            }),
            OUTPUT_RATE,
        )
        .await
        {
            eprintln!("Failed to start FM read: {}", e);
            return;
        }

        let station_info = self.station_info.clone();
        let app = self.app.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(buff) = rx.recv().await {
                audio_output.lock().await.write(pcm_from_f32(&buff)).await;

                let station = to_station_info(fm_get_rds());
                let changed = {
                    let mut current = station_info.lock().unwrap();
                    if !station_info_equal(&current, &station) {
                        *current = station;
                        true
                    } else {
                        false
                    }
                };

                if changed {
                    app.state::<Bus>()
                        .emit(RadioEvent {
                            event_type: "change".to_string(),
                            message: None,
                        })
                        .await;
                }
            }
        });

        self.running = true;
        self.app
            .state::<Bus>()
            .emit(RadioEvent {
                event_type: "change".to_string(),
                message: None,
            })
            .await;
    }

    pub async fn stop(&mut self) {
        if !self.running {
            return;
        }

        if self.device_open {
            fm_stop().await;
            fm_close().await;
            self.device_open = false;
        }

        self.running = false;
        *self.station_info.lock().unwrap() = None;
        if let Some(audio_output) = self.audio_output.take() {
            audio_output.lock().await.stop().await;
        }

        self.app
            .state::<Bus>()
            .emit(RadioEvent {
                event_type: "change".to_string(),
                message: None,
            })
            .await;
    }

    pub async fn set_frequency(&mut self, frequency: u32) {
        self.tune(self.clamp_frequency(frequency)).await;
    }

    pub async fn step(&mut self, direction: i32, fast: bool) {
        let step_size = if fast { FM_FAST_STEP_KHZ } else { FM_STEP_KHZ } as i64;
        let delta = step_size * direction as i64;
        let next = (self.frequency as i64 + delta).max(0) as u32;
        self.tune(self.clamp_frequency(next)).await;
    }

    async fn tune(&mut self, frequency: u32) {
        self.frequency = frequency;
        if self.running && self.device_open {
            fm_set_frequency(frequency * 1000).await;
            *self.station_info.lock().unwrap() = None;
        }

        self.app
            .state::<Bus>()
            .emit(RadioEvent {
                event_type: "change".to_string(),
                message: None,
            })
            .await;
    }

    pub fn clamp_frequency(&self, khz: u32) -> u32 {
        khz.clamp(FM_BAND_MIN_KHZ, FM_BAND_MAX_KHZ)
    }
}

fn to_station_info(rds: RdsInfo) -> Option<StationInfo> {
    if rds.program_id == 0 && rds.program_type == "0" {
        return None;
    }

    Some(StationInfo {
        program_id: rds.program_id as u16,
        genre: rds.program_type,
        name: rds.station_name,
        text: rds.radio_text,
    })
}

fn station_info_equal(a: &Option<StationInfo>, b: &Option<StationInfo>) -> bool {
    match (a, b) {
        (Some(a_info), Some(b_info)) => {
            a_info.program_id == b_info.program_id
                && a_info.genre == b_info.genre
                && a_info.name == b_info.name
                && a_info.text == b_info.text
        }
        (None, None) => true,
        _ => false,
    }
}

fn pcm_from_f32(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}
