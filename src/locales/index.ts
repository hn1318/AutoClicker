import { createI18n } from "vue-i18n";
import zhCN from "./zh-CN";
import enUS from "./en-US";
import type { AppLanguage } from "../types";

export type ResolvedLocale = "zh-CN" | "en-US";

export const i18n = createI18n({
  legacy: false,
  locale: "zh-CN",
  fallbackLocale: "en-US",
  messages: {
    "zh-CN": zhCN,
    "en-US": enUS,
  },
});

export function resolveLocale(language: AppLanguage): ResolvedLocale {
  if (language === "system") {
    return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en-US";
  }
  return language;
}

/** Switch UI language; every string (including toasts and validation) follows. */
export function applyLocale(language: AppLanguage): void {
  const locale = resolveLocale(language);
  i18n.global.locale.value = locale;
  document.documentElement.lang = locale;
}
