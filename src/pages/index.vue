<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { useStatusStore } from "../store/statusStore";

const statusStore = useStatusStore();

const resumeAndroidAuto = () => {
  void invoke("aa_resume");
};
</script>

<template>
  <v-container v-if="statusStore.aaStatus === 'host-ui'" class="fill-height d-flex align-center justify-center">
    <v-btn size="x-large" color="primary" rounded="xl" @click="resumeAndroidAuto">
      Resume Android Auto
    </v-btn>
  </v-container>
</template>
