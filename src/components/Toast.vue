<script setup lang="ts">
import type { Component } from "vue";
import { CircleAlert, Check, Info, X } from "@lucide/vue";
import { useToast, type ToastKind } from "../composables/useToast";

const { items, dismiss } = useToast();

const icons: Record<ToastKind, Component> = {
  info: Info,
  success: Check,
  error: CircleAlert,
};
</script>

<template>
  <TransitionGroup tag="div" name="toast" class="toasts">
    <div
      v-for="item in items"
      :key="item.id"
      class="toast"
      :class="`toast--${item.kind}`"
      role="status"
      aria-live="polite"
    >
      <component :is="icons[item.kind]" class="toast__icon" :size="14" :stroke-width="1.5" />
      <span class="toast__text">{{ item.message }}</span>
      <button type="button" class="toast__close" @click="dismiss(item.id)">
        <X :size="12" :stroke-width="1.5" />
      </button>
    </div>
  </TransitionGroup>
</template>

<style scoped>
.toasts {
  position: fixed;
  right: var(--sp-6);
  bottom: calc(var(--statusbar-h) + var(--sp-6));
  left: var(--sp-6);
  z-index: var(--z-toast);
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
  pointer-events: none;
}

.toast {
  display: flex;
  align-items: flex-start;
  gap: var(--sp-3);
  padding: var(--sp-4) var(--sp-5);
  border: 1px solid var(--border-strong);
  border-left-width: 2px;
  border-radius: var(--r-btn);
  background: var(--surface);
  pointer-events: auto;
}

.toast__icon {
  margin-top: 1px;
  color: var(--text-muted);
}

.toast--success {
  border-left-color: var(--success);
}

.toast--success .toast__icon {
  color: var(--success);
}

.toast--error {
  border-left-color: var(--danger);
}

.toast--error .toast__icon {
  color: var(--danger);
}

.toast--info {
  border-left-color: var(--text-faint);
}

.toast__text {
  flex: 1 1 auto;
  min-width: 0;
  font-size: var(--fs-12);
  line-height: 1.4;
  color: var(--text);
  overflow-wrap: anywhere;
}

.toast__close {
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: var(--r-tag);
  color: var(--text-faint);
  transition:
    background-color var(--dur-2) var(--ease-out),
    color var(--dur-2) var(--ease-out);
}

.toast__close:hover {
  background: var(--surface-2);
  color: var(--text);
}
</style>
