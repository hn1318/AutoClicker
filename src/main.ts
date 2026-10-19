import { createApp } from "vue";
import { createPinia } from "pinia";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App.vue";
import CursorOverlay from "./components/CursorOverlay.vue";
import { applyLocale, i18n } from "./locales";
import { initTheme } from "./composables/useTheme";
import { useSettingsStore } from "./stores/settings";

import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/transitions.css";

/** The overlay is a separate, transparent, click-through window. */
function isCursorOverlay(): boolean {
  return isTauri() && getCurrentWindow().label === "cursor-overlay";
}

async function bootstrap(): Promise<void> {
  if (isCursorOverlay()) {
    // No stores, no router: the overlay only mirrors a coordinate stream.
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    document.body.style.overflow = "hidden";
    window.addEventListener("contextmenu", (event) => event.preventDefault());
    createApp(CursorOverlay).mount("#app");
    return;
  }

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
