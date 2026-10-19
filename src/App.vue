<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager";
import TitleBar from "./components/TitleBar.vue";
import Toast from "./components/Toast.vue";
import ContextMenu from "./components/ContextMenu.vue";
import MainView from "./views/MainView.vue";
import SettingsView from "./views/SettingsView.vue";
import { useRuntimeStore } from "./stores/runtime";
import { useSettingsStore } from "./stores/settings";
import { compareVersions } from "./utils/format";
import { fetchLatestRelease } from "./utils/update";
import { useToast } from "./composables/useToast";
import { useErrors } from "./composables/useErrors";
import { useContextMenu } from "./composables/useContextMenu";
import type { RuntimeEventPayload } from "./types";

const { t } = useI18n();
const runtime = useRuntimeStore();
const settingsStore = useSettingsStore();
const toast = useToast();
const errors = useErrors();

const view = ref<"main" | "settings">("main");
const contextMenu = useContextMenu();

type EditableField = HTMLInputElement | HTMLTextAreaElement;

function editableField(target: EventTarget | null): EditableField | null {
  if (!(target instanceof HTMLElement)) return null;
  const field = target.closest("input, textarea");
  if (field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement) {
    return field.disabled ? null : field;
  }
  return null;
}

function insertText(field: EditableField, text: string): void {
  field.focus();
  const start = field.selectionStart ?? field.value.length;
  const end = field.selectionEnd ?? start;
  field.setRangeText(text, start, end, "end");
  field.dispatchEvent(new Event("input", { bubbles: true }));
}

function clearField(field: EditableField): void {
  field.focus();
  field.value = "";
  field.dispatchEvent(new Event("input", { bubbles: true }));
}

/**
 * WebView2 ships a browser context menu (refresh / save as / print). Suppress it
 * everywhere and offer a small editing menu on text fields instead.
 */
function onContextMenu(event: MouseEvent): void {
  event.preventDefault();

  const field = editableField(event.target);
  if (!field) {
    contextMenu.hide();
    return;
  }

  const selected = field.value.slice(field.selectionStart ?? 0, field.selectionEnd ?? 0);
  const numeric = field.getAttribute("inputmode") === "decimal";
  const hasValue = field.value.length > 0;

  contextMenu.show(event.clientX, event.clientY, [
    {
      id: "copy",
      label: t("contextMenu.copy"),
      disabled: !hasValue,
      run: async () => {
        try {
          await writeText(selected || field.value);
        } catch (error) {
          console.warn("[autoclicker] clipboard write failed", error);
        }
      },
    },
    {
      id: "paste",
      label: t("contextMenu.paste"),
      run: async () => {
        try {
          insertText(field, await readText());
        } catch (error) {
          console.warn("[autoclicker] clipboard read failed", error);
        }
      },
    },
    {
      id: "selectAll",
      label: t("contextMenu.selectAll"),
      disabled: !hasValue,
      run: () => field.select(),
    },
    {
      id: "clear",
      label: t("contextMenu.clear"),
      // Numeric fields snap back to their last valid value on blur, so clearing
      // them is deliberately not offered.
      disabled: numeric || !hasValue,
      run: () => clearField(field),
    },
  ]);
}

/**
 * The overlay window is owned by Rust; the UI only decides whether it should be
 * visible: always, only while clicking, or never.
 */
async function syncCursorTracking(): Promise<void> {
  if (!isTauri()) return;
  const mode = settingsStore.settings.cursorTrackMode;
  const active = mode === "always" || (mode === "running" && runtime.phase !== "idle");
  try {
    await invoke("set_cursor_tracking", { active });
  } catch (error) {
    console.warn("[autoclicker] cursor tracking unavailable", error);
  }
}

watch(
  [() => settingsStore.settings.cursorTrackMode, () => runtime.phase],
  () => void syncCursorTracking(),
  { immediate: true },
);

let lastEscape = 0;

/** Double-Esc emergency stop, only while the window has focus. */
function onKeydown(event: KeyboardEvent): void {
  if (event.key !== "Escape" || view.value !== "main") return;
  const now = performance.now();
  if (now - lastEscape <= 500) {
    lastEscape = 0;
    if (runtime.phase === "idle") return;
    void runtime.stop().catch((error: unknown) => toast.error(errors.from(error)));
    return;
  }
  lastEscape = now;
}

/**
 * Background update check: one quiet probe per launch, delayed so it never
 * competes with startup. Failures stay silent — the Settings page has the
 * explicit button with error reporting.
 */
async function autoCheckUpdate(): Promise<void> {
  if (!isTauri()) return;
  try {
    const [candidate, info] = await Promise.all([
      fetchLatestRelease(settingsStore.settings.updateUrl),
      invoke<{ version: string }>("get_app_info"),
    ]);
    if (compareVersions(candidate.version, info.version) > 0) {
      toast.info(t("update.available", { version: candidate.version }));
    }
  } catch (error) {
    console.warn("[autoclicker] background update check skipped", error);
  }
}

let updateTimer: number | undefined;

onMounted(async () => {
  window.addEventListener("keydown", onKeydown);
  window.addEventListener("contextmenu", onContextMenu);
  await runtime.attach();
  updateTimer = window.setTimeout(() => void autoCheckUpdate(), 8_000);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
  window.removeEventListener("contextmenu", onContextMenu);
  if (updateTimer !== undefined) window.clearTimeout(updateTimer);
});

const lastEvent = computed<RuntimeEventPayload | null>(() => runtime.lastEvent);

watch(lastEvent, (payload) => {
  if (!payload || payload.event !== "error") return;
  const rawCode = payload.params.code;
  const rawParams = payload.params.params;
  toast.error(
    errors.from({
      code: typeof rawCode === "string" ? rawCode : "unknown",
      params:
        typeof rawParams === "object" && rawParams !== null
          ? (rawParams as Record<string, unknown>)
          : {},
    }),
  );
});

async function minimize(): Promise<void> {
  if (!isTauri()) return;
  await getCurrentWindow().minimize();
}

async function close(): Promise<void> {
  if (!isTauri()) return;
  await getCurrentWindow().close();
}

function toggleSettings(): void {
  view.value = view.value === "settings" ? "main" : "settings";
}
</script>

<template>
  <div class="app-shell">
    <TitleBar
      show-settings
      :settings-active="view === 'settings'"
      :title="t('app.name')"
      @open-settings="toggleSettings"
      @minimize="minimize"
      @close="close"
    />

    <MainView v-if="view === 'main'" />
    <SettingsView v-else />

    <Toast />
    <ContextMenu />
  </div>
</template>
