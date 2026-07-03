<template>
  <v-app>
    <v-navigation-drawer permanent rail rail-width="64" class="d-flex flex-column">
      <div class="d-flex flex-column fill-height">
        <div>
          <v-list-item :title="time" class="px-0 text-center justify-center"></v-list-item>
          <v-divider></v-divider>
        </div>
        <div class="d-flex flex-column justify-space-between flex-grow-1 mt-2 mb-2">
          <v-list-item @click="() => router.push('/')" link prepend-icon="mdi-phone" class="flex-1-1-0" :class="{ 'v-list-item--active': route.path === '/' }"></v-list-item>
          <v-list-item v-if="statusStore.rtlSdrDetected" @click="() => router.push('/radio')" link prepend-icon="mdi-radio" class="flex-1-1-0" :class="{ 'v-list-item--active': route.path === '/radio' }"></v-list-item>
          <v-list-item @click="() => router.push('/settings')" link prepend-icon="mdi-cog"
            class="flex-1-1-0" :class="{ 'v-list-item--active': route.path === '/settings' }"></v-list-item>
        </div>
      </div>
    </v-navigation-drawer>
    <v-main>
      <RouterView />
    </v-main>
  </v-app>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { onBeforeMount, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useStatusStore } from './store/statusStore';

const statusStore = useStatusStore();
const router = useRouter();
const route = useRoute();
const time = ref('')

const setDate = () => {
  const now = new Date();

  const mins = now.getMinutes();

  const hour = now.getHours();

  time.value = `${hour.toString().padStart(2, '0')}:${mins.toString().padStart(2, '0')}`;
}

onBeforeMount(async () => {
  setDate();
  const rtlSdrDetected = await invoke<boolean>("plugin:rtl-sdr|detect_rtl_sdr");
  statusStore.setRtlSdrDetected(rtlSdrDetected);

  listen('usb-event', async () => {
    const rtlSdrDetected = await invoke<boolean>("plugin:rtl-sdr|detect_rtl_sdr");
    statusStore.setRtlSdrDetected(rtlSdrDetected);
  });
});

onMounted(() => {
  setInterval(setDate, 1000);

  invoke("list_sinks").then((sinks: any) => {
    console.log("Sinks:", sinks);
  });
});
</script>