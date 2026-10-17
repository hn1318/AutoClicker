<script setup lang="ts">
interface Props {
  disabled?: boolean;
  ariaLabel?: string;
}

withDefaults(defineProps<Props>(), {
  disabled: false,
});

const model = defineModel<boolean>({ required: true });
</script>

<template>
  <button
    type="button"
    class="toggle"
    :class="{ 'is-on': model }"
    role="switch"
    :aria-checked="model"
    :aria-label="ariaLabel"
    :disabled="disabled"
    @click="model = !model"
  >
    <span class="toggle__knob" />
  </button>
</template>

<style scoped>
.toggle {
  position: relative;
  flex: none;
  width: 34px;
  height: 20px;
  border-radius: 10px;
  background: var(--border-strong);
  transition:
    background-color var(--dur-2) var(--ease-out),
    opacity var(--dur-2) var(--ease-out),
    transform var(--dur-1) var(--ease-out);
}

.toggle:active:not(:disabled) {
  transform: scale(0.97);
}

.toggle.is-on {
  background: var(--accent);
}

.toggle:disabled {
  opacity: 0.45;
}

.toggle:focus-visible {
  outline: none;
  box-shadow:
    0 0 0 2px var(--bg),
    0 0 0 4px var(--accent);
}

.toggle__knob {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 14px;
  height: 14px;
  border-radius: 7px;
  background: var(--surface);
  transition:
    transform 180ms var(--ease-spring),
    background-color var(--dur-2) var(--ease-out);
}

.toggle.is-on .toggle__knob {
  transform: translateX(14px);
  background: var(--on-accent);
}
</style>
