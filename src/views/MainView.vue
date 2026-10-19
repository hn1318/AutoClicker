<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { Crosshair, Lock, Monitor, Moon, RefreshCw, Sun } from "@lucide/vue";
import Section from "../components/Section.vue";
import NumberInput from "../components/NumberInput.vue";
import Segmented from "../components/Segmented.vue";
import Toggle from "../components/Toggle.vue";
import TextInput from "../components/TextInput.vue";
import HotkeyInput from "../components/HotkeyInput.vue";
import StatusBar from "../components/StatusBar.vue";
import { useSettingsStore } from "../stores/settings";
import { useRuntimeStore } from "../stores/runtime";
import { useToast } from "../composables/useToast";
import { useErrors } from "../composables/useErrors";
import type { CursorInfo, ForegroundInfo, SegmentedOption } from "../types";

const { t } = useI18n();
const settingsStore = useSettingsStore();
const runtime = useRuntimeStore();
const toast = useToast();
const errors = useErrors();

const settings = computed(() => settingsStore.settings);
const locked = computed(() => runtime.phase !== "idle");
const fixedMode = computed(() => settings.value.positionMode === "fixed");

/* ---------------- theme ---------------- */
const themeOptions = computed<SegmentedOption[]>(() => [
  { value: "system", icon: Monitor, ariaLabel: t("theme.system") },
  { value: "light", icon: Sun, ariaLabel: t("theme.light") },
  { value: "dark", icon: Moon, ariaLabel: t("theme.dark") },
]);

function onTheme(value: string): void {
  if (value === "system" || value === "light" || value === "dark") {
    settingsStore.setTheme(value);
  }
}

/* ---------------- interval / frequency ---------------- */
const intervalMs = computed({
  get: () => settings.value.intervalMs,
  set: (value: number) => {
    settings.value.intervalMs = Math.max(1, Math.round(value));
  },
});

const frequencyHz = computed({
  get: () => Math.round((1000 / Math.max(1, settings.value.intervalMs)) * 10) / 10,
  set: (value: number) => {
    if (value > 0) settings.value.intervalMs = Math.max(1, Math.round(1000 / value));
  },
});

const holdMs = computed({
  get: () => settings.value.holdMs,
  set: (value: number) => {
    settings.value.holdMs = Math.max(1, Math.round(value));
  },
});

const clickLimit = computed({
  get: () => settings.value.clickLimit,
  set: (value: number) => {
    settings.value.clickLimit = Math.max(1, Math.round(value));
  },
});

/* ---------------- segmented options ---------------- */
const clickTypeOptions = computed<SegmentedOption[]>(() => [
  { value: "single", label: t("clicker.clickType.single") },
  { value: "double", label: t("clicker.clickType.double") },
  { value: "hold", label: t("clicker.clickType.hold") },
]);

const buttonOptions = computed<SegmentedOption[]>(() => [
  { value: "left", label: t("mouseButton.left") },
  { value: "middle", label: t("mouseButton.middle") },
  { value: "right", label: t("mouseButton.right") },
  { value: "x1", label: t("mouseButton.x1") },
  { value: "x2", label: t("mouseButton.x2") },
]);

const trackOptions = computed<SegmentedOption[]>(() => [
  { value: "always", label: t("cursorTrack.always") },
  { value: "running", label: t("cursorTrack.running") },
  { value: "off", label: t("cursorTrack.off") },
]);

const curveOptions = computed<SegmentedOption[]>(() => [
  { value: "constant", label: t("curve.constant") },
  { value: "linear", label: t("curve.linear") },
  { value: "random", label: t("curve.random") },
]);

function onClickType(value: string): void {
  if (value === "single" || value === "double" || value === "hold") {
    settings.value.clickType = value;
  }
}

function onButton(value: string): void {
  if (value === "left" || value === "middle" || value === "right" || value === "x1" || value === "x2") {
    settings.value.button = value;
  }
}

function onTrackMode(value: string): void {
  if (value === "always" || value === "running" || value === "off") {
    settings.value.cursorTrackMode = value;
  }
}

function onCurve(value: string): void {
  if (value !== "constant" && value !== "linear" && value !== "random") return;
  settings.value.curve = value;
  // Keep the ramp meaningful when switching modes.
  if (value === "linear" && settings.value.curveEndMs >= settings.value.intervalMs) {
    settings.value.curveEndMs = Math.max(1, Math.round(settings.value.intervalMs / 4));
  }
  if (value === "random" && settings.value.curveMaxMs <= settings.value.curveMinMs) {
    settings.value.curveMinMs = Math.max(1, Math.round(settings.value.intervalMs / 2));
    settings.value.curveMaxMs = Math.max(
      settings.value.curveMinMs + 10,
      Math.round(settings.value.intervalMs * 2),
    );
  }
}

/* ---------------- toggles ---------------- */
const followCursor = computed({
  get: () => !fixedMode.value,
  set: (value: boolean) => {
    settings.value.positionMode = value ? "cursor" : "fixed";
  },
});

/* ---------------- DPI: store physical, display logical ---------------- */
const scale = ref(1);

async function refreshScale(): Promise<void> {
  if (!isTauri()) return;
  try {
    scale.value = await invoke<number>("get_scale_at", {
      x: settings.value.posX,
      y: settings.value.posY,
    });
  } catch {
    scale.value = 1;
  }
}

const posLogicalX = computed({
  get: () => Math.round(settings.value.posX / scale.value),
  set: (value: number) => {
    settings.value.posX = Math.round(value * scale.value);
  },
});

const posLogicalY = computed({
  get: () => Math.round(settings.value.posY / scale.value),
  set: (value: number) => {
    settings.value.posY = Math.round(value * scale.value);
  },
});

/* ---------------- position picker (3 s) ---------------- */
const picking = ref(0);
let pickTimer: number | undefined;

function cancelPick(): void {
  if (pickTimer !== undefined) window.clearInterval(pickTimer);
  pickTimer = undefined;
  picking.value = 0;
}

function togglePick(): void {
  if (picking.value > 0) {
    cancelPick();
    return;
  }
  picking.value = 3;
  pickTimer = window.setInterval(() => {
    picking.value -= 1;
    if (picking.value <= 0) {
      cancelPick();
      void captureCursor();
    }
  }, 1000);
}

async function captureCursor(): Promise<void> {
  if (!isTauri()) return;
  try {
    const info = await invoke<CursorInfo>("get_cursor_position");
    settings.value.posX = info.physicalX;
    settings.value.posY = info.physicalY;
    settings.value.positionMode = "fixed";
    scale.value = info.scale;
    toast.success(t("position.captured", { x: Math.round(info.logicalX), y: Math.round(info.logicalY) }));
  } catch (error) {
    toast.error(errors.from(error));
  }
}

onMounted(() => void refreshScale());
onBeforeUnmount(cancelPick);

watch(
  () => settings.value.positionMode,
  (mode) => {
    if (mode === "fixed") void refreshScale();
  },
);

/* ---------------- foreground capture (3 s) ---------------- */
const capturing = ref(0);
let captureTimer: number | undefined;

function cancelCapture(): void {
  if (captureTimer !== undefined) window.clearInterval(captureTimer);
  captureTimer = undefined;
  capturing.value = 0;
}

function toggleCapture(): void {
  if (capturing.value > 0) {
    cancelCapture();
    return;
  }
  capturing.value = 3;
  captureTimer = window.setInterval(() => {
    capturing.value -= 1;
    if (capturing.value <= 0) {
      cancelCapture();
      void captureForeground();
    }
  }, 1000);
}

async function captureForeground(): Promise<void> {
  if (!isTauri()) return;
  try {
    const info = await invoke<ForegroundInfo | null>("get_foreground_info");
    if (!info || (info.title === "" && info.process === "")) {
      toast.error(t("bind.captureFailed"));
      return;
    }
    settings.value.windowTitle = info.title;
    settings.value.windowProcess = info.process;
  } catch (error) {
    toast.error(errors.from(error));
  }
}

onBeforeUnmount(cancelCapture);

/* ---------------- run control ---------------- */
async function toggleRun(): Promise<void> {
  try {
    if (runtime.phase === "idle") {
      await runtime.start();
    } else {
      await runtime.stop();
    }
  } catch (error) {
    toast.error(errors.from(error));
  }
}

const pickLabel = computed(() =>
  picking.value > 0 ? t("position.picking", { seconds: picking.value }) : t("position.pick"),
);

const captureLabel = computed(() =>
  capturing.value > 0 ? t("bind.capturing", { seconds: capturing.value }) : t("bind.capture"),
);
</script>

<template>
  <div class="view">
    <main class="app-body">
      <div class="page">
        <header class="hero">
          <div class="hero__top">
            <div class="inline">
              <h1 class="hero__title">{{ t("app.name") }}</h1>
              <span class="tag num">{{ t("app.version") }}</span>
            </div>
            <Segmented
              size="sm"
              :model-value="settings.theme"
              :options="themeOptions"
              :aria-label="t('theme.label')"
              @update:model-value="onTheme"
            />
          </div>
          <p class="hero__sub">
            {{ t("status.hotkeyHint", { toggle: settings.toggleHotkey, abort: settings.abortHotkey }) }}
          </p>
        </header>

        <div v-if="locked" class="locked">
          <Lock :size="12" :stroke-width="1.5" />
          <span>{{ t("status.locked") }}</span>
        </div>

        <Section :title="t('groups.click')">
          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("clicker.interval") }}</span>
              <span class="row__desc">ms ⇄ Hz</span>
            </div>
            <div class="row__control">
              <NumberInput
                v-model="intervalMs"
                :min="1"
                :max="86400000"
                :disabled="locked"
                :unit="t('clicker.intervalUnit')"
                :aria-label="t('clicker.interval')"
              />
              <NumberInput
                v-model="frequencyHz"
                :min="0.1"
                :max="1000"
                :step="0.1"
                :precision="1"
                :disabled="locked"
                :unit="t('clicker.frequencyUnit')"
                :aria-label="t('clicker.frequency')"
              />
            </div>
          </div>

          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("limit.label") }}</span>
            </div>
            <div class="row__control">
              <Segmented
                :model-value="settings.limitEnabled ? 'limited' : 'unlimited'"
                :options="[
                  { value: 'unlimited', label: t('limit.unlimited') },
                  { value: 'limited', label: t('limit.limited') },
                ]"
                :disabled="locked"
                @update:model-value="(value: string) => (settings.limitEnabled = value === 'limited')"
              />
              <NumberInput
                v-if="settings.limitEnabled"
                v-model="clickLimit"
                :min="1"
                :max="100000000"
                :disabled="locked"
                :unit="t('limit.unit')"
                :aria-label="t('limit.label')"
              />
            </div>
          </div>

          <div class="row row--top">
            <div class="row__main">
              <span class="row__label">{{ t("mouseButton.label") }}</span>
            </div>
            <div class="row__control row__control--stack">
              <Segmented
                block
                :model-value="settings.button"
                :options="buttonOptions"
                :disabled="locked"
                :aria-label="t('mouseButton.label')"
                @update:model-value="onButton"
              />
            </div>
          </div>

          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("clicker.clickType.label") }}</span>
            </div>
            <div class="row__control">
              <Segmented
                :model-value="settings.clickType"
                :options="clickTypeOptions"
                :disabled="locked"
                @update:model-value="onClickType"
              />
            </div>
          </div>

          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("clicker.hold") }}</span>
            </div>
            <div class="row__control">
              <NumberInput
                v-model="holdMs"
                :min="1"
                :max="10000"
                :disabled="locked || settings.clickType !== 'hold'"
                :unit="t('clicker.intervalUnit')"
                :aria-label="t('clicker.hold')"
              />
            </div>
          </div>
        </Section>

        <Section :title="t('groups.position')">
          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("options.followCursor") }}</span>
            </div>
            <div class="row__control">
              <Toggle v-model="followCursor" :disabled="locked" :aria-label="t('options.followCursor')" />
            </div>
          </div>

          <div class="row row--top">
            <div class="row__main">
              <span class="row__label">{{ t("clicker.positionX") }} / {{ t("clicker.positionY") }}</span>
              <span class="row__desc">{{ t("position.scale", { scale: scale.toFixed(2) }) }}</span>
            </div>
            <div class="row__control row__control--stack">
              <NumberInput
                v-model="posLogicalX"
                :min="-100000"
                :max="100000"
                unit="px"
                :disabled="locked || !fixedMode"
                :aria-label="t('clicker.positionX')"
              />
              <NumberInput
                v-model="posLogicalY"
                :min="-100000"
                :max="100000"
                unit="px"
                :disabled="locked || !fixedMode"
                :aria-label="t('clicker.positionY')"
              />
            </div>
          </div>

          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("position.pick") }}</span>
              <span class="row__desc">{{ t("position.pickHint") }}</span>
            </div>
            <div class="row__control">
              <button
                type="button"
                class="inline-btn"
                :class="{ 'is-active': picking > 0 }"
                :disabled="locked"
                @click="togglePick"
              >
                <Crosshair :size="13" :stroke-width="1.5" />
                <span>{{ pickLabel }}</span>
              </button>
            </div>
          </div>
          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("cursorTrack.label") }}</span>
              <span class="row__desc">{{ t("cursorTrack.hint") }}</span>
            </div>
            <div class="row__control">
              <Segmented
                :model-value="settings.cursorTrackMode"
                :options="trackOptions"
                :aria-label="t('cursorTrack.label')"
                @update:model-value="onTrackMode"
              />
            </div>
          </div>
        </Section>

        <Section :title="t('groups.advanced')">
          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("curve.label") }}</span>
            </div>
            <div class="row__control">
              <Segmented
                :model-value="settings.curve"
                :options="curveOptions"
                :disabled="locked"
                @update:model-value="onCurve"
              />
            </div>
          </div>

          <div v-if="settings.curve === 'linear'" class="row">
            <div class="row__main">
              <span class="row__label">{{ t("curve.end") }}</span>
            </div>
            <div class="row__control">
              <NumberInput
                v-model="settings.curveEndMs"
                :min="1"
                :max="86400000"
                :disabled="locked"
                :unit="t('clicker.intervalUnit')"
                :aria-label="t('curve.end')"
              />
            </div>
          </div>

          <template v-if="settings.curve === 'random'">
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("curve.min") }}</span>
              </div>
              <div class="row__control">
                <NumberInput
                  v-model="settings.curveMinMs"
                  :min="1"
                  :max="86400000"
                  :disabled="locked"
                  :unit="t('clicker.intervalUnit')"
                  :aria-label="t('curve.min')"
                />
              </div>
            </div>
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("curve.max") }}</span>
              </div>
              <div class="row__control">
                <NumberInput
                  v-model="settings.curveMaxMs"
                  :min="1"
                  :max="86400000"
                  :disabled="locked"
                  :unit="t('clicker.intervalUnit')"
                  :aria-label="t('curve.max')"
                />
              </div>
            </div>
          </template>

          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("jitter.interval") }}</span>
            </div>
            <div class="row__control">
              <NumberInput
                v-model="settings.jitterIntervalMs"
                :min="0"
                :max="10000"
                :disabled="locked"
                :unit="t('clicker.intervalUnit')"
                :aria-label="t('jitter.interval')"
              />
            </div>
          </div>

          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("jitter.position") }}</span>
            </div>
            <div class="row__control">
              <NumberInput
                v-model="settings.jitterPositionPx"
                :min="0"
                :max="1000"
                :disabled="locked"
                unit="px"
                :aria-label="t('jitter.position')"
              />
            </div>
          </div>

          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("countdown.label") }}</span>
            </div>
            <div class="row__control">
              <NumberInput
                v-model="settings.countdownSec"
                :min="0"
                :max="10"
                :disabled="locked"
                :unit="t('countdown.unit')"
                :aria-label="t('countdown.label')"
              />
            </div>
          </div>

          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("schedule.delay") }}</span>
              <span class="row__desc">{{ t("schedule.hint") }}</span>
            </div>
            <div class="row__control">
              <NumberInput
                v-model="settings.scheduleDelaySec"
                :min="0"
                :max="86400"
                :disabled="locked"
                :unit="t('countdown.unit')"
                :aria-label="t('schedule.delay')"
              />
            </div>
          </div>

          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("schedule.stop") }}</span>
              <span class="row__desc">{{ t("schedule.hint") }}</span>
            </div>
            <div class="row__control">
              <NumberInput
                v-model="settings.scheduleStopSec"
                :min="0"
                :max="86400"
                :disabled="locked"
                :unit="t('countdown.unit')"
                :aria-label="t('schedule.stop')"
              />
            </div>
          </div>

          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("keyboard.label") }}</span>
              <span class="row__desc">{{ t("keyboard.hint") }}</span>
            </div>
            <div class="row__control">
              <Toggle
                v-model="settings.keyboardEnabled"
                :disabled="locked"
                :aria-label="t('keyboard.label')"
              />
            </div>
          </div>

          <div v-if="settings.keyboardEnabled" class="row">
            <div class="row__main">
              <span class="row__label">{{ t("keyboard.key") }}</span>
            </div>
            <div class="row__control">
              <HotkeyInput
                :model-value="settings.keyboardKey"
                allow-bare-keys
                :disabled="locked"
                :aria-label="t('keyboard.key')"
                @change="(value: string) => (settings.keyboardKey = value)"
              />
            </div>
          </div>

          <div class="row">
            <div class="row__main">
              <span class="row__label">{{ t("bind.label") }}</span>
              <span class="row__desc">{{ t("bind.hint") }}</span>
            </div>
            <div class="row__control">
              <Toggle
                v-model="settings.windowBindEnabled"
                :disabled="locked"
                :aria-label="t('bind.label')"
              />
            </div>
          </div>

          <template v-if="settings.windowBindEnabled">
            <div class="row row--top">
              <div class="row__main">
                <span class="row__label">{{ t("bind.title") }}</span>
              </div>
              <div class="row__control row__control--wide">
                <TextInput
                  v-model="settings.windowTitle"
                  :placeholder="t('bind.titlePlaceholder')"
                  :aria-label="t('bind.title')"
                />
              </div>
            </div>
            <div class="row row--top">
              <div class="row__main">
                <span class="row__label">{{ t("bind.process") }}</span>
              </div>
              <div class="row__control row__control--wide">
                <TextInput
                  v-model="settings.windowProcess"
                  :placeholder="t('bind.processPlaceholder')"
                  :aria-label="t('bind.process')"
                />
              </div>
            </div>
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("bind.capture") }}</span>
                <span class="row__desc">{{ t("bind.captureHint") }}</span>
              </div>
              <div class="row__control">
                <button
                  type="button"
                  class="inline-btn"
                  :class="{ 'is-active': capturing > 0 }"
                  :disabled="locked"
                  @click="toggleCapture"
                >
                  <RefreshCw :size="13" :stroke-width="1.5" />
                  <span>{{ captureLabel }}</span>
                </button>
              </div>
            </div>
          </template>
        </Section>
      </div>
    </main>

    <StatusBar
      :phase="runtime.phase"
      :clicks="runtime.clicks"
      :elapsed-ms="runtime.elapsedMs"
      :cps="runtime.cps"
      :countdown-remaining-ms="runtime.countdownRemainingMs"
      @toggle="toggleRun"
    />
  </div>
</template>

<style scoped>
.hero {
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
}

.hero__top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-4);
}

.hero__title {
  font-size: var(--fs-20);
  font-weight: var(--fw-semibold);
  line-height: var(--lh-tight);
  letter-spacing: -0.01em;
}

.hero__sub {
  font-size: var(--fs-11);
  color: var(--text-faint);
}

.tag {
  padding: 1px var(--sp-3);
  border: 1px solid var(--border);
  border-radius: var(--r-tag);
  color: var(--text-faint);
  font-size: var(--fs-11);
  line-height: 1.4;
}

.locked {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  margin-top: calc(var(--sp-4) * -1);
  color: var(--text-faint);
  font-size: var(--fs-11);
}

.row__control--stack {
  flex: 1 1 180px;
  min-width: 0;
}

.row__control--wide {
  flex: 1 1 200px;
  min-width: 0;
}

.inline-btn {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
  height: var(--control-h);
  padding: 0 var(--sp-5);
  border: 1px solid var(--border-strong);
  border-radius: var(--r-btn);
  background: var(--surface);
  color: var(--text);
  font-size: var(--fs-12);
  white-space: nowrap;
  transition:
    background-color var(--dur-2) var(--ease-out),
    border-color var(--dur-2) var(--ease-out),
    color var(--dur-2) var(--ease-out),
    transform var(--dur-1) var(--ease-out);
}

.inline-btn:hover:not(:disabled) {
  background: var(--surface-2);
}

.inline-btn:active:not(:disabled) {
  transform: scale(0.97);
}

.inline-btn:disabled {
  opacity: 0.45;
}

.inline-btn.is-active {
  border-color: var(--accent);
  color: var(--accent);
}

.inline-btn:focus-visible {
  outline: none;
  box-shadow:
    0 0 0 2px var(--bg),
    0 0 0 4px var(--accent);
}
</style>
