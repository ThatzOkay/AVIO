import { createApp } from "vue";
import App from "./App.vue";

import '@mdi/font/css/materialdesignicons.css'

import 'vuetify/styles'
import { createVuetify } from 'vuetify'
import router from "./router/index.ts";
import { createPinia } from "pinia";

const vuetify = createVuetify({
  theme: {
    defaultTheme: 'dark'
  }
})
  
const pinia = createPinia()

const app = createApp(App)
.use(router)
.use(vuetify)
.use(pinia)

router.isReady().then(async () => {

app.mount("#app");
});
