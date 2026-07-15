<template>
  <!-- The aa-touch window is a transparent, chrome-less overlay just for capturing touch input
       over the Android Auto video surface — it must never render the app shell (Vuetify's
       v-app background isn't transparent, which would hide the video underneath it). -->
  <RouterView v-if="isTouchWindow" />
  <v-app v-else>
    <TopBar v-if="!androidAutoActive" />
    <v-main>
      <RouterView />
    </v-main>
    <BottomBar v-if="!androidAutoActive" />
  </v-app>
</template>

<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed, onBeforeMount } from "vue";
import { useStatusStore } from "./store/statusStore";
import BottomBar from "./components/BottomBar.vue";
import TopBar from "./components/TopBar.vue";

const statusStore = useStatusStore();

const isTouchWindow = getCurrentWindow().label === "aa-touch";

// Hidden only while actively projecting video. "host-ui" (phone kicked back to its home
// screen) shows the sidebar again alongside the resume button.
const androidAutoActive = computed(() => statusStore.aaStatus === "connected");

onBeforeMount(async () => {
  const rtlSdrDetected = await invoke<boolean>("plugin:rtl-sdr|detect_rtl_sdr");
  statusStore.setRtlSdrDetected(rtlSdrDetected);

  listen("usb-event", async () => {
    const rtlSdrDetected = await invoke<boolean>(
      "plugin:rtl-sdr|detect_rtl_sdr",
    );
    statusStore.setRtlSdrDetected(rtlSdrDetected);
  });

  listen<"disconnected" | "connected" | "host-ui">("aa-status", (event) => {
    statusStore.setAaStatus(event.payload);
  });
});
</script>
