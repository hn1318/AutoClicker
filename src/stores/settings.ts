import { defineStore } from "pinia";
import { ref, watch } from "vue";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { load, type Store } from "@tauri-apps/plugin-store";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import type { AppLanguage, Settings, ThemeMode } from "../types";
import { applyLocale } from "../locales";
import { applyThemeMode } from "../composables/useTheme";

const STORE_FILE = "settings.json";
const SETTINGS_KEY = "settings";
const SAVE_DEBOUNCE_MS = 300;

export function createDefaultSettings(): Settings {
  return {
    intervalMs: 100,
    limitEnabled: false,
    clickLimit: 100,
    button: "left",
    clickType: "single",
    holdMs: 40,
    positionMode: "cursor",
    posX: 0,
    posY: 0,
    cursorTrackMode: "running",
    countdownSec: 0,
    jitterIntervalMs: 0,
    jitterPositionPx: 0,
    curve: "constant",
    curveEndMs: 10,
    curveMinMs: 50,
    curveMaxMs: 150,
    sequenceEnabled: false,
    sequenceLoop: true,
    sequence: [],
    scheduleDelaySec: 0,
    scheduleStopSec: 0,
    keyboardEnabled: false,
    keyboardKey: "Space",
    windowBindEnabled: false,
    windowTitle: "",
    windowProcess: "",
    toggleHotkey: "F6",
    abortHotkey: "F7",
    theme: "system",
    language: "system",
    autostart: false,
    closeToTray: true,
    notifyFinish: false,
    updateUrl: "",
  };
}

export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<Settings>(createDefaultSettings());
  const ready = ref(false);

  let fileStore: Store | null = null;
  let saveTimer: number | undefined;
  let hydrating = true;

  async function loadFromDisk(): Promise<void> {
    if (isTauri()) {
      try {
        fileStore = await load(STORE_FILE, { autoSave: false });
        const saved = await fileStore.get<Settings>(SETTINGS_KEY);
        if (saved) {
          settings.value = { ...createDefaultSettings(), ...saved };
        }
      } catch (error) {
        console.warn("[autoclicker] failed to read stored settings", error);
      }

      // The OS is the source of truth for the autostart entry (e.g. the user
      // may have removed it from Task Manager).
      try {
        settings.value.autostart = await isEnabled();
      } catch (error) {
        console.warn("[autoclicker] autostart state unavailable", error);
      }

      try {
        // Push the merged settings to Rust so clamping rules apply once.
        const normalized = await invoke<Settings>("save_settings", {
          settings: settings.value,
        });
        settings.value = normalized;
      } catch (error) {
        console.warn("[autoclicker] failed to sync settings with Rust", error);
      }
    }

    // Mirrored only so index.html can paint the right theme before this module
    // runs; the plugin store stays the source of truth.
    try {
      localStorage.setItem("ac.theme", settings.value.theme);
    } catch {
      /* storage may be unavailable */
    }

    hydrating = false;
    ready.value = true;
  }

  async function persist(): Promise<void> {
    if (!isTauri()) return;
    try {
      if (fileStore) {
        await fileStore.set(SETTINGS_KEY, settings.value);
        await fileStore.save();
      }
      await invoke("save_settings", { settings: settings.value });
    } catch (error) {
      console.error("[autoclicker] failed to persist settings", error);
    }
  }

  function schedulePersist(): void {
    if (hydrating) return;
    window.clearTimeout(saveTimer);
    saveTimer = window.setTimeout(() => void persist(), SAVE_DEBOUNCE_MS);
  }

  /**
   * Replace the profile with an imported one. Hotkeys live in Rust, so they are
   * re-registered explicitly; everything else flows through the normal watchers.
   */
  async function applyImported(next: Settings): Promise<void> {
    settings.value = { ...createDefaultSettings(), ...next };
    await persist();
    try {
      await invoke("set_hotkeys", {
        toggle: settings.value.toggleHotkey,
        abort: settings.value.abortHotkey,
      });
    } catch (error) {
      console.warn("[autoclicker] failed to re-register hotkeys", error);
    }
  }

  async function setHotkeys(toggle: string, abort: string): Promise<void> {
    // Rust validates + registers; a conflict is thrown before we mutate state,
    // so the UI simply keeps the old binding.
    await invoke("set_hotkeys", { toggle, abort });
    settings.value.toggleHotkey = toggle;
    settings.value.abortHotkey = abort;
  }

  function setTheme(theme: ThemeMode): void {
    settings.value.theme = theme;
  }

  function setLanguage(language: AppLanguage): void {
    settings.value.language = language;
  }

  watch(
    () => settings.value.theme,
    (theme) => {
      if (hydrating) return;
      try {
        localStorage.setItem("ac.theme", theme);
      } catch {
        /* ignore */
      }
      applyThemeMode(theme);
    },
  );

  watch(
    () => settings.value.language,
    (language) => {
      if (hydrating) return;
      applyLocale(language);
    },
  );

  watch(
    () => settings.value.autostart,
    async (enabled) => {
      if (hydrating || !isTauri()) return;
      try {
        await (enabled ? enable() : disable());
      } catch (error) {
        console.error("[autoclicker] failed to toggle autostart", error);
        // Reflect reality instead of lying to the user.
        try {
          settings.value.autostart = await isEnabled();
        } catch {
          settings.value.autostart = false;
        }
      }
    },
  );

  watch(settings, schedulePersist, { deep: true });

  return {
    settings,
    ready,
    loadFromDisk,
    persist,
    applyImported,
    setHotkeys,
    setTheme,
    setLanguage,
  };
});
