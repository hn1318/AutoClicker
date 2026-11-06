import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useSettingsStore } from "./settings";
import type { UpdateInfo, UpdateProgress } from "../types";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "installing";

/**
 * Thin wrapper over the Rust updater commands. Downloading and installing are
 * separate calls on purpose: installing closes the app, so it only ever runs
 * when the user clicks restart.
 */
export const useUpdateStore = defineStore("update", () => {
  const settingsStore = useSettingsStore();

  const status = ref<UpdateStatus>("idle");
  const info = ref<UpdateInfo | null>(null);
  const downloaded = ref(0);
  const total = ref<number | null>(null);

  let attached = false;
  let unlisten: UnlistenFn | undefined;

  const percent = computed(() =>
    total.value ? Math.min(100, Math.round((downloaded.value / total.value) * 100)) : 0,
  );
  const busy = computed(
    () =>
      status.value === "checking" ||
      status.value === "downloading" ||
      status.value === "installing",
  );
  const ready = computed(() => status.value === "ready" || status.value === "installing");

  async function attach(): Promise<void> {
    if (attached || !isTauri()) return;
    attached = true;
    unlisten = await listen<UpdateProgress>("update:progress", ({ payload }) => {
      downloaded.value = payload.downloaded;
      total.value = payload.total;
    });
  }

  function detach(): void {
    unlisten?.();
    unlisten = undefined;
    attached = false;
  }

  /** Resolves with the newer release, or `null` when the app is current. */
  async function check(): Promise<UpdateInfo | null> {
    if (!isTauri()) return null;
    await attach();
    status.value = "checking";
    try {
      const found = await invoke<UpdateInfo | null>("check_update", {
        source: settingsStore.settings.updateUrl.trim(),
      });
      info.value = found;
      status.value = found ? "available" : "idle";
      return found;
    } catch (error) {
      status.value = "idle";
      throw error;
    }
  }

  /** Downloads and verifies the installer; nothing is installed yet. */
  async function download(): Promise<UpdateInfo> {
    if (!isTauri()) throw new Error("updates require the desktop app");
    await attach();
    status.value = "downloading";
    downloaded.value = 0;
    total.value = null;
    try {
      const payload = await invoke<UpdateInfo>("download_update", {
        source: settingsStore.settings.updateUrl.trim(),
      });
      info.value = payload;
      status.value = "ready";
      total.value = null;
      return payload;
    } catch (error) {
      status.value = info.value ? "available" : "idle";
      throw error;
    }
  }

  /** Runs the cached installer. On Windows this exits and relaunches the app. */
  async function install(): Promise<void> {
    status.value = "installing";
    try {
      await invoke("install_update");
    } catch (error) {
      status.value = "ready";
      throw error;
    }
  }

  function dismiss(): void {
    info.value = null;
    status.value = "idle";
  }

  return {
    status,
    info,
    downloaded,
    total,
    percent,
    busy,
    ready,
    attach,
    detach,
    check,
    download,
    install,
    dismiss,
  };
});
