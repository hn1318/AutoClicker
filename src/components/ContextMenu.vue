<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useContextMenu } from "../composables/useContextMenu";

const { state, hide } = useContextMenu();

const root = ref<HTMLElement | null>(null);
const position = ref({ left: 0, top: 0 });
const active = ref(-1);

watch(
  () => state.open,
  async (open) => {
    if (!open) {
      active.value = -1;
      return;
    }
    position.value = { left: state.x, top: state.y };
    await nextTick();
    clampIntoView();
    active.value = state.items.findIndex((item) => !item.disabled);
  },
);

/** Flip the menu when it would overflow the window. */
function clampIntoView(): void {
  const element = root.value;
  if (!element) return;
  const rect = element.getBoundingClientRect();
  const margin = 8;
  let left = state.x;
  let top = state.y;
  if (left + rect.width + margin > window.innerWidth) {
    left = Math.max(margin, state.x - rect.width);
  }
  if (top + rect.height + margin > window.innerHeight) {
    top = Math.max(margin, state.y - rect.height);
  }
  position.value = { left, top };
}

async function run(index: number): Promise<void> {
  const item = state.items[index];
  if (!item || item.disabled) return;
  hide();
  await item.run();
}

function move(step: 1 | -1): void {
  const count = state.items.length;
  if (count === 0) return;
  let index = active.value;
  for (let i = 0; i < count; i += 1) {
    index = (index + step + count) % count;
    if (!state.items[index]?.disabled) {
      active.value = index;
      return;
    }
  }
}

function onPointerDown(event: MouseEvent): void {
  if (!state.open) return;
  if (root.value && event.target instanceof Node && root.value.contains(event.target)) return;
  hide();
}

function onKeydown(event: KeyboardEvent): void {
  if (!state.open) return;
  switch (event.key) {
    case "Escape":
      // Keep the app-level double-Esc emergency stop from firing.
      event.preventDefault();
      event.stopPropagation();
      hide();
      break;
    case "ArrowDown":
      event.preventDefault();
      move(1);
      break;
    case "ArrowUp":
      event.preventDefault();
      move(-1);
      break;
    case "Enter":
      event.preventDefault();
      void run(active.value);
      break;
    default:
      break;
  }
}

function onWindowChange(): void {
  if (state.open) hide();
}

onMounted(() => {
  // Capture phase: close before the click lands anywhere else.
  window.addEventListener("mousedown", onPointerDown, true);
  window.addEventListener("keydown", onKeydown, true);
  window.addEventListener("resize", onWindowChange);
  window.addEventListener("blur", onWindowChange);
});

onBeforeUnmount(() => {
  window.removeEventListener("mousedown", onPointerDown, true);
  window.removeEventListener("keydown", onKeydown, true);
  window.removeEventListener("resize", onWindowChange);
  window.removeEventListener("blur", onWindowChange);
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="state.open"
      ref="root"
      class="ctx"
      role="menu"
      :style="{ left: `${position.left}px`, top: `${position.top}px` }"
      @contextmenu.prevent.stop
      @mousedown.prevent
    >
      <button
        v-for="(item, index) in state.items"
        :key="item.id"
        type="button"
        role="menuitem"
        class="ctx__item"
        :class="{ 'is-active': index === active }"
        :disabled="item.disabled"
        @mouseenter="active = index"
        @click="run(index)"
      >
        {{ item.label }}
      </button>
    </div>
  </Teleport>
</template>

<style scoped>
.ctx {
  position: fixed;
  z-index: 3000;
  display: flex;
  flex-direction: column;
  min-width: 132px;
  padding: var(--sp-2, 4px);
  border: 1px solid var(--border-strong);
  border-radius: var(--r-card, 10px);
  background: var(--surface);
  box-shadow:
    0 12px 28px rgba(0, 0, 0, 0.28),
    0 2px 6px rgba(0, 0, 0, 0.16);
  animation: ctx-in var(--dur-1, 90ms) var(--ease-out, ease-out);
}

@keyframes ctx-in {
  from {
    opacity: 0;
    transform: translateY(-2px);
  }

  to {
    opacity: 1;
    transform: none;
  }
}

.ctx__item {
  display: flex;
  align-items: center;
  height: 28px;
  padding: 0 var(--sp-5, 10px);
  border: 0;
  border-radius: calc(var(--r-card, 10px) - 4px);
  background: none;
  color: var(--text);
  font-size: var(--fs-12, 12px);
  text-align: left;
  white-space: nowrap;
  cursor: default;
}

.ctx__item.is-active:not(:disabled) {
  background: var(--surface-2);
}

.ctx__item:disabled {
  color: var(--text-faint);
}
</style>
