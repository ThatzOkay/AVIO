<script setup lang="ts">
import { onBeforeMount, onMounted, ref, watch } from "vue";
import SlidingCard from "./SlidingCard.vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { platform } from "@tauri-apps/plugin-os";

const router = useRouter();

const activePanel = ref<"bluetooth" | "audio" | "brightness" | null>(null);

const closePanel = (panel: "bluetooth" | "audio" | "brightness") => {
  if (activePanel.value === panel) activePanel.value = null;
};

const defaultDeviceName = ref("");
const currentVolume = ref(0);
const currentBrightness = ref(0);

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

  await invoke<string>("get_default_device_name").then((name: string) => {
    defaultDeviceName.value = name;
  });

  await invoke<number>("get_current_volume").then((volume: number) => {
    currentVolume.value = volume;
  });

  await invoke<number>("get_current_brightness").then((brightness: number) => {
    currentBrightness.value = brightness;
    console.log("Current brightness:", brightness);
  });
});

watch(currentVolume, (newVolume) => {
  updateVolume(newVolume);
});

watch(currentBrightness, (newBrightness) => {
  updateBrightness(newBrightness);
});

const updateVolume = async (volume: number) => {
  await invoke("set_current_volume", { volume: Number(volume.toFixed(0)) });
};

const updateBrightness = async (brightness: number) => {
  await invoke("set_brightness", { value: Number(brightness.toFixed(0)) });
};
</script>

<template>
  <v-layout
    :full-height="false"
    style="height: 70px !important"
    class="mt-2 border-bottom"
  >
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
            <p>{{ defaultDeviceName }}</p>
            <v-slider
              v-model="currentVolume"
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
  </v-layout>
</template>
