<script setup lang="ts">
import { ref, watch } from "vue";

const show = ref(false);

const props = defineProps<{
  shown: boolean;
}>();

const toggle = () => {
  show.value = !show.value;
};

watch(
  () => props.shown,
  (newVal) => {
    show.value = newVal;
  },
);
defineExpose({
  toggle,
});
</script>

<template>
  <v-slide-y-transition>
    <v-card
      v-show="show"
      class="pa-4"
      style="position: fixed; top: 70px; width: 450px; z-index: 1000"
    >
      <slot />
    </v-card>
  </v-slide-y-transition>
</template>
