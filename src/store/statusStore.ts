import { defineStore } from "pinia";

export const useStatusStore = defineStore("status", {
  state: () => ({
    rtlSdrDetected: false,
  }),
  actions: {
    setRtlSdrDetected(value: boolean) {
      this.rtlSdrDetected = value;
    },
  },
});
