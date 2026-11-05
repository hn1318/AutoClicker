<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { Circle, CircleDot, X } from "@lucide/vue";
import Button from "./Button.vue";
import Toggle from "./Toggle.vue";
import { useSettingsStore } from "../stores/settings";
import { useToast } from "../composables/useToast";
import { useErrors } from "../composables/useErrors";
import type { MouseButton, SequenceStep } from "../types";

const props = defineProps<{ locked: boolean }>();

const { t } = useI18n();
const settingsStore = useSettingsStore();
const toast = useToast();
const errors = useErrors();

const settings = computed(() => settingsStore.settings);
const steps = computed(() => settings.value.sequence);
const recording = ref(false);
const busy = ref(false);

const buttonLabels = computed<Record<MouseButton, string>>(() => ({
  left: t("mouseButton.left"),
  middle: t("mouseButton.middle"),
  right: t("mouseButton.right"),
  x1: t("mouseButton.x1"),
  x2: t("mouseButton.x2"),
}));

async function startRecording(): Promise<void> {
  busy.value = true;
  try {
    await invoke("start_recording");
    recording.value = true;
    toast.info(t("sequence.recordingHint"));
  } catch (error) {
    toast.error(errors.from(error));
  } finally {
    busy.value = false;
  }
}

async function stopRecording(): Promise<void> {
  busy.value = true;
  try {
    const recorded = await invoke<SequenceStep[]>("stop_recording");
    recording.value = false;
    if (recorded.length === 0) {
      toast.info(t("sequence.recordEmpty"));
      return;
    }
    settings.value.sequence = recorded;
    settings.value.sequenceEnabled = true;
    toast.success(t("sequence.recordDone", { count: recorded.length }));
  } catch (error) {
    recording.value = false;
    toast.error(errors.from(error));
  } finally {
    busy.value = false;
  }
}

function toggleRecording(): void {
  void (recording.value ? stopRecording() : startRecording());
}

function removeStep(index: number): void {
  settings.value.sequence.splice(index, 1);
}

function clearSteps(): void {
  settings.value.sequence = [];
}

// The recorder lives in Rust, so a hot reload can remount us mid-capture.
onMounted(async () => {
  if (!isTauri()) return;
  try {
    recording.value = await invoke<boolean>("is_recording");
  } catch {
    recording.value = false;
  }
});

// Never leave the hook installed when the panel goes away.
onBeforeUnmount(() => {
  if (recording.value && isTauri()) {
    void invoke("stop_recording").catch(() => undefined);
  }
});
</script>

<template>
  <div class="seq">
    <div class="row">
      <div class="row__main">
        <span class="row__label">{{ t("sequence.enabled") }}</span>
        <span class="row__desc">{{ t("sequence.enabledHint") }}</span>
      </div>
      <div class="row__control">
        <Toggle v-model="settings.sequenceEnabled" :disabled="props.locked" />
      </div>
    </div>

    <div class="row">
      <div class="row__main">
        <span class="row__label">{{ t("sequence.loop") }}</span>
        <span class="row__desc">{{ t("sequence.loopHint") }}</span>
      </div>
      <div class="row__control">
        <Toggle v-model="settings.sequenceLoop" :disabled="props.locked" />
      </div>
    </div>

    <div class="toolbar">
      <Button
        size="sm"
        :variant="recording ? 'danger' : 'secondary'"
        :loading="busy"
        :disabled="props.locked"
        @click="toggleRecording"
      >
        <template #icon>
          <Circle v-if="recording" :size="12" :stroke-width="1.5" />
          <CircleDot v-else :size="12" :stroke-width="1.5" />
        </template>
        {{ recording ? t("sequence.stopRecording") : t("sequence.startRecording") }}
      </Button>
      <Button
        size="sm"
        variant="ghost"
        :disabled="props.locked || recording || steps.length === 0"
        @click="clearSteps"
      >
        {{ t("sequence.clear") }}
      </Button>
      <span class="toolbar__count">{{ t("sequence.count", { count: steps.length }) }}</span>
    </div>

    <p v-if="recording" class="hint">
      <span class="hint__dot" />
      {{ t("sequence.recording") }}
    </p>

    <ol v-if="steps.length" class="list">
      <li v-for="(step, index) in steps" :key="step.id" class="item">
        <span class="item__index">{{ index + 1 }}</span>
        <span class="item__coords">{{ step.x }}, {{ step.y }}</span>
        <span class="item__meta">{{ buttonLabels[step.button] }}</span>
        <span class="item__meta item__meta--num">{{ step.delayMs }} ms</span>
        <Toggle
          :model-value="step.enabled"
          :disabled="props.locked"
          :aria-label="t('sequence.enabled')"
          @update:model-value="(value: boolean) => (step.enabled = value)"
        />
        <button
          type="button"
          class="item__remove"
          :disabled="props.locked"
          :aria-label="t('sequence.remove')"
          @click="removeStep(index)"
        >
          <X :size="12" :stroke-width="1.5" />
        </button>
      </li>
    </ol>
    <p v-else class="empty">{{ t("sequence.empty") }}</p>
  </div>
</template>

<style scoped>
.seq {
  display: flex;
  flex-direction: column;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-4) 0 var(--sp-3);
}

.toolbar__count {
  margin-left: auto;
  color: var(--text-faint);
  font-size: var(--fs-11);
}

.hint {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  margin-bottom: var(--sp-3);
  color: var(--text-muted);
  font-size: var(--fs-11);
}

.hint__dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--danger);
  animation: pulse 1.2s ease-in-out infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }

  50% {
    opacity: 0.25;
  }
}

.list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.item {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: 4px var(--sp-3);
  border-radius: var(--r-control);
}

.item:hover {
  background: var(--surface-2);
}

.item__index {
  width: 18px;
  color: var(--text-faint);
  font-size: var(--fs-11);
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.item__coords {
  flex: 1;
  font-family: var(--font-mono);
  font-size: var(--fs-11);
  font-variant-numeric: tabular-nums;
}

.item__meta {
  color: var(--text-muted);
  font-size: var(--fs-11);
}

.item__meta--num {
  min-width: 56px;
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.item__remove {
  display: grid;
  place-items: center;
  width: 20px;
  height: 20px;
  border: 0;
  border-radius: var(--r-control);
  background: none;
  color: var(--text-faint);
  cursor: default;
}

.item__remove:hover:not(:disabled) {
  background: var(--surface-3);
  color: var(--danger);
}

.item__remove:disabled {
  opacity: 0.35;
}

.empty {
  padding: var(--sp-4) 0;
  color: var(--text-faint);
  font-size: var(--fs-11);
}
</style>
