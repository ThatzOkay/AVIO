import { createApp } from "vue";
import App from "./App.vue";

import "@mdi/font/css/materialdesignicons.css";

import { injectM3CssVars } from "./composables/useM3Theme";
import { createAppVuetify } from "./plugins/vuetify";
import "vuetify/styles";
import { createPinia } from "pinia";
import router from "./router/index.ts";
import "@mdi/font/css/materialdesignicons.css";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "./store/settingsStore.ts";

const SEED = "#1B6EF3";

const pinia = createPinia();
const vuetify = createAppVuetify(SEED);

const app = createApp(App).use(router).use(vuetify).use(pinia);

injectM3CssVars(SEED, true);

interface StoredSettings {
  masterVolume: number;
  defaultSoundDevice: string;
  defaultInputDevice: string;
  brightness: number;
  theme: "m3dark" | "m3light";
  seed: string;
  scale: number;
  aoResolution: "720" | "1080";
  aoFramerate: "30" | "60";
  aoDpi: number;
  aoViewAreaTop: number;
  aoViewAreaBottom: number;
  aoViewAreaLeft: number;
  aoViewAreaRight: number;
  aoSafeAreaTop: number;
  aoSafeAreaBottom: number;
  aoSafeAreaLeft: number;
  aoSafeAreaRight: number;
  kiosk: boolean;
  displayMode: string;
}

router.isReady().then(async () => {
  const settingsStore = useSettingsStore();
  const settings = await invoke<StoredSettings>("get_settings");
  console.log(settings);
  settingsStore.setMasterVolume(settings.masterVolume || 50);
  settingsStore.setDefaultSoundDevice(settings.defaultSoundDevice || "");
  settingsStore.setDefaultInputDevice(settings.defaultInputDevice || "");
  settingsStore.setBrightness(settings.brightness || 100);
  settingsStore.setTheme(settings.theme || "m3light");
  settingsStore.setSeed(settings.seed || SEED);
  settingsStore.setScale(settings.scale || 0);
  settingsStore.setAoResolution(settings.aoResolution || "720");
  settingsStore.setAoFramerate(settings.aoFramerate || "60");
  settingsStore.setAoDpi(settings.aoDpi || 96.0);
  settingsStore.setAoViewAreaTop(settings.aoViewAreaTop || 0);
  settingsStore.setAoViewAreaBottom(settings.aoViewAreaBottom || 0);
  settingsStore.setAoViewAreaLeft(settings.aoViewAreaLeft || 0);
  settingsStore.setAoViewAreaRight(settings.aoViewAreaRight || 0);
  settingsStore.setAoSafeAreaTop(settings.aoSafeAreaTop || 0);
  settingsStore.setAoSafeAreaBottom(settings.aoSafeAreaBottom || 0);
  settingsStore.setAoSafeAreaLeft(settings.aoSafeAreaLeft || 0);
  settingsStore.setAoSafeAreaRight(settings.aoSafeAreaRight || 0);
  settingsStore.setKiosk(settings.kiosk || false);
  settingsStore.setDisplayMode(settings.displayMode || "");
  app.use(vuetify);
  app.mount("#app");
});
