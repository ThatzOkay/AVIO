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
          @click="settingsStore.setTheme('m3dark')"
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
          @click="settingsStore.setTheme('m3light')"
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
      </v-list>
    </v-card>
  </div>
</template>

<script setup lang="ts">
import { useSettingsStore } from "@/store/settingsStore";
import { computed } from "vue";
import { useTheme } from "vuetify";

const settingsStore = useSettingsStore();
const theme = useTheme();

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
</style>
