<script setup lang="ts">
import { computed } from "vue";
import type { ButtonVariant, ControlSize } from "../types";

interface Props {
  variant?: ButtonVariant;
  size?: ControlSize;
  block?: boolean;
  loading?: boolean;
  disabled?: boolean;
  iconOnly?: boolean;
  type?: "button" | "submit" | "reset";
}

const props = withDefaults(defineProps<Props>(), {
  variant: "secondary",
  size: "md",
  block: false,
  loading: false,
  disabled: false,
  iconOnly: false,
  type: "button",
});

const classes = computed(() => [
  `btn--${props.variant}`,
  `btn--${props.size}`,
  {
    "btn--block": props.block,
    "btn--icon": props.iconOnly,
    "is-loading": props.loading,
  },
]);
</script>

<template>
  <button
    class="btn"
    :class="classes"
    :type="type"
    :disabled="disabled || loading"
    :aria-busy="loading || undefined"
  >
    <span v-if="loading" class="btn__spinner spin" aria-hidden="true" />
    <slot v-else name="icon" />
    <span v-if="!iconOnly" class="btn__label"><slot /></span>
  </button>
</template>

<style scoped>
.btn {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--sp-3);
  border: 1px solid transparent;
  border-radius: var(--r-btn);
  font-size: var(--fs-13);
  font-weight: var(--fw-medium);
  line-height: 1;
  white-space: nowrap;
  user-select: none;
  transition:
    background-color var(--dur-2) var(--ease-out),
    border-color var(--dur-2) var(--ease-out),
    color var(--dur-2) var(--ease-out),
    opacity var(--dur-2) var(--ease-out),
    transform var(--dur-1) var(--ease-out);
}

.btn:active:not(:disabled) {
  transform: scale(0.97);
}

.btn:focus-visible {
  outline: none;
  box-shadow:
    0 0 0 2px var(--bg),
    0 0 0 4px var(--accent);
}

.btn:disabled {
  opacity: 0.45;
}

/* ---- sizes ---- */
.btn--md {
  height: var(--control-h);
  padding: 0 var(--sp-6);
}

.btn--sm {
  height: var(--control-h-sm);
  padding: 0 var(--sp-4);
  font-size: var(--fs-12);
}

.btn--md.btn--icon {
  width: var(--control-h);
  padding: 0;
}

.btn--sm.btn--icon {
  width: var(--control-h-sm);
  padding: 0;
}

.btn--block {
  width: 100%;
}

/* ---- variants ---- */
.btn--primary {
  background: var(--accent);
  color: var(--on-accent);
}

.btn--primary:hover:not(:disabled) {
  background: var(--accent-hover);
}

.btn--danger {
  background: var(--danger);
  color: var(--on-danger);
}

.btn--danger:hover:not(:disabled) {
  background: var(--danger-hover);
}

.btn--secondary {
  border-color: var(--border-strong);
  background: var(--surface);
  color: var(--text);
}

.btn--secondary:hover:not(:disabled) {
  background: var(--surface-2);
}

.btn--ghost {
  color: var(--text-muted);
}

.btn--ghost:hover:not(:disabled) {
  background: var(--surface-2);
  color: var(--text);
}

.btn__label {
  display: inline-flex;
  align-items: center;
}

.btn.is-loading {
  pointer-events: none;
}

.btn__spinner {
  width: 12px;
  height: 12px;
  border: 1.5px solid currentcolor;
  border-top-color: transparent;
  border-radius: 50%;
  opacity: 0.8;
}
</style>
