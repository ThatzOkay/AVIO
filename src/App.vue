<template>
  <v-app>
    <TopBar v-if="!androidAutoActive" />
    <v-main v-if="!androidAutoActive">
      <v-container
        fluid
        style="position: unset"
        class="relative fill-width fill-height d-flex align-center justify-center"
      >
        <v-card
          class="w-100 h-100 d-flex flex-column align-center justify-center"
        >
          <router-view v-slot="{ Component }" class="inline">
            <transition :name="transitionName">
              <component :is="Component" />
            </transition>
          </router-view>
        </v-card>
      </v-container>
    </v-main>
    <BottomBar v-if="!androidAutoActive" />
    <!-- While projecting, this replaces the app shell entirely: a transparent, full-viewport
         layer that both lets the compositor's video plane underneath show through (via the
         show-video class below) and captures touch/pointer input for it — same window, same
         element, so no separate overlay window is needed once the main window itself can go
         transparent (see tauri.conf.json + aa_set_main_transparent). -->
    <div
      v-if="androidAutoActive"
      class="aa-video-layer"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
      @touchstart="onTouchStart"
      @touchmove="onTouchMove"
      @touchend="onTouchEnd"
      @touchcancel="onTouchEnd"
    />
  </v-app>
</template>

<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { check } from "@tauri-apps/plugin-updater";
import { computed, onBeforeMount, onMounted, ref, watch } from "vue";
import { useTheme } from "vuetify";
import { AaStatus, useStatusStore } from "./store/statusStore";
import BottomBar from "./components/BottomBar.vue";
import TopBar from "./components/TopBar.vue";
import { useRouter } from "vue-router";
import { useSettingsStore } from "./store/settingsStore.ts";
import { buildM3Theme, injectM3CssVars } from "./composables/useM3Theme";

const settingsStore = useSettingsStore();
const statusStore = useStatusStore();
const router = useRouter();
const theme = useTheme();

// Re-derives both theme variants from the new seed and swaps the live colors in place —
// Vuetify's theme stylesheet is reactive to `theme.themes`, so this repaints immediately
// without recreating the Vuetify instance.
watch(
  () => settingsStore.seed,
  (seed) => {
    Object.assign(theme.themes.value.m3light.colors, buildM3Theme(seed, false).colors);
    Object.assign(theme.themes.value.m3dark.colors, buildM3Theme(seed, true).colors);
    injectM3CssVars(seed, theme.global.name.value === "m3dark");
  },
  { immediate: true },
);

// Hidden only while actively projecting video. "host-ui" (phone kicked back to its home
// screen) shows the sidebar again alongside the resume button.
const androidAutoActive = computed(
  () => statusStore.aaStatus.status === "connected",
);

const touchActive = ref(false);

const touchPoint = (t: Touch) => ({
  x: t.clientX / window.innerWidth,
  y: t.clientY / window.innerHeight,
  id: t.identifier,
});

const sendTouch = (
  touches: { x: number; y: number; id: number }[],
  phase: "down" | "pointerdown" | "move" | "pointerup" | "up",
  actionIndex: number,
) => {
  void invoke("aa_send_touch", { touches, phase, actionIndex });
};

const sendPointer = (event: PointerEvent, phase: "down" | "move" | "up") => {
  const x = event.clientX / window.innerWidth;
  const y = event.clientY / window.innerHeight;
  void invoke("aa_send_pointer", { x, y, phase });
};

const onTouchStart = (event: TouchEvent) => {
  touchActive.value = true;
  const all = Array.from(event.touches).map(touchPoint);
  const changed = event.changedTouches[0];
  const actionIndex = all.findIndex((p) => p.id === changed.identifier);
  // A second (or third...) finger touching down while others are already active is a
  // POINTER_DOWN on the existing gesture, not a fresh DOWN - a plain DOWN here would reset the
  // phone's touch state machine instead of adding a pointer to it.
  sendTouch(all, all.length === 1 ? "down" : "pointerdown", actionIndex);
};

const onTouchMove = (event: TouchEvent) => {
  if (!touchActive.value) return;
  sendTouch(Array.from(event.touches).map(touchPoint), "move", 0);
};

const onTouchEnd = (event: TouchEvent) => {
  if (!touchActive.value) return;
  // `event.touches` already excludes the lifted pointer(s) by this point - AA needs the full
  // pointer set as it stood just before the lift, with actionIndex pointing at the one that
  // went up (POINTER_UP), or a plain UP only once every finger is off the screen.
  const remaining = Array.from(event.touches).map(touchPoint);
  const lifted = Array.from(event.changedTouches).map(touchPoint);
  const all = [...remaining, ...lifted];
  sendTouch(all, remaining.length === 0 ? "up" : "pointerup", remaining.length);
  if (remaining.length === 0) touchActive.value = false;
};

const onPointerDown = (event: PointerEvent) => {
  touchActive.value = true;
  sendPointer(event, "down");
};

const onPointerMove = (event: PointerEvent) => {
  if (!touchActive.value) return;
  sendPointer(event, "move");
};

const onPointerUp = (event: PointerEvent) => {
  if (!touchActive.value) return;
  touchActive.value = false;
  sendPointer(event, "up");
};

// The main window can go transparent (tauri.conf.json enables the capability) so that when the
// DOM itself goes transparent, the AA video plane underneath (owned by the compositor, not this
// webview) becomes visible. Both the CSS class (document.documentElement, not just this
// component's own root, since Vue's scoped styles can't reach html/body anyway) and the
// window's own background color need toggling together — only while projecting, or WebKitGTK's
// normal opaque default breaks for the rest of the app too.
watch(
  androidAutoActive,
  (active) => {
    document.documentElement.classList.toggle("show-video", active);
    void invoke("aa_set_main_transparent", { transparent: active });
  },
  { immediate: true },
);

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
    settingsStore.initDevices();

    const rtlSdrDetected = await invoke<boolean>(
      "plugin:rtl-sdr|detect_rtl_sdr",
    );
    statusStore.setRtlSdrDetected(rtlSdrDetected);
  });

  listen<AaStatus>("aa-status", (event) => {
    statusStore.setAaStatus(event.payload);
  });
});

onMounted(() => {
  settingsStore.init();
  settingsStore.initDisplayModes();
  const windowViewWidth = window.innerWidth;
  const windowViewHeight = window.innerHeight;
  const scaleFactor = window.devicePixelRatio;

  console.log(
    `Window view width: ${windowViewWidth}, height: ${windowViewHeight}, scale factor: ${scaleFactor}`,
  );

  // Download/install UI (notification pull-down) comes later — for now this just proves the
  // check itself works end to end against the published latest.json.
  check()
    .then((update) => {
      if (update) {
        console.log(
          `Update available: ${update.currentVersion} -> ${update.version}`,
        );
      } else {
        console.log("No update available");
      }
    })
    .catch((e) => {
      console.error("Update check failed:", e);
    });
});
</script>

<style>
/* Unscoped: needs to reach html/body, which a scoped style can't target. Punches the DOM
   background transparent while AA is projecting, so the compositor's video plane underneath
   this (now OS-transparent, see tauri.conf.json) window becomes visible. */
html.show-video,
html.show-video body,
html.show-video #app,
html.show-video .v-application {
  background: transparent !important;
}
</style>

<style scoped>
.aa-video-layer {
  position: fixed;
  inset: 0;
  touch-action: none;
}

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

/* :deep(.v-list-item__prepend) {
  width: 35px !important;
} */

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
