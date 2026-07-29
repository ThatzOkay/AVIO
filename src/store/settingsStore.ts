import { invoke } from "@tauri-apps/api/core";
import { defineStore } from "pinia";
import { useTheme } from "vuetify";

export const useSettingsStore = defineStore("settings", {
  state: () => ({
    masterVolume: 0,
    defaultSoundDevice: "",
    defaultInputDevice: "",
    audioDevices: [] as string[],
    inputDevices: [] as string[],
    brightness: 0,
    theme: "m3dark" as "m3dark" | "m3light",
    scale: 0,
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
    setMasterVolume(value: number) {
      const actualVolume = Math.round(value * 0.65);
      if (value === 0) {
        invoke("set_current_volume", { volume: 0 });
        invoke("set_mute", { mute: true });
        this.masterVolume = value;
        return;
      }

      invoke("set_mute", { mute: false });

      if (value === 100) {
        invoke("set_current_volume", { volume: 65 });
        this.masterVolume = value;
        return;
      }

      invoke("set_current_volume", { volume: actualVolume });
      this.masterVolume = value;
    },
    setDefaultSoundDevice(value: string) {
      invoke("set_default_device", { deviceName: value });
      this.defaultSoundDevice = value;
    },
    setDefaultInputDevice(value: string) {
      invoke("set_default_input_device", { deviceName: value });
      this.defaultInputDevice = value;
    },
    setBrightness(value: number) {
      invoke("set_brightness", { value: Number(value.toFixed(0)) });
      this.brightness = value;
    },
    setTheme(value: "m3dark" | "m3light") {
      const theme = useTheme();
      theme.global.name.value = value;
      this.theme = value;
    },
    setScale(value: number) {
      const actualScale = 1 + value / 100; // 0=1.0, 100=2.0

      document.body.style.transform = `scale(${actualScale})`;
      document.body.style.transformOrigin = "top left";

      document.body.style.width = `${100 / actualScale}%`;
      document.body.style.height = `${100 / actualScale}%`;
      this.scale = value;
    },
  },
});
