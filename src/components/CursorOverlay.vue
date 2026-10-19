<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { CursorPositionPayload } from "../types";

/**
 * Rendered into the `cursor-overlay` webview window (see `src-tauri/src/cursor.rs`).
 * The window itself is moved by Rust at ~50 Hz; this component only mirrors the
 * coordinates, so the DOM stays idle while the pointer is still.
 */
const physicalX = ref(0);
const physicalY = ref(0);
const scale = ref(1);

let unlisten: UnlistenFn | undefined;

onMounted(async () => {
  unlisten = await listen<CursorPositionPayload>("cursor:position", ({ payload }) => {
    physicalX.value = payload.physicalX;
    physicalY.value = payload.physicalY;
    scale.value = payload.scale;
  });
});

onBeforeUnmount(() => unlisten?.());
</script>

<template>
  <div class="overlay">
    <span class="pin" aria-hidden="true" />
    <span class="coords">{{ physicalX }}, {{ physicalY }}</span>
    <span class="scale">{{ scale.toFixed(2) }}x</span>
  </div>
</template>

<style scoped>
.overlay {
  display: flex;
  align-items: center;
  gap: 6px;
  box-sizing: border-box;
  width: 100vw;
  height: 100vh;
  padding: 0 8px;
  border: 1px solid rgba(255, 255, 255, 0.18);
  border-radius: 8px;
  background: rgba(18, 18, 20, 0.86);
  color: #f2f2f4;
  font-family: var(--font-mono, ui-monospace, monospace);
  font-size: 11px;
  line-height: 1;
  white-space: nowrap;
  overflow: hidden;
  user-select: none;
}

/* Small marker in the corner closest to the pointer. */
.pin {
  flex: none;
  width: 5px;
  height: 5px;
  margin-left: 1px;
  border-radius: 50%;
  background: #4ade80;
  box-shadow: 0 0 0 2px rgba(74, 222, 128, 0.25);
}

.coords {
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.01em;
}

.scale {
  margin-left: auto;
  color: rgba(242, 242, 244, 0.55);
  font-size: 10px;
}
</style>
