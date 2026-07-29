<script setup lang="ts">
import { ref, type Component } from "vue";
import ConnectedDevices from "../components/settings/ConnectedDevices.vue";
import Sound from "../components/settings/Sound.vue";
import Display from "../components/settings/Display.vue";
import AndroidAuto from "../components/settings/AndroidAuto.vue";
import Updates from "../components/settings/Updates.vue";
import About from "../components/settings/About.vue";

definePage({
  meta: {
    index: 2,
  },
});

const currentCategory = ref("connected-devices");

const categories = [
  {
    id: "connected-devices",
    label: "Connected Devices",
    icon: "mdi-cellphone",
  },
  {
    id: "sound",
    label: "Sound",
    icon: "mdi-volume-high",
  },
  {
    id: "display",
    label: "Display",
    icon: "mdi-monitor",
  },
  {
    id: "android-auto",
    label: "Android Auto",
    icon: "mdi-car",
  },
  {
    id: "updates",
    label: "Updates",
    icon: "mdi-update",
  },
  {
    id: "about",
    label: "About",
    icon: "mdi-information-outline",
  },
];

const categoryComponents: Record<string, Component> = {
  "connected-devices": ConnectedDevices,
  sound: Sound,
  display: Display,
  "android-auto": AndroidAuto,
  updates: Updates,
  about: About,
};
</script>

<template>
  <div class="settings d-flex w-100 h-100">
    <div class="settings-nav h-100 flex-shrink-0 d-flex flex-column">
      <v-list-subheader
        class="settings-title text-title-large font-weight-medium opacity-100 pt-4 pb-4 ps-6"
        >Settings</v-list-subheader
      >
      <v-list nav class="flex-grow-1 overflow-y-auto d-flex flex-column">
        <v-list-item
          v-for="cat in categories"
          :key="cat.id"
          :prepend-icon="cat.icon"
          :title="cat.label"
          :value="cat.id"
          rounded="xl"
          color="primary"
          :active="currentCategory === cat.id"
          @click="currentCategory = cat.id"
        />
      </v-list>
    </div>

    <v-divider vertical />

    <div
      class="settings-content flex-grow-1 h-100 position-relative overflow-hidden"
    >
      <transition name="panel">
        <div
          :key="currentCategory"
          class="settings-panel position-absolute overflow-y-auto py-7 px-8"
        >
          <component :is="categoryComponents[currentCategory]" />
        </div>
      </transition>
    </div>
  </div>
</template>

<style scoped>
.settings-nav {
  width: 280px;
}

.settings-title {
  height: auto;
  color: rgb(var(--v-theme-on-surface));
  background-color: rgb(var(--v-theme-surface-container-low));
}

.settings-panel {
  inset: 0;
}

.panel-enter-active,
.panel-leave-active {
  transition:
    opacity 0.22s ease,
    transform 0.22s ease;
}

.panel-enter-from {
  opacity: 0;
  transform: translateX(24px);
}

.panel-leave-to {
  opacity: 0;
  transform: translateX(-24px);
}
</style>
