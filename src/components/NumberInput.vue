<script setup lang="ts">
import { ref, useTemplateRef, watch } from "vue";

interface Props {
  min?: number;
  max?: number;
  step?: number;
  precision?: number;
  unit?: string;
  width?: string;
  placeholder?: string;
  disabled?: boolean;
  ariaLabel?: string;
}

const props = withDefaults(defineProps<Props>(), {
  step: 1,
  precision: 0,
  width: "96px",
  disabled: false,
});

const model = defineModel<number>({ required: true });

const field = useTemplateRef<HTMLInputElement>("field");
const focused = ref(false);
const text = ref(formatNumber(model.value));

function formatNumber(value: number): string {
  if (!Number.isFinite(value)) return "";
  if (props.precision > 0) return String(Number(value.toFixed(props.precision)));
  return String(Math.round(value));
}

function clamp(value: number): number {
  const lo = props.min ?? Number.NEGATIVE_INFINITY;
  const hi = props.max ?? Number.POSITIVE_INFINITY;
  const bounded = Math.min(Math.max(value, lo), hi);
  const factor = 10 ** props.precision;
  return Math.round(bounded * factor) / factor;
}

watch(model, (value) => {
  if (!focused.value) text.value = formatNumber(value);
});

function sanitize(raw: string): string {
  const digitsOnly = raw.replace(/[^\d.]/g, "");
  if (props.precision === 0) return digitsOnly.replace(/\./g, "");
  const dot = digitsOnly.indexOf(".");
  if (dot === -1) return digitsOnly;
  return `${digitsOnly.slice(0, dot + 1)}${digitsOnly.slice(dot + 1).replace(/\./g, "")}`;
}

function onInput(event: Event): void {
  const target = event.target as HTMLInputElement;
  const next = sanitize(target.value);
  if (target.value !== next) target.value = next;
  text.value = next;
}

function commit(): void {
  const parsed = Number.parseFloat(text.value);
  const next = Number.isFinite(parsed) ? clamp(parsed) : model.value;
  text.value = formatNumber(next);
  if (next !== model.value) model.value = next;
}

function onFocus(): void {
  focused.value = true;
  field.value?.select();
}

function onBlur(): void {
  focused.value = false;
  commit();
}

function onArrow(direction: 1 | -1, event: KeyboardEvent): void {
  const parsed = Number.parseFloat(text.value);
  const base = Number.isFinite(parsed) ? parsed : model.value;
  const factor = event.shiftKey ? 10 : 1;
  const next = clamp(base + direction * props.step * factor);
  model.value = next;
  text.value = formatNumber(next);
}

function onEscape(): void {
  text.value = formatNumber(model.value);
  field.value?.blur();
}
</script>

<template>
  <div
    class="num-input"
    :class="{ 'is-focused': focused, 'is-disabled': disabled }"
    :style="{ width }"
  >
    <input
      ref="field"
      class="num-input__field num"
      type="text"
      inputmode="decimal"
      autocomplete="off"
      spellcheck="false"
      :value="text"
      :disabled="disabled"
      :placeholder="placeholder"
      :aria-label="ariaLabel"
      @input="onInput"
      @focus="onFocus"
      @blur="onBlur"
      @keydown.enter.prevent="commit"
      @keydown.up.prevent="onArrow(1, $event)"
      @keydown.down.prevent="onArrow(-1, $event)"
      @keydown.esc.prevent="onEscape"
      @wheel.prevent
    />
    <span v-if="unit" class="num-input__unit">{{ unit }}</span>
  </div>
</template>

<style scoped>
.num-input {
  display: flex;
  flex: none;
  align-items: center;
  height: var(--control-h);
  padding: 0 var(--sp-4) 0 var(--sp-5);
  border: 1px solid var(--border-strong);
  border-radius: var(--r-input);
  background: var(--surface);
  transition:
    border-color var(--dur-2) var(--ease-out),
    background-color var(--dur-2) var(--ease-out),
    box-shadow var(--dur-2) var(--ease-out);
}

.num-input:hover:not(.is-disabled) {
  border-color: var(--text-faint);
}

.num-input.is-focused {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-soft);
}

.num-input.is-disabled {
  background: var(--surface-2);
}

.num-input__field {
  flex: 1 1 auto;
  width: 100%;
  min-width: 0;
  height: 100%;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--text);
  font-size: var(--fs-13);
  text-align: right;
  outline: none;
}

.num-input__field::placeholder {
  color: var(--text-faint);
}

.num-input__field:disabled {
  color: var(--text-faint);
  cursor: not-allowed;
}

.num-input__unit {
  flex: none;
  padding-left: var(--sp-1);
  color: var(--text-faint);
  font-size: var(--fs-11);
  font-variant-numeric: tabular-nums;
  user-select: none;
}
</style>
