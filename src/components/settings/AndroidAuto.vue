<script setup lang="ts">
import { useSettingsStore } from "@/store/settingsStore";
import { useStatusStore } from "@/store/statusStore";
import { computed, reactive } from "vue";

const status = useStatusStore();
const settingsStore = useSettingsStore();

const resolution = computed({
  get: () => settingsStore.aoResolution,
  set: (value: '720' | '1080') => {
    settingsStore.setAoResolution(value);
  },
});
const resolutionOptions = ["720p", "1080p"];

const frameRate = computed({
  get: () => settingsStore.aoFramerate,
  set: (value: '30' | '60') => {
    settingsStore.setAoFramerate(value);
  },
});
const frameRateOptions = [
  { value: "30", label: "30 fps" },
  { value: "60", label: "60 fps" },
];

const { min, max } = reactive({ min: 96, max: 213 });

const currentDpi = computed({
  get: () => settingsStore.aoDpi,
  set: (value: number) => {
    settingsStore.setAoDpi(value);
  },
});

const viewAreaTop = computed({
  get: () => settingsStore.aoViewAreaTop,
  set: (value: number) => settingsStore.setAoViewAreaTop(value),
});
const viewAreaBottom = computed({
  get: () => settingsStore.aoViewAreaBottom,
  set: (value: number) => settingsStore.setAoViewAreaBottom(value),
});
const viewAreaLeft = computed({
  get: () => settingsStore.aoViewAreaLeft,
  set: (value: number) => settingsStore.setAoViewAreaLeft(value),
});
const viewAreaRight = computed({
  get: () => settingsStore.aoViewAreaRight,
  set: (value: number) => settingsStore.setAoViewAreaRight(value),
});

const safeAreaTop = computed({
  get: () => settingsStore.aoSafeAreaTop,
  set: (value: number) => settingsStore.setAoSafeAreaTop(value),
});
const safeAreaBottom = computed({
  get: () => settingsStore.aoSafeAreaBottom,
  set: (value: number) => settingsStore.setAoSafeAreaBottom(value),
});
const safeAreaLeft = computed({
  get: () => settingsStore.aoSafeAreaLeft,
  set: (value: number) => settingsStore.setAoSafeAreaLeft(value),
});
const safeAreaRight = computed({
  get: () => settingsStore.aoSafeAreaRight,
  set: (value: number) => settingsStore.setAoSafeAreaRight(value),
});
</script>

<template>
  <div>
    <h2 class="text-headline-small mb-4">Android Auto</h2>

    <v-card color="surface-container-high" class="mb-3">
      <v-list bg-color="transparent">
        <v-list-item class="py-3 mx-2 rounded-lg" style="height: 72px" slim>
          <template #prepend>
            <v-icon icon="mdi-connection" color="primary" size="24" />
          </template>
          <v-list-item-title>Status</v-list-item-title>
          <v-list-item-subtitle>Connected to Android Auto</v-list-item-subtitle>
          <template #append>
            <v-chip variant="outlined" color="primary" rounded="pill">
              {{
                status.aaStatus.status === "connected" || status.aaStatus.status === "host-ui"
                  ? "Connected"
                  : "Disconnected"
              }}
            </v-chip>
          </template>
        </v-list-item>
        <v-divider class="mt-2 mb-2" />
        <v-list-item class="py-3 mx-2 rounded-lg" style="height: 72px" slim>
          <template #prepend>
            <v-icon icon="mdi-cellphone" color="primary" size="24" />
          </template>
          <v-list-item-title>Device</v-list-item-title>
          <v-list-item-subtitle>{{
            status.aaStatus.deviceName || "Not connected"
          }}</v-list-item-subtitle>
        </v-list-item>
      </v-list>
    </v-card>

    <v-card color="surface-container-high">
      <v-list bg-color="transparent">
        <v-list-subheader class="opacity-100 text-medium-emphasis">
          Video quality
        </v-list-subheader>

        <v-list-item
          class="aa-row py-4"
          slim
          style="--v-list-prepend-gap: 30px"
        >
          <template #prepend>
            <v-icon icon="mdi-television" color="primary" size="24" />
          </template>

          <v-list-item-title>Resolution</v-list-item-title>

          <template #append>
            <v-chip-group v-model="resolution" mandatory>
              <v-chip
                v-for="option in resolutionOptions"
                :key="option"
                :value="option"
                variant="outlined"
                rounded="pill"
                color="primary"
                class="quality-chip"
              >
                {{ option }}
              </v-chip>
            </v-chip-group>
          </template>
        </v-list-item>

        <v-divider />

        <v-list-item
          class="aa-row py-4"
          slim
          style="--v-list-prepend-gap: 30px"
        >
          <template #prepend>
            <v-icon icon="mdi-artboard" color="primary" size="24" />
          </template>

          <v-list-item-title>Frame rate</v-list-item-title>

          <template #append>
            <v-chip-group v-model="frameRate" mandatory>
              <v-chip
                v-for="option in frameRateOptions"
                :key="option.value"
                :value="option.value"
                variant="outlined"
                rounded="pill"
                color="primary"
                class="quality-chip"
              >
                {{ option.label }}
              </v-chip>
            </v-chip-group>
          </template>
        </v-list-item>

        <v-divider />

        <v-list-item id="dpi-item" class="py-3">
          <template #prepend>
            <v-icon icon="mdi-image-size-select-large" color="primary" size="24" />
          </template>

          <v-list-item-title>Dpi</v-list-item-title>

          <template #append>
            <v-chip variant="outlined" color="primary" rounded="pill">
              {{ currentDpi }}
            </v-chip>
          </template>

          <div class="d-flex align-center mb-2 mt-2">
            <v-icon
              icon="mdi-format-font-size-increase"
              color="accent"
              size="18"
              class="mr-2"
            />
            <v-slider
              v-model="currentDpi"
              :min="min"
              :max="max"
              :step="1"
              hide-details
            ></v-slider>
            <v-icon
              icon="mdi-format-font-size-decrease"
              color="accent"
              size="18"
              class="ml-4"
            />
          </div>
        </v-list-item>

      </v-list>
    </v-card>

    <v-card color="surface-container-high" class="mt-3">
      <v-list bg-color="transparent">
        <v-list-subheader class="opacity-100 text-medium-emphasis">
          Screen geometry
        </v-list-subheader>

        <v-list-item class="py-3">
          <template #prepend>
            <v-icon icon="mdi-arrow-expand-all" color="primary" size="24" />
          </template>

          <v-list-item-title>View area insets</v-list-item-title>
          <v-list-item-subtitle>
            Extra space reserved around the projected video, on top of the automatic
            aspect-ratio letterboxing
          </v-list-item-subtitle>

          <v-row class="mt-2" dense>
            <v-col cols="6" sm="3">
              <v-text-field
                v-model.number="viewAreaTop"
                label="Top"
                type="number"
                min="0"
                density="compact"
                variant="outlined"
                hide-details
              />
            </v-col>
            <v-col cols="6" sm="3">
              <v-text-field
                v-model.number="viewAreaBottom"
                label="Bottom"
                type="number"
                min="0"
                density="compact"
                variant="outlined"
                hide-details
              />
            </v-col>
            <v-col cols="6" sm="3">
              <v-text-field
                v-model.number="viewAreaLeft"
                label="Left"
                type="number"
                min="0"
                density="compact"
                variant="outlined"
                hide-details
              />
            </v-col>
            <v-col cols="6" sm="3">
              <v-text-field
                v-model.number="viewAreaRight"
                label="Right"
                type="number"
                min="0"
                density="compact"
                variant="outlined"
                hide-details
              />
            </v-col>
          </v-row>
        </v-list-item>

        <v-divider />

        <v-list-item class="py-3">
          <template #prepend>
            <v-icon icon="mdi-crop-free" color="primary" size="24" />
          </template>

          <v-list-item-title>Safe area insets</v-list-item-title>
          <v-list-item-subtitle>
            Content insets the phone keeps clear of overlays (e.g. a dashboard cutout), without
            affecting the letterbox crop
          </v-list-item-subtitle>

          <v-row class="mt-2" dense>
            <v-col cols="6" sm="3">
              <v-text-field
                v-model.number="safeAreaTop"
                label="Top"
                type="number"
                min="0"
                density="compact"
                variant="outlined"
                hide-details
              />
            </v-col>
            <v-col cols="6" sm="3">
              <v-text-field
                v-model.number="safeAreaBottom"
                label="Bottom"
                type="number"
                min="0"
                density="compact"
                variant="outlined"
                hide-details
              />
            </v-col>
            <v-col cols="6" sm="3">
              <v-text-field
                v-model.number="safeAreaLeft"
                label="Left"
                type="number"
                min="0"
                density="compact"
                variant="outlined"
                hide-details
              />
            </v-col>
            <v-col cols="6" sm="3">
              <v-text-field
                v-model.number="safeAreaRight"
                label="Right"
                type="number"
                min="0"
                density="compact"
                variant="outlined"
                hide-details
              />
            </v-col>
          </v-row>
        </v-list-item>
      </v-list>
    </v-card>
  </div>
</template>

<style scoped>
.aa-row {
  min-height: 64px;
}

.quality-chip:not(.v-chip--selected) {
  color: rgb(var(--v-theme-on-surface-variant));
  border-color: rgb(var(--v-theme-on-surface-variant));
}

:deep(#dpi-item .v-list-item__append) {
  height: 100%;
  align-items: end;
  margin-bottom: 16px;
  margin-left: 8px;
}
</style>
