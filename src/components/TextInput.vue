<script setup lang="ts">
interface Props {
  placeholder?: string;
  disabled?: boolean;
  ariaLabel?: string;
  width?: string;
  mono?: boolean;
}

withDefaults(defineProps<Props>(), {
  disabled: false,
  width: "100%",
  mono: false,
});

const model = defineModel<string>({ required: true });

function onInput(event: Event): void {
  model.value = (event.target as HTMLInputElement).value;
}
</script>

<template>
  <div class="text-input" :class="{ 'is-disabled': disabled }" :style="{ width }">
    <input
      class="text-input__field"
      :class="{ 'text-input__field--mono': mono }"
      type="text"
      autocomplete="off"
      spellcheck="false"
      :value="model"
      :placeholder="placeholder"
      :disabled="disabled"
      :aria-label="ariaLabel"
      @input="onInput"
    />
  </div>
</template>

<style scoped>
.text-input {
  display: flex;
  align-items: center;
  height: var(--control-h);
  padding: 0 var(--sp-5);
  border: 1px solid var(--border-strong);
  border-radius: var(--r-input);
  background: var(--surface);
  transition:
    border-color var(--dur-2) var(--ease-out),
    box-shadow var(--dur-2) var(--ease-out),
    background-color var(--dur-2) var(--ease-out);
}

.text-input:hover:not(.is-disabled) {
  border-color: var(--text-faint);
}

.text-input:focus-within {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-soft);
}

.text-input.is-disabled {
  background: var(--surface-2);
}

.text-input__field {
  width: 100%;
  min-width: 0;
  border: 0;
  background: transparent;
  color: var(--text);
  font-size: var(--fs-12);
  outline: none;
}

.text-input__field--mono {
  font-family: var(--font-mono);
  font-size: var(--fs-11);
}

.text-input__field::placeholder {
  color: var(--text-faint);
}

.text-input__field:disabled {
  color: var(--text-faint);
}
</style>
