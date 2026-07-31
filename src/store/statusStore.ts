import { defineStore } from "pinia";

export type AaStatus = {
  status: "disconnected" | "connected" | "host-ui";
  deviceName?: string;
};

export const useStatusStore = defineStore("status", {
  state: () => ({
    rtlSdrDetected: false,
    aaStatus: { status: "disconnected" } as AaStatus,
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
