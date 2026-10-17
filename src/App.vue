<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import TitleBar from "./components/TitleBar.vue";
import Toast from "./components/Toast.vue";
import MainView from "./views/MainView.vue";
import SettingsView from "./views/SettingsView.vue";
import { useRuntimeStore } from "./stores/runtime";
import { useToast } from "./composables/useToast";
import { useErrors } from "./composables/useErrors";
import type { RuntimeEventPayload } from "./types";

const { t } = useI18n();
const runtime = useRuntimeStore();
const toast = useToast();
const errors = useErrors();

const view = ref<"main" | "settings">("main");

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

onMounted(async () => {
  window.addEventListener("keydown", onKeydown);
  await runtime.attach();
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
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
  </div>
</template>
