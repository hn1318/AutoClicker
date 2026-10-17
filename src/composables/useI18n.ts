import { computed } from "vue";
import { useI18n as useVueI18n } from "vue-i18n";
import type { AppLanguage, SegmentedOption } from "../types";
import { useSettingsStore } from "../stores/settings";

/**
 * Language switching lives in the settings store (persisted + applied once),
 * this composable only exposes the controls a view needs.
 */
export function useLocale() {
  const store = useSettingsStore();
  const { t } = useVueI18n();

  const language = computed<AppLanguage>(() => store.settings.language);

  const languageOptions = computed<SegmentedOption[]>(() => [
    { value: "system", label: t("language.system") },
    { value: "zh-CN", label: t("language.zh") },
    { value: "en-US", label: t("language.en") },
  ]);

  function setLanguage(value: string): void {
    if (value === "system" || value === "zh-CN" || value === "en-US") {
      store.setLanguage(value);
    }
  }

  return { language, languageOptions, setLanguage };
}
