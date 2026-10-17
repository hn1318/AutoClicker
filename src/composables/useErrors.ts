import { useI18n } from "vue-i18n";
import type { AppErrorPayload } from "../types";

export function isAppError(value: unknown): value is AppErrorPayload {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as { code?: unknown };
  return typeof candidate.code === "string";
}

/**
 * Turn a structured Rust error into a localized sentence. Unknown codes fall
 * back to a generic message instead of leaking internal English text.
 */
export function useErrors() {
  const { t, te } = useI18n();

  function from(error: unknown): string {
    if (!isAppError(error)) return t("errors.unknown");

    const key = `errors.${error.code}`;
    if (!te(key)) return t("errors.unknown");

    const params: Record<string, unknown> = { ...error.params };
    if (typeof params.field === "string") {
      const fieldKey = `errors.field.${params.field}`;
      params.field = te(fieldKey) ? t(fieldKey) : params.field;
    }
    return t(key, params);
  }

  return { from };
}
