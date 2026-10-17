import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  CountdownPayload,
  RunPhase,
  RuntimeEventPayload,
  TickPayload,
} from "../types";

/**
 * Mirror of the Rust engine. Never drives clicks: it only reflects the
 * 10 Hz `runtime:tick` stream and the lifecycle events.
 */
export const useRuntimeStore = defineStore("runtime", () => {
  const phase = ref<RunPhase>("idle");
  const clicks = ref(0);
  const elapsedMs = ref(0);
  const cps = ref(0);
  const countdownRemainingMs = ref(0);
  const countdownTotalMs = ref(0);
  const lastEvent = ref<RuntimeEventPayload | null>(null);

  let attached = false;
  let unlisten: UnlistenFn[] = [];

  function resetCounters(): void {
    clicks.value = 0;
    elapsedMs.value = 0;
    cps.value = 0;
  }

  function applyEvent(payload: RuntimeEventPayload): void {
    lastEvent.value = payload;
    switch (payload.event) {
      case "started":
        phase.value = "running";
        resetCounters();
        break;
      case "countdownStarted":
      case "scheduled":
        phase.value = "countdown";
        resetCounters();
        break;
      case "stopped":
      case "error":
        phase.value = "idle";
        countdownRemainingMs.value = 0;
        break;
      default:
        break;
    }
  }

  async function attach(): Promise<void> {
    if (attached || !isTauri()) return;
    attached = true;

    unlisten.push(
      await listen<TickPayload>("runtime:tick", ({ payload }) => {
        clicks.value = payload.clicks;
        elapsedMs.value = payload.elapsedMs;
        cps.value = payload.cps;
        if (phase.value !== "running") phase.value = "running";
      }),
    );

    unlisten.push(
      await listen<CountdownPayload>("runtime:countdown", ({ payload }) => {
        countdownRemainingMs.value = payload.remainingMs;
        countdownTotalMs.value = payload.totalMs;
        // Remaining 0 means the delay is over and clicking has begun.
        phase.value = payload.remainingMs > 0 ? "countdown" : "running";
        if (payload.remainingMs > 0) resetCounters();
      }),
    );

    unlisten.push(
      await listen<RuntimeEventPayload>("runtime:event", ({ payload }) => applyEvent(payload)),
    );

    if (await invoke<boolean>("is_running")) {
      phase.value = "running";
    }
  }

  async function detach(): Promise<void> {
    for (const off of unlisten) off();
    unlisten = [];
    attached = false;
  }

  async function start(): Promise<void> {
    await invoke("start_clicking");
  }

  async function stop(): Promise<void> {
    await invoke("stop_clicking");
  }

  return {
    phase,
    clicks,
    elapsedMs,
    cps,
    countdownRemainingMs,
    countdownTotalMs,
    lastEvent,
    attach,
    detach,
    start,
    stop,
  };
});
