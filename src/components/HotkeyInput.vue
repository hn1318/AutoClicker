<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { Keyboard } from "@lucide/vue";

interface Props {
  modelValue: string;
  disabled?: boolean;
  ariaLabel?: string;
  /** Allow a single key (no modifier) — used for keyboard-repeat mode. */
  allowBareKeys?: boolean;
}

const props = withDefaults(defineProps<Props>(), { disabled: false, allowBareKeys: false });

const emit = defineEmits<{
  change: [value: string];
}>();

const { t } = useI18n();
const recording = ref(false);

const keys = computed(() => props.modelValue.split("+").filter(Boolean));

const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta", "AltGraph", "OS"]);

/**
 * Build an accelerator string understood by the Rust shortcut parser.
 * Returns "" to cancel, null to keep waiting (modifier only / unsupported key).
 */
function toAccelerator(event: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(event.key)) return null;
  if (event.key === "Escape") return "";

  const key = event.key;
  let main = "";
  if (/^F([1-9]|1[0-9]|2[0-4])$/i.test(key)) main = key.toUpperCase();
  else if (/^[a-zA-Z]$/.test(key)) main = key.toUpperCase();
  else if (/^[0-9]$/.test(key)) main = key;
  else if (key === " ") main = "Space";
  else if (key === "Enter") main = "Enter";
  else if (key === "Tab") main = "Tab";
  else return null;

  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Control");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Super");

  // A bare letter or digit would swallow normal typing system-wide.
  if (parts.length === 0 && !main.startsWith("F") && !props.allowBareKeys) return null;

  parts.push(main);
  return parts.join("+");
}

function onKeydown(event: KeyboardEvent): void {
  if (!recording.value) return;
  event.preventDefault();
  event.stopPropagation();

  const accelerator = toAccelerator(event);
  if (accelerator === null) return;

  recording.value = false;
  if (accelerator === "" || accelerator === props.modelValue) return;
  emit("change", accelerator);
}

function stopRecording(): void {
  recording.value = false;
}

function toggleRecording(): void {
  if (props.disabled) return;
  recording.value = !recording.value;
}

watch(recording, (active) => {
  if (active) window.addEventListener("keydown", onKeydown, true);
  else window.removeEventListener("keydown", onKeydown, true);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown, true);
});
</script>

<template>
  <button
    type="button"
    class="hotkey"
    :class="{ 'is-recording': recording }"
    :disabled="disabled"
    :aria-label="ariaLabel"
    @click="toggleRecording"
    @blur="stopRecording"
  >
    <Keyboard class="hotkey__icon" :size="13" :stroke-width="1.5" />
    <span v-if="recording" class="hotkey__hint">{{ t("hotkeys.press") }}</span>
    <kbd v-for="key in keys" v-else :key="key" class="hotkey__key">{{ key }}</kbd>
  </button>
</template>

<style scoped>
.hotkey {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
  min-width: 96px;
  height: var(--control-h);
  padding: 0 var(--sp-4);
  border: 1px solid var(--border-strong);
  border-radius: var(--r-input);
  background: var(--surface);
  color: var(--text);
  transition:
    border-color var(--dur-2) var(--ease-out),
    background-color var(--dur-2) var(--ease-out),
    box-shadow var(--dur-2) var(--ease-out);
}

.hotkey:hover:not(:disabled) {
  border-color: var(--text-faint);
}

.hotkey.is-recording {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-soft);
}

.hotkey:focus-visible {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-soft);
}

.hotkey:disabled {
  opacity: 0.5;
}

.hotkey__icon {
  color: var(--text-faint);
}

.hotkey.is-recording .hotkey__icon {
  color: var(--accent);
}

.hotkey__key {
  padding: 1px var(--sp-3);
  border: 1px solid var(--border);
  border-radius: var(--r-tag);
  background: var(--surface-2);
  color: var(--text-muted);
  font-family: var(--font-sans);
  font-size: var(--fs-11);
  font-weight: var(--fw-medium);
  line-height: 1.5;
}

.hotkey__hint {
  color: var(--accent);
  font-size: var(--fs-11);
  white-space: nowrap;
}
</style>
