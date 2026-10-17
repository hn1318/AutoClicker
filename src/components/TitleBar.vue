<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { Minus, MousePointerClick, Settings, X } from "@lucide/vue";

interface Props {
  title?: string;
  showSettings?: boolean;
  settingsActive?: boolean;
}

withDefaults(defineProps<Props>(), {
  showSettings: false,
  settingsActive: false,
});

const emit = defineEmits<{
  minimize: [];
  close: [];
  "open-settings": [];
}>();

const { t } = useI18n();
</script>

<template>
  <header class="titlebar" data-tauri-drag-region>
    <div class="titlebar__brand" data-tauri-drag-region>
      <MousePointerClick class="titlebar__mark" :size="14" :stroke-width="1.5" />
      <span class="titlebar__name" data-tauri-drag-region>{{ title ?? t("app.name") }}</span>
    </div>

    <div class="titlebar__actions">
      <button
        v-if="showSettings"
        type="button"
        class="winbtn"
        :class="{ 'is-active': settingsActive }"
        :aria-label="t('titlebar.settings')"
        :title="t('titlebar.settings')"
        @click="emit('open-settings')"
      >
        <Settings :size="14" :stroke-width="1.5" />
      </button>
      <button
        type="button"
        class="winbtn"
        :aria-label="t('titlebar.minimize')"
        :title="t('titlebar.minimize')"
        @click="emit('minimize')"
      >
        <Minus :size="14" :stroke-width="1.5" />
      </button>
      <button
        type="button"
        class="winbtn winbtn--close"
        :aria-label="t('titlebar.close')"
        :title="t('titlebar.close')"
        @click="emit('close')"
      >
        <X :size="14" :stroke-width="1.5" />
      </button>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: space-between;
  height: var(--titlebar-h);
  padding: 0 var(--sp-2) 0 var(--sp-6);
  border-bottom: 1px solid var(--border);
  background: var(--surface);
  user-select: none;
  -webkit-user-select: none;
}

.titlebar__brand {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  pointer-events: none;
}

.titlebar__mark {
  color: var(--accent);
}

.titlebar__name {
  font-size: var(--fs-12);
  font-weight: var(--fw-medium);
  letter-spacing: 0.01em;
  color: var(--text);
}

.titlebar__actions {
  display: flex;
  align-items: center;
  gap: var(--sp-1);
}

.winbtn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 26px;
  border-radius: var(--r-tag);
  color: var(--text-muted);
  transition:
    background-color var(--dur-2) var(--ease-out),
    color var(--dur-2) var(--ease-out),
    transform var(--dur-1) var(--ease-out);
}

.winbtn:hover {
  background: var(--surface-2);
  color: var(--text);
}

.winbtn:active {
  transform: scale(0.97);
}

.winbtn.is-active {
  color: var(--accent);
}

.winbtn:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}

.winbtn--close:hover {
  background: var(--danger);
  color: var(--on-danger);
}
</style>
