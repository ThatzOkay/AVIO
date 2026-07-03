use std::sync::Arc;

use evno::{Bus, Guard, from_fn};
use serde::{Serialize, Deserialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;

use crate::radio::fm_radio_service::{FMRadioService, RadioEvent, StationInfo};


const PERSIST_DEBOUNCE_MS: u64 = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RadioMode {
    FM,
    DAB
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioState {
    running: bool,
    frequency: u32,
    mode: RadioMode,
    station: Option<StationInfo>,
    favorites: Option<Vec<u32>>
}

pub struct RadioService {
    pub mode: RadioMode,
    persist_timer: Option<tokio::task::JoinHandle<()>>,
    fm: FMRadioService,
    app: tauri::AppHandle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DabStation {
    id: u32,
    label: String,
    channel: u32,
    frequency: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioConfig {
    pub last_frequency: u32,
    pub last_mode: RadioMode,
    pub favorites: Option<Vec<u32>>,
    pub dab_favorites: Option<Vec<DabStation>>,
    pub last_dab_station: Option<DabStation>,
}

impl RadioService {
    pub fn new(app: &AppHandle) -> Self {
        Self {
            mode: RadioMode::FM,
            fm: FMRadioService::new(app.clone()),
            persist_timer: None,
            app: app.clone()
        }
    }

    pub fn init(self: &mut Self) {
        let store = match self.app.store("config.json") {
            Ok(store) => store,
            Err(e) => {
                eprintln!("Failed to open config store: {}", e);
                return;
            }
        };

        let bus = self.app.state::<Bus>();

        let radio = store.get("radio")
            .and_then(|v| serde_json::from_value::<RadioConfig>(v).ok());

        self.mode = radio.clone().unwrap_or(RadioConfig {
            last_frequency: 0,
            last_mode: RadioMode::FM,
            favorites: None,
            dab_favorites: None,
            last_dab_station: None
        }).last_mode;
        self.fm.init(radio.clone());

        let app = self.app.clone();
        bus.on(from_fn(move |_event: Guard<RadioEvent>| {
            println!("Radio event");
            let app = app.clone();
            async move {
                let radio_service = app.state::<Arc<Mutex<RadioService>>>();
                let mut radio_service = radio_service.lock().await;
                radio_service.schedule_persist();
                radio_service.broadcast_fm_state();
            }
        }));

        store.close_resource();
    }

    pub fn get_fm_state(&self) -> RadioState {
        RadioState {
            running: self.fm.running,
            frequency: self.fm.frequency,
            mode: RadioMode::FM,
            station: self.fm.station_info.lock().unwrap().clone(),
            favorites: self.fm.favorites.clone()
        }
    }

    fn broadcast_fm_state(&self) {
        let state = self.get_fm_state();
        let app = self.app.clone();
        app.emit("fm-state", state);
    }

    pub fn set_mode(&mut self, mode: RadioMode) {
        self.mode = mode;
    }
    
    // -- FM --
    pub async fn start_fm(&mut self, frequency: u32) -> RadioState {
        self.fm.start(frequency).await;
        self.get_fm_state()
    }

    pub async fn stop_fm(&mut self) -> RadioState {
        self.fm.stop().await;
        self.get_fm_state()
    }

    pub async fn set_fm_frequency(&mut self, frequency: u32) -> RadioState {
        self.fm.set_frequency(frequency).await;
        self.get_fm_state()
    }

    pub async fn step_fm(&mut self, direction: i32, fast: bool) -> RadioState {
        self.fm.step(direction, fast).await;
        self.get_fm_state()
    }

    pub async fn set_fm_favorite(&mut self, slot: usize) -> RadioState {
        self.fm.set_favorite(slot).await;
        self.persist_now().await;
        self.get_fm_state()
    }

    pub async fn recall_fm_favorite(&mut self, slot: usize) -> RadioState {
        self.fm.recall_favorite(slot).await;
        self.get_fm_state()
    }

    fn build_radio_config(&self) -> RadioConfig {
        let fm = self.fm.get_state();

        RadioConfig {
            last_frequency: fm.frequency,
            last_mode: self.mode.clone(),
            favorites: fm.favorites.clone(),
            dab_favorites: None,
            last_dab_station: None
        }
    }

    fn schedule_persist(&mut self) {
        if let Some(timer) = self.persist_timer.take() {
            timer.abort();
        }

        let app = self.app.clone();
        let radio_config = self.build_radio_config();

        self.persist_timer = Some(tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(PERSIST_DEBOUNCE_MS)).await;
            Self::write_radio_config(&app, radio_config);
        }));
    }

    async fn persist_now(&mut self)  {
        if let Some(timer) = self.persist_timer.take() {
            timer.abort();
        }

        let radio_config = self.build_radio_config();
        Self::write_radio_config(&self.app, radio_config);
    }

    fn write_radio_config(app: &AppHandle, radio_config: RadioConfig) {
        println!("Writing radio config: {:?}", radio_config);
        let store = match app.store("config.json") {
            Ok(store) => store,
            Err(e) => {
                eprintln!("Failed to open config store: {}", e);
                return;
            }
        };

        match serde_json::to_value(&radio_config) {
            Ok(value) => store.set("radio", value),
            Err(e) => eprintln!("Failed to serialize radio config: {}", e),
        }

        store.close_resource();
    }

}