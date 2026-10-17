<script setup lang="ts">
import { computed } from "vue";
import type { CSSProperties } from "vue";
import type { ControlSize, SegmentedOption } from "../types";

interface Props {
  options: SegmentedOption[];
  size?: ControlSize;
  block?: boolean;
  disabled?: boolean;
  ariaLabel?: string;
}

const props = withDefaults(defineProps<Props>(), {
  size: "md",
  block: false,
  disabled: false,
});

const model = defineModel<string>({ required: true });

const activeIndex = computed(() => {
  const index = props.options.findIndex((option) => option.value === model.value);
  return index < 0 ? 0 : index;
});

const thumbStyle = computed<CSSProperties>(() => ({
  "--seg-count": String(Math.max(props.options.length, 1)),
  "--seg-index": String(activeIndex.value),
}));

const classes = computed(() => [`seg--${props.size}`, { "seg--block": props.block }]);

function isDisabled(option: SegmentedOption): boolean {
  return props.disabled || option.disabled === true;
}

function select(value: string): void {
  if (value === model.value) return;
  model.value = value;
}

function move(direction: 1 | -1): void {
  if (props.disabled) return;
  const total = props.options.length;
  let index = activeIndex.value;
  for (let step = 0; step < total; step += 1) {
    index = (index + direction + total) % total;
    const candidate = props.options[index];
    if (candidate && !isDisabled(candidate)) {
      select(candidate.value);
      return;
    }
  }
}
</script>

<template>
  <div
    class="seg"
    :class="classes"
    :style="thumbStyle"
    role="radiogroup"
    :aria-label="ariaLabel"
    @keydown.left.prevent="move(-1)"
    @keydown.right.prevent="move(1)"
  >
    <span class="seg__thumb" aria-hidden="true" />
    <button
      v-for="option in options"
      :key="option.value"
      type="button"
      class="seg__item"
      :class="{ 'is-active': option.value === model, 'seg__item--icon': !option.label }"
      role="radio"
      :aria-checked="option.value === model"
      :aria-label="option.ariaLabel"
      :title="option.label ?? option.ariaLabel"
      :disabled="isDisabled(option)"
      @click="select(option.value)"
    >
      <component :is="option.icon" v-if="option.icon" :size="14" :stroke-width="1.5" />
      <span v-if="option.label" class="seg__label">{{ option.label }}</span>
    </button>
  </div>
</template>

<style scoped>
/*
 * Grid with `auto-columns: 1fr` + `width: max-content` gives every option the
 * width of the widest label — equal segments that never truncate the text.
 */
.seg {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: 1fr;
  width: max-content;
  max-width: 100%;
  padding: 3px;
  border-radius: var(--r-input);
  background: var(--surface-2);
  user-select: none;
}

.seg--block {
  width: 100%;
}

.seg__thumb {
  position: absolute;
  top: 3px;
  bottom: 3px;
  left: 3px;
  width: calc((100% - 6px) / var(--seg-count));
  border: 1px solid var(--border-strong);
  border-radius: var(--r-tag);
  background: var(--surface);
  transform: translateX(calc(var(--seg-index) * 100%));
  transition: transform var(--dur-2) var(--ease-out);
}

.seg {
  position: relative;
}

.seg__item {
  position: relative;
  z-index: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--sp-2);
  min-width: 0;
  border-radius: var(--r-tag);
  color: var(--text-muted);
  font-size: var(--fs-12);
  font-weight: var(--fw-medium);
  line-height: 1;
  white-space: nowrap;
  transition:
    color var(--dur-2) var(--ease-out),
    opacity var(--dur-2) var(--ease-out);
}

.seg__item:hover:not(:disabled) {
  color: var(--text);
}

.seg__item.is-active {
  color: var(--text);
}

.seg__item:disabled {
  opacity: 0.45;
}

.seg__item:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}

.seg__label {
  overflow: hidden;
  text-overflow: ellipsis;
}

.seg--md .seg__item {
  height: var(--control-h-sm);
  padding: 0 var(--sp-5);
}

.seg--md .seg__item--icon {
  padding: 0 var(--sp-4);
}

.seg--sm {
  padding: 2px;
}

.seg--sm .seg__thumb {
  top: 2px;
  bottom: 2px;
  left: 2px;
  width: calc((100% - 4px) / var(--seg-count));
}

.seg--sm .seg__item {
  height: 22px;
  padding: 0 var(--sp-4);
  font-size: var(--fs-11);
}
</style>
