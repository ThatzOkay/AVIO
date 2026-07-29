<script setup lang="ts">
import { useStatusStore } from "@/store/statusStore";
import { ref } from "vue";

const status = useStatusStore();

const resolution = ref("1080p");
const resolutionOptions = ["720p", "1080p"];

const frameRate = ref("60");
const frameRateOptions = [
  { value: "30", label: "30 fps" },
  { value: "60", label: "60 fps" },
];
</script>

<template>
  <div>
    <h2 class="text-headline-small mb-4">Android Auto</h2>

    <v-card color="surface-container-high" class="mb-3">
      <v-list bg-color="transparent">
        <v-list-item class="py-3 mx-2 rounded-lg" style="height: 72px" slim>
          <template #prepend>
            <v-icon icon="mdi-connection" color="primary" size="24" />
          </template>
          <v-list-item-title>Status</v-list-item-title>
          <v-list-item-subtitle>Connected to Android Auto</v-list-item-subtitle>
          <template #append>
            <v-chip variant="outlined" color="primary" rounded="pill">
              {{
                status.aaStatus.status === "connected" || status.aaStatus.status === "host-ui"
                  ? "Connected"
                  : "Disconnected"
              }}
            </v-chip>
          </template>
        </v-list-item>
        <v-divider class="mt-2 mb-2" />
        <v-list-item class="py-3 mx-2 rounded-lg" style="height: 72px" slim>
          <template #prepend>
            <v-icon icon="mdi-cellphone" color="primary" size="24" />
          </template>
          <v-list-item-title>Device</v-list-item-title>
          <v-list-item-subtitle>{{
            status.aaStatus.deviceName || "Not connected"
          }}</v-list-item-subtitle>
        </v-list-item>
      </v-list>
    </v-card>

    <v-card color="surface-container-high">
      <v-list bg-color="transparent">
        <v-list-subheader class="opacity-100 text-medium-emphasis">
          Video quality
        </v-list-subheader>

        <v-list-item
          class="aa-row py-4"
          slim
          style="--v-list-prepend-gap: 30px"
        >
          <template #prepend>
            <v-icon icon="mdi-television" color="primary" size="24" />
          </template>

          <v-list-item-title>Resolution</v-list-item-title>

          <template #append>
            <v-chip-group v-model="resolution" mandatory>
              <v-chip
                v-for="option in resolutionOptions"
                :key="option"
                :value="option"
                variant="outlined"
                rounded="pill"
                color="primary"
                class="quality-chip"
              >
                {{ option }}
              </v-chip>
            </v-chip-group>
          </template>
        </v-list-item>

        <v-divider />

        <v-list-item
          class="aa-row py-4"
          slim
          style="--v-list-prepend-gap: 30px"
        >
          <template #prepend>
            <v-icon icon="mdi-artboard" color="primary" size="24" />
          </template>

          <v-list-item-title>Frame rate</v-list-item-title>

          <template #append>
            <v-chip-group v-model="frameRate" mandatory>
              <v-chip
                v-for="option in frameRateOptions"
                :key="option.value"
                :value="option.value"
                variant="outlined"
                rounded="pill"
                color="primary"
                class="quality-chip"
              >
                {{ option.label }}
              </v-chip>
            </v-chip-group>
          </template>
        </v-list-item>
      </v-list>
    </v-card>
  </div>
</template>

<style scoped>
.aa-row {
  min-height: 64px;
}

.quality-chip:not(.v-chip--selected) {
  color: rgb(var(--v-theme-on-surface-variant));
  border-color: rgb(var(--v-theme-on-surface-variant));
}
</style>
