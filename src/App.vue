<template>
  <!-- The aa-touch window is a transparent, chrome-less overlay just for capturing touch input
       over the Android Auto video surface — it must never render the app shell (Vuetify's
       v-app background isn't transparent, which would hide the video underneath it). -->
  <RouterView v-if="isTouchWindow" />
  <v-app v-else>
    <v-navigation-drawer v-if="!androidAutoActive" permanent rail rail-width="64" class="d-flex flex-column">
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
import { getCurrentWindow } from '@tauri-apps/api/window';
import { computed, onBeforeMount, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useStatusStore } from './store/statusStore';

const statusStore = useStatusStore();
const router = useRouter();
const route = useRoute();
const time = ref('')

const isTouchWindow = getCurrentWindow().label === 'aa-touch';

// Hidden only while actively projecting video. "host-ui" (phone kicked back to its home
// screen) shows the sidebar again alongside the resume button.
const androidAutoActive = computed(() => statusStore.aaStatus === 'connected');

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

  listen<'disconnected' | 'connected' | 'host-ui'>('aa-status', (event) => {
    statusStore.setAaStatus(event.payload);
  });
});

onMounted(() => {
  setInterval(setDate, 1000);

  invoke("list_sinks").then((sinks: any) => {
    console.log("Sinks:", sinks);
  });
});
</script>