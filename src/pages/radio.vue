<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

interface StationInfo {
  program_id: number;
  genre: string;
  name: string | null;
  text: string | null;
}

interface RadioState {
  running: boolean;
  frequency: number;
  mode: 'FM' | 'DAB';
  station: StationInfo | null;
  favorites: number[] | null;
}

const FAVORITE_SLOTS = 5;
const LONG_PRESS_MS = 600;

const tab = ref('fm');
const busy = ref(false);
const running = ref(false);
const frequency = ref(100_000);
const favorites = ref<number[]>(Array(FAVORITE_SLOTS).fill(0));

const frequencyDisplay = computed(() => (frequency.value / 1000).toFixed(2));
const statusText = computed(() => (running.value ? 'Playing' : 'Stopped'));

const station = ref<StationInfo | null>(null);

const presets = computed(() =>
  favorites.value.map((freq, i) => ({
    label: freq > 0 ? (freq / 1000).toFixed(2) : String(i + 1),
    saved: freq > 0,
    active: freq > 0 && freq === frequency.value,
  })),
);

function applyState(state: RadioState) {
  running.value = state.running;
  frequency.value = state.frequency;
  favorites.value = state.favorites?.length ? state.favorites : Array(FAVORITE_SLOTS).fill(0);
  station.value = state.station;
}

async function refresh() {
  applyState(await invoke<RadioState>('get_fm_state'));
}

async function withBusy(action: () => Promise<RadioState>) {
  if (busy.value) return;
  busy.value = true;
  try {
    applyState(await action());
  } finally {
    busy.value = false;
  }
}

function toggleStartStop() {
  return withBusy(() =>
    running.value ? invoke<RadioState>('stop') : invoke<RadioState>('start', { frequency: frequency.value }),
  );
}

function step(direction: number, fast: boolean) {
  return withBusy(() => invoke<RadioState>('step_fm', { direction, fast }));
}

let pressTimer: ReturnType<typeof setTimeout> | null = null;
let longPressFired = false;

function onPresetPointerDown(slot: number) {
  if (busy.value) return;
  longPressFired = false;
  pressTimer = setTimeout(() => {
    longPressFired = true;
    withBusy(() => invoke<RadioState>('set_fm_favorite', { slot }));
  }, LONG_PRESS_MS);
}

function onPresetPointerUp(slot: number) {
  if (pressTimer) {
    clearTimeout(pressTimer);
    pressTimer = null;
  }
  if (longPressFired || busy.value || !presets.value[slot].saved) return;
  withBusy(() => invoke<RadioState>('recall_fm_favorite', { slot }));
}

function onPresetPointerCancel() {
  if (pressTimer) {
    clearTimeout(pressTimer);
    pressTimer = null;
  }
}

let unlistenFmState: UnlistenFn | null = null;

onMounted(async () => {
  await refresh();
  applyState(await invoke<RadioState>('start', { frequency: frequency.value }));

  unlistenFmState = await listen("fm-state", (event) => {
    const state = event.payload as RadioState;
    applyState(state);
  });
});

// Leaving the FM tab (DAB is a placeholder today, but this holds regardless of what's
// implemented behind it) or leaving the page entirely should release the SDR hardware -
// otherwise the RTL-SDR read/DSP threads keep running in the background indefinitely.
watch(tab, (newTab, oldTab) => {
  if (oldTab === 'fm' && newTab !== 'fm') {
    invoke('stop').catch(() => {});
  }
});

onUnmounted(() => {
  invoke('stop').catch(() => {});
  unlistenFmState?.();
});
</script>

<template>
  <div class="radio-page fill-height d-flex flex-column align-center">
    <div class="flex-grow-1" />

    <v-tabs
      v-model="tab"
      class="radio-tabs mb-6"
      density="compact"
      color="cyan-accent-3"
      slider-color="cyan-accent-3"
      align-tabs="center"
    >
      <v-tab value="dab">
        DAB
      </v-tab>
      <v-tab value="fm">
        FM
      </v-tab>
    </v-tabs>

    <v-tabs-window
      v-model="tab"
      class="w-100"
      crossfade
    >
      <v-tabs-window-item
        value="fm"
        class="header-panel d-flex flex-column align-center w-100"
      >
        <div class="section-label text-medium-emphasis mb-2">
          FM RADIO
        </div>

        <div class="d-flex align-end">
          <span class="freq-display">{{ frequencyDisplay }}</span>
          <span class="freq-unit text-medium-emphasis ml-2 mb-2">MHz</span>
        </div>
        <div class="text-medium-emphasis mb-2">
          {{ station?.name || station?.text || 'No station' }}
        </div>
        <div class="text-medium-emphasis">
          {{ statusText }}
        </div>
      </v-tabs-window-item>
      <v-tabs-window-item
        value="dab"
        class="header-panel d-flex flex-column align-center w-100"
      >
        <v-icon
          icon="mdi-radio"
          size="48"
          class="text-medium-emphasis mb-4"
        />
        <div class="section-label text-medium-emphasis mb-2">
          DAB RADIO
        </div>
        <div class="text-medium-emphasis">
          Coming soon
        </div>
      </v-tabs-window-item>
    </v-tabs-window>

    <div class="flex-grow-1" />

    <div
      class="d-flex align-center ga-4 mb-8"
      :class="{ 'controls-hidden': tab !== 'fm' }"
    >
      <v-btn
        icon="mdi-rewind"
        size="64"
        variant="flat"
        color="grey-darken-3"
        :disabled="busy"
        @click="step(-1, true)"
      />
      <v-btn
        icon="mdi-chevron-left"
        size="64"
        variant="flat"
        color="grey-darken-3"
        :disabled="busy"
        @click="step(-1, false)"
      />
      <v-btn
        :icon="running ? 'mdi-stop' : 'mdi-play'"
        size="76"
        variant="flat"
        color="white"
        :disabled="busy"
        @click="toggleStartStop"
      />
      <v-btn
        icon="mdi-chevron-right"
        size="64"
        variant="flat"
        color="grey-darken-3"
        :disabled="busy"
        @click="step(1, false)"
      />
      <v-btn
        icon="mdi-fast-forward"
        size="64"
        variant="flat"
        color="grey-darken-3"
        :disabled="busy"
        @click="step(1, true)"
      />
    </div>

    <div
      class="d-flex align-center ga-3"
      :class="{ 'controls-hidden': tab !== 'fm' }"
    >
      <v-btn
        v-for="(preset, i) in presets"
        :key="i"
        icon
        size="64"
        variant="flat"
        color="grey-darken-3"
        class="preset-btn"
        :class="{ 'preset-btn--active': preset.active }"
        :disabled="busy"
        @pointerdown="onPresetPointerDown(i)"
        @pointerup="onPresetPointerUp(i)"
        @pointerleave="onPresetPointerCancel"
        @pointercancel="onPresetPointerCancel"
        @contextmenu.prevent
      >
        <span class="preset-label">{{ preset.label }}</span>
      </v-btn>
    </div>
    <div
      class="text-medium-emphasis text-body-small mt-3 mb-10"
      :class="{ 'controls-hidden': tab !== 'fm' }"
    >
      <v-icon
        icon="mdi-star"
        size="x-small"
        class="mr-1"
      />tap to play, hold to save
    </div>
  </div>
</template>

<style scoped>
.radio-tabs {
    max-width: 220px;
}

.controls-hidden {
    visibility: hidden;
}

.header-panel {
    min-height: 140px;
}

.section-label {
    font-size: 0.8rem;
    font-weight: 600;
    letter-spacing: 0.3em;
}

.freq-display {
    font-size: 5.5rem;
    font-weight: 700;
    line-height: 1;
}

.freq-unit {
    font-size: 1.75rem;
    font-weight: 500;
}

.preset-btn--active {
    border: 2px solid #00e5ff;
}

.preset-label {
    font-size: 0.85rem;
    font-weight: 700;
}
</style>
