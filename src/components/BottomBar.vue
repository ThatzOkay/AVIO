<script setup lang="ts">
import { useRoute, useRouter } from "vue-router";
import { computed } from "vue";
import androidAutoIcon from "../assets/icons/android-auto.png";
import radioIcon from "../assets/icons/aosp_ic_launcher_radio.png";
import settingsIcon from "../assets/icons/aosp_ic_launcher_settings.png";
import { useStatusStore } from "@/store/statusStore";

const statusStore = useStatusStore();

const route = useRoute();
const router = useRouter();

const settingsIconMask = computed(() => `url(${settingsIcon})`);
</script>

<template>
  <v-bottom-navigation
    :height="70"
    color="primary"
    :model-value="route.path"
    grow
    @update:model-value="router.push($event as string)"
  >
    <v-btn value="/">
      <v-img :src="androidAutoIcon" width="64" height="64" contain />
    </v-btn>

    <v-btn v-if="statusStore.rtlSdrDetected" value="/radio">
      <v-img :src="radioIcon" width="64" height="64" contain />
    </v-btn>

    <v-btn value="/settings">
      <div class="settings-icon" style="clip-path: circle(50%)" />
    </v-btn>
  </v-bottom-navigation>
</template>

<style scoped>
.settings-icon {
  width: 64px;
  height: 64px;
  background-color: rgb(var(--v-theme-primary));
  -webkit-mask-image: v-bind(settingsIconMask);
  mask-image: v-bind(settingsIconMask);
  -webkit-mask-repeat: no-repeat;
  mask-repeat: no-repeat;
  -webkit-mask-position: center;
  mask-position: center;
  -webkit-mask-size: contain;
  mask-size: contain;
}
</style>
