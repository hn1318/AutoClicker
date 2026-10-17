import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { applyLocale, i18n } from "./locales";
import { initTheme } from "./composables/useTheme";
import { useSettingsStore } from "./stores/settings";

import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/transitions.css";

async function bootstrap(): Promise<void> {
  const app = createApp(App);
  const pinia = createPinia();

  app.use(pinia).use(i18n);

  // Settings must be loaded before mount so the first paint has the right
  // theme, language and control values.
  const settingsStore = useSettingsStore(pinia);
  await settingsStore.loadFromDisk();

  initTheme(settingsStore.settings.theme);
  applyLocale(settingsStore.settings.language);

  app.mount("#app");
}

void bootstrap();
