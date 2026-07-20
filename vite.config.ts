// @ts-expect-error type error without @types/node package
import process from "node:process";
import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";
import vuetify from "vite-plugin-vuetify";
import VueRouter from "vue-router/vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [
    // Kiosk app with a handful of fixed pages - no benefit to per-route code splitting, and
    // the default async chunk fetch is what makes page content visibly "pop in" after the
    // slide transition already started (independent of hardware speed).
    VueRouter({ importMode: "sync" }),
    vue(),
    vuetify({ autoImport: true }),
  ],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },

  resolve: {
    alias: {
      "@": new URL("./src", import.meta.url).pathname,
    },
  },
}));
