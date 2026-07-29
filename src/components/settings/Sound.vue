<template>
  <div>
    <h2 class="text-headline-small mb-4">sound</h2>
    <v-card color="surface-container-high" class="mb-3">
      <v-list bg-color="transparent">
        <v-list-subheader class="opacity-100 text-medium-emphasis">
          Volume
        </v-list-subheader>

        <v-list-item class="py-3">
          <div class="d-flex align-center mb-1">
            <v-icon
              icon="mdi-volume-high"
              color="primary"
              size="18"
              class="mr-2"
            />
            <span class="text-body-2 text-medium-emphasis">Master</span>
          </div>

          <v-slider
            v-model="currentVolume"
            :min="0"
            :max="100"
            hide-details
          ></v-slider>
        </v-list-item>
      </v-list>
    </v-card>
    <v-card color="surface-container-high" class="mb-3">
      <v-list bg-color="transparent">
        <v-list-subheader class="opacity-100 text-medium-emphasis">
          Default Sound Device
        </v-list-subheader>
        <template
          v-for="(device, index) in settingsStore.audioDevices"
          :key="device"
        >
          <v-list-item
            class="py-3 mx-2 rounded-lg"
            style="height: 72px"
            slim
            @click="() => setDefaultDevice(device)"
          >
            <template #prepend>
              <v-icon icon="mdi-speaker" color="primary" size="24" />
            </template>

            <v-list-item-title>{{ device }}</v-list-item-title>

            <v-list-item-subtitle v-if="isDefaultDevice(device)">
              Default
            </v-list-item-subtitle>

            <template #append>
              <div style="width: 30px; display: flex; justify-content: center">
                <v-icon
                  v-if="isDefaultDevice(device)"
                  icon="mdi-check-circle-outline"
                  color="primary"
                  size="20"
                />
              </div>
            </template>
          </v-list-item>

          <v-divider
            v-if="index < settingsStore.audioDevices.length - 1"
            class="mt-2 mb-2"
          />
        </template>
      </v-list>
    </v-card>
    <v-card color="surface-container-high">
      <v-list bg-color="transparent">
        <v-list-subheader class="opacity-100 text-medium-emphasis">
          Default Input Device
        </v-list-subheader>
        <template
          v-for="(device, index) in settingsStore.inputDevices"
          :key="device"
        >
          <v-list-item
            class="py-3 mx-2 rounded-lg"
            style="height: 72px"
            slim
            @click="() => setDefaultInputDevice(device)"
          >
            <template #prepend>
              <v-icon icon="mdi-microphone" color="primary" size="24" />
            </template>

            <v-list-item-title>{{ device }}</v-list-item-title>

            <v-list-item-subtitle v-if="isDefaultInputDevice(device)">
              Default
            </v-list-item-subtitle>

            <template #append>
              <v-icon
                v-if="isDefaultInputDevice(device)"
                icon="mdi-check-circle-outline"
                color="primary"
                size="20"
              />
            </template>
          </v-list-item>

          <v-divider
            v-if="index < settingsStore.inputDevices.length - 1"
            class="mt-2 mb-2"
          />
        </template>
      </v-list>
    </v-card>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useSettingsStore } from "@/store/settingsStore";

const settingsStore = useSettingsStore();

const currentVolume = computed({
  get: () => settingsStore.masterVolume,
  set: (value: number) => {
    settingsStore.setMasterVolume(value);
  },
});

const isDefaultDevice = (deviceName: string) => {
  return deviceName === settingsStore.defaultSoundDevice;
};

const isDefaultInputDevice = (deviceName: string) => {
  return deviceName === settingsStore.defaultInputDevice;
};

// onMounted(async () => {
//   watch(currentVolume, (newVolume) => {
//     settingsStore.setMasterVolume(newVolume);
//   });
// });

const setDefaultDevice = async (deviceName: string) => {
  settingsStore.setDefaultSoundDevice(deviceName);
};

const setDefaultInputDevice = async (deviceName: string) => {
  settingsStore.setDefaultInputDevice(deviceName);
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
</style>
