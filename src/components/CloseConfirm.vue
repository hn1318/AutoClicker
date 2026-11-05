<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import Button from "./Button.vue";

const props = defineProps<{ open: boolean }>();

const emit = defineEmits<{
  choose: [action: "tray" | "quit", remember: boolean];
  cancel: [];
}>();

const { t } = useI18n();
const remember = ref(false);

// Every close request starts from a clean checkbox.
watch(
  () => props.open,
  (open) => {
    if (open) remember.value = false;
  },
);

function choose(action: "tray" | "quit"): void {
  emit("choose", action, remember.value);
}
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="scrim" @mousedown.self="emit('cancel')">
      <div class="dialog" role="dialog" aria-modal="true" :aria-label="t('closeDialog.title')">
        <h2 class="dialog__title">{{ t("closeDialog.title") }}</h2>
        <p class="dialog__body">{{ t("closeDialog.body") }}</p>

        <label class="dialog__remember">
          <input v-model="remember" type="checkbox" />
          <span>{{ t("closeDialog.remember") }}</span>
        </label>

        <div class="dialog__actions">
          <Button size="sm" variant="ghost" @click="emit('cancel')">
            {{ t("closeDialog.cancel") }}
          </Button>
          <Button size="sm" @click="choose('tray')">{{ t("closeDialog.tray") }}</Button>
          <Button size="sm" variant="danger" @click="choose('quit')">
            {{ t("closeDialog.quit") }}
          </Button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.scrim {
  position: fixed;
  z-index: 2500;
  display: grid;
  place-items: center;
  inset: 0;
  padding: var(--sp-7);
  background: var(--overlay);
  animation: fade-in var(--dur-2) var(--ease-out);
}

.dialog {
  display: flex;
  flex-direction: column;
  gap: var(--sp-5);
  width: 100%;
  max-width: 320px;
  padding: var(--sp-7) var(--sp-7) var(--sp-6);
  border: 1px solid var(--border);
  border-radius: var(--r-card);
  background: var(--surface);
  box-shadow: 0 24px 48px rgba(0, 0, 0, 0.28);
}

.dialog__title {
  font-size: var(--fs-14);
  font-weight: var(--fw-semibold);
  line-height: var(--lh-tight);
}

.dialog__body {
  color: var(--text-muted);
  font-size: var(--fs-12);
  line-height: var(--lh-normal);
}

.dialog__remember {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  color: var(--text-muted);
  font-size: var(--fs-11);
  cursor: pointer;
  user-select: none;
}

.dialog__remember input {
  width: 13px;
  height: 13px;
  accent-color: var(--accent);
}

.dialog__actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-3);
  margin-top: var(--sp-2);
}

@keyframes fade-in {
  from {
    opacity: 0;
  }

  to {
    opacity: 1;
  }
}
</style>
