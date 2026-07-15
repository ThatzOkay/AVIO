<script setup lang="ts">
import { onBeforeMount, onMounted, ref } from "vue";

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

onMounted(() => {
  setInterval(setDate, 1000);
});
</script>

<template>
  <v-layout style="height: 70px" class="pt-2">
    <v-col></v-col>
    <v-col class="text-center"
      ><p class="font-weight-medium text-display-small">{{ time }}</p></v-col
    >
    <v-col></v-col>
  </v-layout>
</template>
