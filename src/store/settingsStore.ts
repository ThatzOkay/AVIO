import { invoke } from "@tauri-apps/api/core";
import { defineStore } from "pinia";

export const useSettingsStore = defineStore("settings", {
  state: () => ({
    masterVolume: 0,
    defaultSoundDevice: "",
    defaultInputDevice: "",
    audioDevices: [] as string[],
    inputDevices: [] as string[],
    brightness: 0,
    theme: "m3dark" as "m3dark" | "m3light",
    seed: "#1B6EF3",
    scale: 0,
    aoResolution: "720" as "720" | "1080",
    aoFramerate: "60" as "30" | "60",
    aoDpi: 96.0,
    aoViewAreaTop: 0,
    aoViewAreaBottom: 0,
    aoViewAreaLeft: 0,
    aoViewAreaRight: 0,
    aoSafeAreaTop: 0,
    aoSafeAreaBottom: 0,
    aoSafeAreaLeft: 0,
    aoSafeAreaRight: 0,
    kiosk: false,
    displayMode: "",
    displayModes: [] as string[],
  }),
  actions: {
    init() {
      invoke<string[]>("get_audio_devices").then((devices: string[]) => {
        this.audioDevices = devices;
      });
      invoke<string[]>("get_input_devices").then((devices: string[]) => {
        this.inputDevices = devices;
      });
      invoke<number>("get_current_volume").then((volume: number) => {
        this.masterVolume = volume;
      });
      invoke<string>("get_default_device_name").then((device: string) => {
        this.defaultSoundDevice = device;
      });
      invoke<string>("get_default_input_device_name").then((device: string) => {
        this.defaultInputDevice = device;
      });
      invoke<number>("get_current_brightness").then((brightness: number) => {
        this.brightness = brightness;
      });
    },
    initDevices() {
      invoke<string[]>("get_audio_devices").then((devices: string[]) => {
        this.audioDevices = devices;
      });
      invoke<string[]>("get_input_devices").then((devices: string[]) => {
        this.inputDevices = devices;
      });
    },
    initDisplayModes() {
      invoke<string[]>("list_display_modes").then((modes: string[]) => {
        console.log(modes);
        this.displayModes = modes;
      });
    },
    setMasterVolume(value: number) {
      const actualVolume = Math.round(value * 0.65);
      if (value === 0) {
        invoke("set_current_volume", { volume: 0 });
        invoke("set_mute", { mute: true });
        this.masterVolume = value;
        saveSettings();
        return;
      }

      invoke("set_mute", { mute: false });

      if (value === 100) {
        invoke("set_current_volume", { volume: 65 });
        this.masterVolume = value;
        saveSettings();
        return;
      }

      invoke("set_current_volume", { volume: actualVolume });
      this.masterVolume = value;
      saveSettings();
    },
    setDefaultSoundDevice(value: string) {
      invoke("set_default_device", { deviceName: value });
      this.defaultSoundDevice = value;
      saveSettings();
    },
    setDefaultInputDevice(value: string) {
      invoke("set_default_input_device", { deviceName: value });
      this.defaultInputDevice = value;
      saveSettings();
    },
    setBrightness(value: number) {
      invoke("set_brightness", { value: Number(value.toFixed(0)) });
      this.brightness = value;
      saveSettings();
    },
    setTheme(value: "m3dark" | "m3light") {
      this.theme = value;
      saveSettings();
    },
    setSeed(value: string) {
      this.seed = value;
      saveSettings();
    },
    setScale(value: number) {
      const actualScale = 1 + value / 100; // 0=1.0, 100=2.0

      // `zoom` (not `transform: scale`) - transform on body would make it the new containing
      // block for any `position: fixed` descendant (e.g. BottomBar's v-bottom-navigation),
      // detaching it from the real viewport instead of staying pinned to the bottom.
      document.body.style.zoom = actualScale.toString();
      this.scale = value;
      saveSettings();
    },
    setAoResolution(value: "720" | "1080") {
      this.aoResolution = value;
      saveSettings();
    },
    setAoFramerate(value: "30" | "60") {
      this.aoFramerate = value;
      saveSettings();
    },
    setAoDpi(value: number) {
      this.aoDpi = value;
      saveSettings();
    },
    setAoViewAreaTop(value: number) {
      this.aoViewAreaTop = value;
      saveSettings();
    },
    setAoViewAreaBottom(value: number) {
      this.aoViewAreaBottom = value;
      saveSettings();
    },
    setAoViewAreaLeft(value: number) {
      this.aoViewAreaLeft = value;
      saveSettings();
    },
    setAoViewAreaRight(value: number) {
      this.aoViewAreaRight = value;
      saveSettings();
    },
    setAoSafeAreaTop(value: number) {
      this.aoSafeAreaTop = value;
      saveSettings();
    },
    setAoSafeAreaBottom(value: number) {
      this.aoSafeAreaBottom = value;
      saveSettings();
    },
    setAoSafeAreaLeft(value: number) {
      this.aoSafeAreaLeft = value;
      saveSettings();
    },
    setAoSafeAreaRight(value: number) {
      this.aoSafeAreaRight = value;
      saveSettings();
    },
    setKiosk(value: boolean) {
      this.kiosk = value;
      saveSettings();
    },
    setDisplayMode(value: string) {
      this.displayMode = value;
      saveSettings();
    },
  },
});

let saveTimeout: ReturnType<typeof setTimeout> | undefined;

const saveSettings = () => {
  const settingsStore = useSettingsStore();
  clearTimeout(saveTimeout);
  saveTimeout = setTimeout(async () => {
    await invoke("save_settings", {
      settings: {
        masterVolume: settingsStore.masterVolume,
        defaultSoundDevice: settingsStore.defaultSoundDevice,
        defaultInputDevice: settingsStore.defaultInputDevice,
        brightness: settingsStore.brightness,
        theme: settingsStore.theme,
        seed: settingsStore.seed,
        scale: settingsStore.scale,
        aoResolution: settingsStore.aoResolution,
        aoFramerate: settingsStore.aoFramerate,
        aoDpi: settingsStore.aoDpi,
        aoViewAreaTop: settingsStore.aoViewAreaTop,
        aoViewAreaBottom: settingsStore.aoViewAreaBottom,
        aoViewAreaLeft: settingsStore.aoViewAreaLeft,
        aoViewAreaRight: settingsStore.aoViewAreaRight,
        aoSafeAreaTop: settingsStore.aoSafeAreaTop,
        aoSafeAreaBottom: settingsStore.aoSafeAreaBottom,
        aoSafeAreaLeft: settingsStore.aoSafeAreaLeft,
        aoSafeAreaRight: settingsStore.aoSafeAreaRight,
        kiosk: settingsStore.kiosk,
        displayMode: settingsStore.displayMode,
      },
    });
  }, 500);
};

export { saveSettings };
