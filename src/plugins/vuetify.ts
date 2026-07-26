import { createVuetify } from "vuetify";
import { buildM3Theme } from "@/composables/useM3Theme";

export const createAppVuetify = (seed: string) =>
  createVuetify({
    theme: {
      defaultTheme: "m3dark",
      themes: {
        m3light: {
          ...buildM3Theme(seed, false),
          variables: {
            "border-radius-root": "16px",
            "medium-emphasis-opacity": "0.74",
            "overlay-opacity": "0.08",
          },
        },
        m3dark: {
          ...buildM3Theme(seed, true),
          variables: {
            "border-radius-root": "16px",
            "medium-emphasis-opacity": "0.74",
            "overlay-opacity": "0.12",
          },
        },
      },
    },
    defaults: {
      VBtn: { rounded: "pill", elevation: 0 },
      VCard: { rounded: "xl", elevation: 0, color: "surface-container" },
      VTextField: { variant: "outlined", rounded: "lg" },
      VSelect: { variant: "outlined", rounded: "lg" },
      VChip: { rounded: "sm" },
      VNavigationDrawer: { color: "surface-container-low" },
      VAppBar: { color: "surface", elevation: 0 },
      VBottomNavigation: { color: "surface-container", elevation: 0 },
      VDialog: { rounded: "xl" },
      VList: { bgColor: "surface-container-low" },
      VListItem: { rounded: "xl" },
      VFab: { rounded: "lg", elevation: 3 },
    },
  });
