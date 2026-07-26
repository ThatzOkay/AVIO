import { createApp } from "vue";
import App from "./App.vue";

import "@mdi/font/css/materialdesignicons.css";

import { injectM3CssVars } from "./composables/useM3Theme";
import { createAppVuetify } from "./plugins/vuetify";
import "vuetify/styles";
import { createPinia } from "pinia";
import router from "./router/index.ts";
import "@mdi/font/css/materialdesignicons.css";

const SEED = "#1B6EF3";

const pinia = createPinia();
const vuetify = createAppVuetify(SEED);

const app = createApp(App).use(router).use(vuetify).use(pinia);

injectM3CssVars(SEED, true);

router.isReady().then(async () => {
  app.use(vuetify);
  app.mount("#app");
});
