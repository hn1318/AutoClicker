import type { ResolvedTheme, ThemeMode } from "../types";

/**
 * Theme is a document-wide concern with exactly one source of truth: the
 * persisted settings. This module only resolves `system` and paints the DOM.
 */

let media: MediaQueryList | null = null;
let releaseTimer: number | undefined;
let currentMode: ThemeMode = "system";

export function systemPrefersDark(): boolean {
  if (typeof window.matchMedia !== "function") return false;
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function resolveTheme(mode: ThemeMode): ResolvedTheme {
  if (mode === "system") return systemPrefersDark() ? "dark" : "light";
  return mode;
}

function paint(theme: ResolvedTheme, animate: boolean): void {
  const root = document.documentElement;
  if (animate) {
    root.classList.add("theme-changing");
    window.clearTimeout(releaseTimer);
    releaseTimer = window.setTimeout(() => root.classList.remove("theme-changing"), 220);
  }
  root.dataset.theme = theme;
}

function ensureMediaListener(): void {
  if (media || typeof window.matchMedia !== "function") return;
  media = window.matchMedia("(prefers-color-scheme: dark)");
  media.addEventListener("change", () => {
    if (currentMode === "system") paint(resolveTheme("system"), true);
  });
}

/** Apply the persisted mode before the first paint (no transition). */
export function initTheme(mode: ThemeMode): void {
  ensureMediaListener();
  currentMode = mode;
  paint(resolveTheme(mode), false);
}

/** Apply a user-initiated mode change with the 180 ms color transition. */
export function applyThemeMode(mode: ThemeMode): void {
  ensureMediaListener();
  currentMode = mode;
  paint(resolveTheme(mode), true);
}
