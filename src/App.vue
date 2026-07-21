<template>
  <v-app>
    <TopBar />
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
    />
  </v-app>
</template>

<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { check } from "@tauri-apps/plugin-updater";
import { computed, onBeforeMount, onMounted, ref, watch } from "vue";
import { useStatusStore } from "./store/statusStore";
import BottomBar from "./components/BottomBar.vue";
import TopBar from "./components/TopBar.vue";
import { useRouter } from "vue-router";

const statusStore = useStatusStore();
const router = useRouter();

// Hidden only while actively projecting video. "host-ui" (phone kicked back to its home
// screen) shows the sidebar again alongside the resume button.
const androidAutoActive = computed(() => statusStore.aaStatus === "connected");

const touchActive = ref(false);

const sendTouch = (event: PointerEvent, phase: "down" | "move" | "up") => {
  const x = event.clientX / window.innerWidth;
  const y = event.clientY / window.innerHeight;
  void invoke("aa_send_touch", { x, y, phase });
};

const onPointerDown = (event: PointerEvent) => {
  touchActive.value = true;
  sendTouch(event, "down");
};

const onPointerMove = (event: PointerEvent) => {
  if (!touchActive.value) return;
  sendTouch(event, "move");
};

const onPointerUp = (event: PointerEvent) => {
  if (!touchActive.value) return;
  touchActive.value = false;
  sendTouch(event, "up");
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
    const rtlSdrDetected = await invoke<boolean>(
      "plugin:rtl-sdr|detect_rtl_sdr",
    );
    statusStore.setRtlSdrDetected(rtlSdrDetected);
  });

  listen<"disconnected" | "connected" | "host-ui">("aa-status", (event) => {
    statusStore.setAaStatus(event.payload);
  });
});

onMounted(() => {
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
        console.log(`Update available: ${update.currentVersion} -> ${update.version}`);
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
