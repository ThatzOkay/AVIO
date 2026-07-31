<template>
  <div>
    <h2 class="text-headline-small mb-4">Display</h2>
    <v-card color="surface-container-high" class="mb-3">
      <v-list bg-color="transparent">
        <v-list-subheader class="opacity-100 text-medium-emphasis">
          Brightness
        </v-list-subheader>

        <v-list-item class="py-3">
          <div class="d-flex align-center mb-2 mt-2">
            <v-icon
              icon="mdi-brightness-5"
              color="accent"
              size="18"
              class="mr-2"
            />
            <v-slider
              v-model="currentBrightness"
              :min="0"
              :max="100"
              hide-details
            ></v-slider>
            <v-icon
              icon="mdi-brightness-7"
              color="accent"
              size="18"
              class="ml-4"
            />
          </div>
        </v-list-item>
      </v-list>
      <v-list bg-color="transparent">
        <v-list-subheader class="opacity-100 text-medium-emphasis">
          Scale
        </v-list-subheader>

        <v-list-item class="py-3">
          <div class="d-flex align-center mb-2 mt-2">
            <v-icon
              icon="mdi-magnify-minus-outline"
              color="accent"
              size="18"
              class="mr-2"
            />
            <v-slider
              v-model="currentScale"
              :min="0"
              :max="100"
              hide-details
            ></v-slider>
            <v-icon
              icon="mdi-magnify-plus-outline"
              color="accent"
              size="18"
              class="ml-4"
            />
          </div>
        </v-list-item>
      </v-list>
    </v-card>
    <v-card color="surface-container-high">
      <v-list bg-color="transparent">
        <v-list-subheader class="opacity-100 text-medium-emphasis">
          Theme
        </v-list-subheader>

        <v-list-item
          class="py-3 mx-2 rounded-lg"
          style="height: 72px"
          slim
          @click="() => setTheme('m3dark')"
        >
          <template #prepend>
            <v-icon icon="mdi-weather-night" color="primary" size="24" />
          </template>

          <v-list-item-title>Dark</v-list-item-title>
          <v-list-item-subtitle v-if="theme.global.name.value === 'm3dark'">
            Current
          </v-list-item-subtitle>

          <template #append>
            <v-icon
              v-if="theme.global.name.value === 'm3dark'"
              icon="mdi-check-circle-outline"
              color="primary"
              size="20"
            />
          </template>
        </v-list-item>

        <v-divider class="mt-2 mb-2" />

        <v-list-item
          class="py-3 mx-2 rounded-lg"
          style="height: 72px"
          slim
          @click="() => setTheme('m3light')"
        >
          <template #prepend>
            <v-icon icon="mdi-white-balance-sunny" color="primary" size="24" />
          </template>

          <v-list-item-title>Light</v-list-item-title>
          <v-list-item-subtitle v-if="theme.global.name.value === 'm3light'">
            Current
          </v-list-item-subtitle>

          <template #append>
            <v-icon
              v-if="theme.global.name.value === 'm3light'"
              icon="mdi-check-circle-outline"
              color="primary"
              size="20"
            />
          </template>
        </v-list-item>

        <v-divider class="mt-2 mb-2" />

        <v-list-item class="py-3 mx-2 rounded-lg" style="height: 72px" slim>
          <template #prepend>
            <v-icon icon="mdi-palette" color="primary" size="24" />
          </template>

          <v-list-item-title>Base Color</v-list-item-title>
          <v-list-item-subtitle>
            Change the base color of the app
          </v-list-item-subtitle>

          <template #append>
            <div class="d-flex align-center ga-2">
              <span class="text-body-2 text-medium-emphasis seed-hex">{{
                settingsStore.seed
              }}</span>
              <div
                class="seed-swatch"
                :style="{ backgroundColor: settingsStore.seed }"
              />
            </div>
          </template>

          <v-menu
            activator="parent"
            location="bottom"
            transition="slide-y-transition"
            :close-on-content-click="false"
          >
            <v-card color="surface-container" width="260">
              <v-color-picker
                v-model="currentSeed"
                mode="hex"
                bg-color="surface-container"
                width="260"
                flat
                hide-canvas
                hide-inputs
                hide-mode-switch
              />
            </v-card>
          </v-menu>
        </v-list-item>
      </v-list>
    </v-card>

    <v-card color="surface-container-high" class="mt-3">
      <v-list bg-color="transparent">
        <v-list-subheader class="opacity-100 text-medium-emphasis">
          Kiosk &amp; output
        </v-list-subheader>

        <v-list-item class="py-3 mx-2 rounded-lg" style="height: 72px" slim>
          <template #prepend>
            <v-icon icon="mdi-fullscreen" color="primary" size="24" />
          </template>

          <v-list-item-title>Kiosk mode</v-list-item-title>
          <v-list-item-subtitle>
            Always start fullscreen and stay there. Takes effect on next launch.
          </v-list-item-subtitle>

          <template #append>
            <v-switch
              v-model="kioskMode"
              color="primary"
              hide-details
              inset
            />
          </template>
        </v-list-item>

        <v-divider class="mt-2 mb-2" />

        <v-list-item class="py-3 mx-2 rounded-lg" slim>
          <template #prepend>
            <v-icon icon="mdi-monitor-screenshot" color="primary" size="24" />
          </template>

          <v-list-item-title>Display mode</v-list-item-title>
          <v-list-item-subtitle>
            Physical output resolution/refresh rate, applied when kiosk mode starts. Takes effect
            on next launch.
          </v-list-item-subtitle>

          <template #append>
            <v-select
              v-model="displayMode"
              :items="displayModeOptions"
              item-title="label"
              item-value="value"
              density="compact"
              variant="outlined"
              hide-details
              style="min-width: 180px"
            />
          </template>
        </v-list-item>
      </v-list>
    </v-card>
  </div>
</template>

<script setup lang="ts">
import { buildM3Theme, injectM3CssVars } from "@/composables/useM3Theme";
import { useSettingsStore } from "@/store/settingsStore";
import { computed } from "vue";
import { useTheme } from "vuetify";

const settingsStore = useSettingsStore();
const theme = useTheme();

const currentSeed = computed({
  get: () => settingsStore.seed,
  set: (value: string) => {
    Object.assign(theme.themes.value.m3light.colors, buildM3Theme(value, false).colors);
    Object.assign(theme.themes.value.m3dark.colors, buildM3Theme(value, true).colors);
    injectM3CssVars(value, theme.global.name.value === "m3dark");
    settingsStore.setSeed(value);
  },
});

const currentBrightness = computed({
  get: () => settingsStore.brightness,
  set: (value: number) => {
    settingsStore.setBrightness(value);
  },
});

const currentScale = computed({
  get: () => settingsStore.scale,
  set: (value: number) => {
    settingsStore.setScale(value);
  },
});

const kioskMode = computed({
  get: () => settingsStore.kiosk,
  set: (value: boolean) => {
    settingsStore.setKiosk(value);
  },
});

const displayMode = computed({
  get: () => settingsStore.displayMode,
  set: (value: string) => {
    settingsStore.setDisplayMode(value);
  },
});

const displayModeOptions = computed(() => [
  { label: "Panel default", value: "" },
  ...settingsStore.displayModes.map((mode) => ({ label: mode, value: mode })),
]);

const setTheme = (themeName: "m3dark" | "m3light") => {
  theme.global.name.value = themeName;
  settingsStore.setTheme(themeName);
};

</script>

<style scoped>
:deep(.v-list-item__content) {
  padding-left: 12px !important;
  padding-right: 12px !important;
  padding-bottom: 8px !important;
}

:deep(.v-list-subheader) {
  padding-left: 28px !important;
}

.seed-hex {
  font-family: monospace;
  text-transform: uppercase;
}

.seed-swatch {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid rgb(var(--v-theme-outline-variant));
}

:deep(.v-color-picker-preview__dot) {
  background: none;
}
</style>
