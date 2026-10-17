<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { Play, X } from "@lucide/vue";
import Button from "./Button.vue";
import { formatCps, formatDuration } from "../utils/format";
import type { RunPhase } from "../types";

interface Props {
  phase: RunPhase;
  clicks: number;
  elapsedMs: number;
  cps: number;
  countdownRemainingMs: number;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  toggle: [];
}>();

const { t } = useI18n();

const hasStats = computed(() => props.clicks > 0 || props.phase !== "idle");

const variant = computed(() => {
  if (props.phase === "running") return "danger" as const;
  if (props.phase === "countdown") return "secondary" as const;
  return "primary" as const;
});

const label = computed(() => {
  if (props.phase === "running") return t("common.stop");
  if (props.phase === "countdown") return t("common.cancel");
  return t("common.start");
});

const countdownSeconds = computed(() => Math.max(1, Math.ceil(props.countdownRemainingMs / 1000)));
</script>

<template>
  <footer class="actionbar">
    <div class="actionbar__status num">
      <span class="status-dot" :class="`status-dot--${phase}`" />
      <template v-if="phase === 'countdown'">
        {{ t("status.countdown", { value: countdownSeconds }) }}
      </template>
      <template v-else-if="!hasStats">{{ t("status.idle") }}</template>
      <template v-else>
        <span>{{ t("status.clicks", { count: clicks }) }}</span>
        <span class="sep">·</span>
        <span>{{ t("status.duration", { value: formatDuration(elapsedMs) }) }}</span>
        <span class="sep">·</span>
        <span>{{ t("status.cps", { value: formatCps(cps) }) }}</span>
      </template>
    </div>

    <Button :variant="variant" @click="emit('toggle')">
      <template #icon>
        <span v-if="phase === 'running'" class="run-dot dot-pulse" />
        <X v-else-if="phase === 'countdown'" :size="13" :stroke-width="1.5" />
        <Play v-else :size="13" :stroke-width="1.5" />
      </template>
      {{ label }}
    </Button>
  </footer>
</template>

<style scoped>
.actionbar {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-6);
  height: var(--statusbar-h);
  padding: 0 var(--sp-6) 0 var(--sp-7);
  border-top: 1px solid var(--border);
  background: var(--surface);
}

.actionbar__status {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  min-width: 0;
  overflow: hidden;
  color: var(--text-muted);
  font-size: var(--fs-11);
  white-space: nowrap;
  text-overflow: ellipsis;
}

.status-dot {
  flex: none;
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--text-faint);
  transition: background-color var(--dur-2) var(--ease-out);
}

.status-dot--countdown,
.status-dot--running {
  background: var(--accent);
}

.status-dot--running {
  animation: pulse-dot 1.6s var(--ease-out) infinite;
}

.run-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentcolor;
}

.sep {
  color: var(--text-faint);
}
</style>
