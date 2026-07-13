import { defineStore } from "pinia";

export type AaStatus = "disconnected" | "connected" | "host-ui";

export const useStatusStore = defineStore("status", {
  state: () => ({
    rtlSdrDetected: false,
    aaStatus: "disconnected" as AaStatus,
  }),
  actions: {
    setRtlSdrDetected(value: boolean) {
      this.rtlSdrDetected = value;
    },
    setAaStatus(value: AaStatus) {
      this.aaStatus = value;
    },
  },
});
