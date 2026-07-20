<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { useStatusStore } from "../store/statusStore";

definePage({
  meta: {
    index: 0,
  },
});

const statusStore = useStatusStore();

const resumeAndroidAuto = () => {
  void invoke("aa_resume");
};
</script>

<template>
  <div class="w-100 h-100 d-flex flex-column align-center justify-center">
    <v-btn
      v-if="statusStore.aaStatus === 'host-ui'"
      size="x-large"
      color="primary"
      rounded="xl"
      @click="resumeAndroidAuto"
    >
      Resume Android Auto
    </v-btn>
    <v-card-text v-else class="d-flex align-center text-center">
      <h3>Connected phone to start Android Auto</h3>
    </v-card-text>
  </div>
</template>
