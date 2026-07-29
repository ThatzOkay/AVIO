<script setup lang="ts">
import { computed, onBeforeMount, onMounted, ref, watch } from "vue";
import SlidingCard from "./SlidingCard.vue";
import { useRouter } from "vue-router";
import { platform } from "@tauri-apps/plugin-os";
import { useSettingsStore } from "@/store/settingsStore.ts";

const router = useRouter();

const settingsStore = useSettingsStore();

const activePanel = ref<"bluetooth" | "audio" | "brightness" | null>(null);

const closePanel = (panel: "bluetooth" | "audio" | "brightness") => {
  if (activePanel.value === panel) activePanel.value = null;
};

const currentVolume = computed({
  get: () => settingsStore.masterVolume,
  set: (value: number) => {
    settingsStore.setMasterVolume(value);
  },
});

const currentBrightness = computed({
  get: () => settingsStore.brightness,
  set: (value: number) => {
    settingsStore.setBrightness(value);
  },
});

const time = ref("");

const setDate = () => {
  const now = new Date();

  const mins = now.getMinutes();

  const hour = now.getHours();

  time.value = `${hour.toString().padStart(2, "0")}:${mins.toString().padStart(2, "0")}`;
};

onBeforeMount(() => {
  setDate();
});

onMounted(async () => {
  setInterval(setDate, 1000);

  // watch(currentVolume, (newVolume) => {
  //   settingsStore.setMasterVolume(newVolume);
  // });
  // watch(currentBrightness, (newBrightness) => {
  //   settingsStore.setBrightness(newBrightness);
  // });
});
</script>

<template>
  <div style="height: 70px !important" class="mt-2 border-bottom">
    <v-row style="height: 70px">
      <v-col class="d-flex align-center">
        <div
          v-click-outside="() => closePanel('bluetooth')"
          class="relative ml-2"
        >
          <v-btn-toggle
            v-model="activePanel"
            color="primary"
            :variant="activePanel === 'bluetooth' ? 'tonal' : 'flat'"
            density="default"
          >
            <v-btn value="bluetooth">
              <v-icon
                size="32"
                style="position: relative"
                icon="mdi-bluetooth"
              />
            </v-btn>
          </v-btn-toggle>
          <sliding-card :shown="activePanel === 'bluetooth'">
            <v-card-title
              class="text-headline-small d-flex align-center justify-space-between"
              >Bluetooth
              <v-switch
                inset="material"
                color="primary"
                size="large"
                hide-details
              />
            </v-card-title>
            <v-row class="mt-0">
              <v-col class="d-flex align-center gap-2">
                <v-icon icon="mdi-plus" size="32" />
                <p class="text-headline-small">
                  To pair a device, open settings.
                </p>
              </v-col>
            </v-row>
            <v-row class="mt-0">
              <v-col>
                <v-btn
                  color="primary"
                  variant="tonal"
                  size="x-large"
                  block
                  width="100%"
                  @click="
                    () => {
                      activePanel = null;
                      router.push('/settings');
                    }
                  "
                >
                  Open Settings
                </v-btn>
              </v-col>
            </v-row>
          </sliding-card>
        </div>

        <div v-click-outside="() => closePanel('audio')" class="relative ml-2">
          <v-btn-toggle
            v-model="activePanel"
            color="primary"
            :variant="activePanel === 'audio' ? 'tonal' : 'flat'"
            density="default"
          >
            <v-btn value="audio">
              <v-icon
                size="32"
                style="position: relative"
                icon="mdi-volume-high"
              />
            </v-btn>
          </v-btn-toggle>
          <sliding-card :shown="activePanel === 'audio'">
            <v-card-title class="text-headline-small">Audio </v-card-title>
            <p>{{ settingsStore.defaultSoundDevice }}</p>
            <v-slider
              v-model="currentVolume"
              :min="0"
              :max="100"
              prepend-icon="mdi-volume-high"
            ></v-slider>
          </sliding-card>
        </div>

        <div
          v-if="platform()"
          v-click-outside="() => closePanel('brightness')"
          class="relative ml-2"
        >
          <v-btn-toggle
            v-model="activePanel"
            color="primary"
            :variant="activePanel === 'brightness' ? 'tonal' : 'flat'"
            density="default"
          >
            <v-btn value="brightness">
              <v-icon
                size="32"
                style="position: relative"
                icon="mdi-brightness-5"
              />
            </v-btn>
          </v-btn-toggle>
          <sliding-card :shown="activePanel === 'brightness'">
            <v-card-title class="text-headline-small">Brightness </v-card-title>
            <v-slider
              v-model="currentBrightness"
              prepend-icon="mdi-monitor"
            ></v-slider>
          </sliding-card>
        </div>
      </v-col>
      <v-col class="text-center align-center justify-center d-flex"
        ><p class="mt-0 mb-0 font-weight-medium text-display-small">
          {{ time }}
        </p></v-col
      >
      <v-col></v-col>
    </v-row>
  </div>
</template>
