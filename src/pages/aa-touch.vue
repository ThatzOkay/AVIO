<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";

// Dedicated transparent, always-on-top window that only exists to capture touch input while
// Android Auto is projecting. The video itself is a separate native/compositor surface, not
// part of this webview's content — focusing the *main* window (which isn't transparent) would
// raise it above that surface and hide the video, so input focus lives here instead.
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
</script>

<template>
  <div
    class="aa-touch-layer"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointercancel="onPointerUp"
  />
</template>

<style>
html,
body,
.aa-touch-layer {
  background: transparent !important;
  border: 1px solid red !important; /* debug */
}

.aa-touch-layer {
  position: fixed;
  inset: 0;
}
</style>
