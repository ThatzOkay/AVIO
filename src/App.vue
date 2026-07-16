<template>
  <!-- The aa-touch window is a transparent, chrome-less overlay just for capturing touch input
       over the Android Auto video surface — it must never render the app shell (Vuetify's
       v-app background isn't transparent, which would hide the video underneath it). -->
  <RouterView v-if="isTouchWindow" />
  <v-app v-else>
    <TopBar v-if="!androidAutoActive" />
    <v-main>
      <router-view v-slot="{ Component }" class="inline">
        <transition :name="transitionName">
          <component :is="Component" />
        </transition>
      </router-view>
    </v-main>
    <BottomBar v-if="!androidAutoActive" />
  </v-app>
</template>

<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed, onBeforeMount, ref } from "vue";
import { useStatusStore } from "./store/statusStore";
import BottomBar from "./components/BottomBar.vue";
import TopBar from "./components/TopBar.vue";
import { useRouter } from "vue-router";

const statusStore = useStatusStore();
const router = useRouter();
const isTouchWindow = getCurrentWindow().label === "aa-touch";

// Hidden only while actively projecting video. "host-ui" (phone kicked back to its home
// screen) shows the sidebar again alongside the resume button.
const androidAutoActive = computed(() => statusStore.aaStatus === "connected");

const transitionName = ref("slide-left");

router.afterEach((to, from) => {
  const toIndex = (to.meta.index as number) || 0;
  const fromIndex = (from.meta.index as number) || 0;

  transitionName.value = toIndex > fromIndex ? "slide-right" : "slide-left";
});

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

<style scoped>
.v-main {
  position: relative;
  overflow: hidden;
}

:deep(.inline) {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
}

.slide-left-enter-active,
.slide-right-leave-active {
  transition: all 0.3s ease;
}
.slide-left-leave-active,
.slide-right-enter-active {
  transition: all 0.3s ease;
}
.slide-left-enter-from,
.slide-right-leave-to {
  transform: translateX(100%);
}
.slide-left-leave-to,
.slide-right-enter-from {
  transform: translateX(-100%);
}
</style>
