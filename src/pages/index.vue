<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const greetMsg = ref("");
const name = ref("");

async function greet() {
  greetMsg.value = await invoke("greet", { name: name.value });
}
</script>

<template>
  <v-container class="fill-height d-flex align-center justify-center">
    <v-card width="500" elevation="8" rounded="xl">
      
      <v-card-title class="text-h5">
        Tauri + Vue + Vuetify
      </v-card-title>

      <v-card-subtitle>
        Infotainment UI prototype
      </v-card-subtitle>

      <v-card-text>
        <v-text-field
          v-model="name"
          label="Enter a name"
          variant="outlined"
          density="comfortable"
        />

        <v-btn
          block
          color="primary"
          class="mt-2"
          @click="greet"
        >
          Greet
        </v-btn>

        <v-alert
          v-if="greetMsg"
          class="mt-4"
          type="success"
          variant="tonal"
        >
          {{ greetMsg }}
        </v-alert>
      </v-card-text>
    </v-card>
  </v-container>
</template>