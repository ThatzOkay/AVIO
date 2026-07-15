<script setup lang="ts">
import { useRoute, useRouter } from "vue-router";
import androidAutoIcon from "../assets/icons/android-auto.png";
import radioIcon from "../assets/icons/aosp_ic_launcher_radio.png";
import settingsIcon from "../assets/icons/aosp_ic_launcher_settings.png";

const route = useRoute();
const router = useRouter();
</script>

<template>
  <v-layout style="height: 70px">
    <v-bottom-navigation
      style="height: 70px"
      color="primary"
      :model-value="route.path"
      grow
      @update:model-value="router.push($event as string)"
    >
      <v-btn value="/">
        <v-img :src="androidAutoIcon" width="64" height="64" contain />
      </v-btn>

      <v-btn value="/radio">
        <v-img :src="radioIcon" width="64" height="64" contain />
      </v-btn>

      <v-btn value="/settings">
        <v-img
          :src="settingsIcon"
          width="64"
          height="64"
          style="clip-path: circle(50%)"
          contain
        />
      </v-btn>
    </v-bottom-navigation>
  </v-layout>
</template>
