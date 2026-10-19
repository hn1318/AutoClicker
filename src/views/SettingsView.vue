<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { Download, FolderOpen, Monitor, Moon, RefreshCw, Sun, Upload } from "@lucide/vue";
import Section from "../components/Section.vue";
import Segmented from "../components/Segmented.vue";
import Toggle from "../components/Toggle.vue";
import Button from "../components/Button.vue";
import TextInput from "../components/TextInput.vue";
import HotkeyInput from "../components/HotkeyInput.vue";
import { useSettingsStore } from "../stores/settings";
import { useRuntimeStore } from "../stores/runtime";
import { useLocale } from "../composables/useI18n";
import { useToast } from "../composables/useToast";
import { useErrors } from "../composables/useErrors";
import { compareVersions } from "../utils/format";
import { GITHUB_RELEASES_PAGE, fetchLatestRelease, type UpdateCandidate } from "../utils/update";
import type { AppInfo, SegmentedOption, Settings } from "../types";

const { t } = useI18n();
const settingsStore = useSettingsStore();
const runtime = useRuntimeStore();
const toast = useToast();
const errors = useErrors();
const { language, languageOptions, setLanguage } = useLocale();

const settings = computed(() => settingsStore.settings);
const tab = ref<"general" | "about">("general");
const appInfo = ref<AppInfo | null>(null);
const checking = ref(false);

const tabs = computed<SegmentedOption[]>(() => [
  { value: "general", label: t("nav.general") },
  { value: "about", label: t("nav.about") },
]);

const themeOptions = computed<SegmentedOption[]>(() => [
  { value: "system", icon: Monitor, ariaLabel: t("theme.system") },
  { value: "light", icon: Sun, ariaLabel: t("theme.light") },
  { value: "dark", icon: Moon, ariaLabel: t("theme.dark") },
]);

onMounted(async () => {
  if (!isTauri()) return;
  try {
    appInfo.value = await invoke<AppInfo>("get_app_info");
  } catch (error) {
    console.warn("[autoclicker] app info unavailable", error);
  }
});

function onTheme(value: string): void {
  if (value === "system" || value === "light" || value === "dark") {
    settingsStore.setTheme(value);
  }
}

async function changeHotkey(kind: "toggle" | "abort", value: string): Promise<void> {
  const toggle = kind === "toggle" ? value : settings.value.toggleHotkey;
  const abort = kind === "abort" ? value : settings.value.abortHotkey;
  try {
    await settingsStore.setHotkeys(toggle, abort);
  } catch (error) {
    toast.error(errors.from(error));
  }
}

const pendingRelease = ref<UpdateCandidate | null>(null);

/**
 * Reads the newest GitHub Release by default (the same feed the CI workflow
 * publishes to); a custom manifest URL in settings takes precedence.
 */
async function checkUpdates(): Promise<void> {
  checking.value = true;
  pendingRelease.value = null;
  try {
    const candidate = await fetchLatestRelease(settings.value.updateUrl);
    const current = appInfo.value?.version ?? "0.0.0";
    if (compareVersions(candidate.version, current) > 0) {
      pendingRelease.value = candidate;
      toast.success(t("update.available", { version: candidate.version }));
    } else {
      toast.success(t("update.upToDate"));
    }
  } catch (error) {
    console.error("[autoclicker] update check failed", error);
    toast.error(t("update.failed"));
  } finally {
    checking.value = false;
  }
}

async function openRelease(): Promise<void> {
  const url = pendingRelease.value?.url || GITHUB_RELEASES_PAGE;
  try {
    await invoke("open_url", { url });
  } catch (error) {
    toast.error(errors.from(error));
  }
}

async function openDataDir(): Promise<void> {
  try {
    await invoke("open_data_dir");
  } catch (error) {
    toast.error(errors.from(error));
  }
}

/* ---------------- profile backup ---------------- */
const busy = ref<"" | "export" | "import">("");
// Importing swaps the authoritative settings, which must not happen mid-run.
const running = computed(() => runtime.phase !== "idle");

async function exportProfile(): Promise<void> {
  busy.value = "export";
  try {
    const path = await invoke<string | null>("export_settings", {
      settings: settings.value,
      title: t("backup.exportTitle"),
    });
    if (path) toast.success(t("backup.exported", { path }));
  } catch (error) {
    toast.error(errors.from(error));
  } finally {
    busy.value = "";
  }
}

async function importProfile(): Promise<void> {
  busy.value = "import";
  try {
    const imported = await invoke<Settings | null>("import_settings", {
      title: t("backup.importTitle"),
    });
    if (!imported) return;
    await settingsStore.applyImported(imported);
    toast.success(t("backup.imported"));
  } catch (error) {
    toast.error(errors.from(error));
  } finally {
    busy.value = "";
  }
}

const platform = computed(() => {
  if (!appInfo.value) return "—";
  return `${appInfo.value.os} · ${appInfo.value.arch}`;
});
</script>

<template>
  <div class="view">
    <main class="app-body">
      <div class="page">
        <header class="head">
          <h1 class="head__title">{{ t("settings.title") }}</h1>
          <Segmented v-model="tab" size="sm" :options="tabs" :aria-label="t('settings.title')" />
        </header>

        <template v-if="tab === 'general'">
          <Section :title="t('settings.appearance')">
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("theme.label") }}</span>
              </div>
              <div class="row__control">
                <Segmented
                  :model-value="settings.theme"
                  :options="themeOptions"
                  :aria-label="t('theme.label')"
                  @update:model-value="onTheme"
                />
              </div>
            </div>
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("language.label") }}</span>
              </div>
              <div class="row__control">
                <Segmented
                  :model-value="language"
                  :options="languageOptions"
                  :aria-label="t('language.label')"
                  @update:model-value="setLanguage"
                />
              </div>
            </div>
          </Section>

          <Section :title="t('settings.startup')">
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("settings.autostart") }}</span>
                <span class="row__desc">{{ t("settings.autostartHint") }}</span>
              </div>
              <div class="row__control">
                <Toggle v-model="settings.autostart" :aria-label="t('settings.autostart')" />
              </div>
            </div>
          </Section>

          <Section :title="t('settings.hotkeys')" :description="t('settings.hotkeyHint')">
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("settings.hotkeyToggle") }}</span>
              </div>
              <div class="row__control">
                <HotkeyInput
                  :model-value="settings.toggleHotkey"
                  :aria-label="t('settings.hotkeyToggle')"
                  @change="(value: string) => changeHotkey('toggle', value)"
                />
              </div>
            </div>
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("settings.hotkeyAbort") }}</span>
              </div>
              <div class="row__control">
                <HotkeyInput
                  :model-value="settings.abortHotkey"
                  :aria-label="t('settings.hotkeyAbort')"
                  @change="(value: string) => changeHotkey('abort', value)"
                />
              </div>
            </div>
          </Section>

          <Section :title="t('backup.title')" :description="t('backup.hint')">
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("backup.export") }}</span>
                <span class="row__desc">{{ t("backup.exportHint") }}</span>
              </div>
              <div class="row__control">
                <Button size="sm" :loading="busy === 'export'" @click="exportProfile">
                  <template #icon><Download :size="12" :stroke-width="1.5" /></template>
                  {{ t("backup.export") }}
                </Button>
              </div>
            </div>
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("backup.import") }}</span>
                <span class="row__desc">{{ t("backup.importHint") }}</span>
              </div>
              <div class="row__control">
                <Button
                  size="sm"
                  :loading="busy === 'import'"
                  :disabled="running"
                  @click="importProfile"
                >
                  <template #icon><Upload :size="12" :stroke-width="1.5" /></template>
                  {{ t("backup.import") }}
                </Button>
              </div>
            </div>
          </Section>

          <Section :title="t('settings.updates')">
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("settings.updateVersion") }}</span>
                <span class="row__desc num">
                  {{ appInfo ? `v${appInfo.version}` : t("app.version") }}
                </span>
              </div>
              <div class="row__control">
                <Button size="sm" :loading="checking" @click="checkUpdates">
                  <template #icon><RefreshCw :size="12" :stroke-width="1.5" /></template>
                  {{ checking ? t("common.checking") : t("settings.checkUpdate") }}
                </Button>
              </div>
            </div>
            <div v-if="pendingRelease" class="row">
              <div class="row__main">
                <span class="row__label">
                  {{ t("update.available", { version: pendingRelease.version }) }}
                </span>
              </div>
              <div class="row__control">
                <Button size="sm" @click="openRelease">{{ t("update.openRelease") }}</Button>
              </div>
            </div>
            <div class="row row--top">
              <div class="row__main">
                <span class="row__label">{{ t("settings.updateSource") }}</span>
                <span class="row__desc">{{ t("settings.updateSourceHint") }}</span>
              </div>
              <div class="row__control row__control--wide">
                <TextInput
                  v-model="settings.updateUrl"
                  mono
                  :placeholder="t('settings.updateSourcePlaceholder')"
                  :aria-label="t('settings.updateSource')"
                />
              </div>
            </div>
          </Section>
        </template>

        <template v-else>
          <Section :title="t('nav.about')">
            <div class="about">
              <div class="about__mark">AC</div>
              <div class="about__text">
                <span class="about__name">{{ t("app.name") }}</span>
                <span class="about__tagline">{{ t("about.tagline") }}</span>
              </div>
            </div>
          </Section>

          <Section :title="t('about.info')">
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("about.version") }}</span>
              </div>
              <div class="row__control num text-muted">
                {{ appInfo ? `v${appInfo.version}` : t("app.version") }}
              </div>
            </div>
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("about.tauri") }}</span>
              </div>
              <div class="row__control num text-muted">
                {{ appInfo ? `v${appInfo.tauriVersion}` : "—" }}
              </div>
            </div>
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("about.platform") }}</span>
              </div>
              <div class="row__control text-muted">{{ platform }}</div>
            </div>
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("settings.dataDir") }}</span>
                <span class="row__desc mono">{{ appInfo?.dataDir || "—" }}</span>
              </div>
              <div class="row__control">
                <Button size="sm" @click="openDataDir">
                  <template #icon><FolderOpen :size="12" :stroke-width="1.5" /></template>
                  {{ t("settings.openDataDir") }}
                </Button>
              </div>
            </div>
          </Section>

          <Section :title="t('about.tech')">
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("about.tech") }}</span>
              </div>
              <div class="row__control text-muted">{{ t("about.techStack") }}</div>
            </div>
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("about.input") }}</span>
              </div>
              <div class="row__control text-muted">{{ t("about.inputValue") }}</div>
            </div>
            <div class="row">
              <div class="row__main">
                <span class="row__label">{{ t("about.license") }}</span>
              </div>
              <div class="row__control text-muted">{{ t("about.licenseValue") }}</div>
            </div>
          </Section>

        </template>
      </div>
    </main>
  </div>
</template>

<style scoped>
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-4);
}

.head__title {
  font-size: var(--fs-20);
  font-weight: var(--fw-semibold);
  line-height: var(--lh-tight);
  letter-spacing: -0.01em;
}

.row__control--wide {
  flex: 1 1 200px;
  min-width: 0;
}

.about {
  display: flex;
  align-items: center;
  gap: var(--sp-6);
  padding: var(--sp-7) var(--sp-6);
}

.about__mark {
  display: grid;
  flex: none;
  place-items: center;
  width: 40px;
  height: 40px;
  border: 1px solid var(--border-strong);
  border-radius: var(--r-card);
  background: var(--surface-2);
  color: var(--text);
  font-size: var(--fs-13);
  font-weight: var(--fw-semibold);
  letter-spacing: 0.02em;
}

.about__text {
  display: flex;
  flex-direction: column;
  gap: var(--sp-1);
  min-width: 0;
}

.about__name {
  font-size: var(--fs-14);
  font-weight: var(--fw-medium);
}

.about__tagline {
  font-size: var(--fs-12);
  color: var(--text-muted);
}

.mono {
  font-family: var(--font-mono);
  font-size: var(--fs-11);
  overflow-wrap: anywhere;
}
</style>
