import type { Component } from "vue";

/* ---------------- UI primitives ---------------- */

export type ThemeMode = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";
export type AppLanguage = "system" | "zh-CN" | "en-US";

export type ButtonVariant = "primary" | "danger" | "secondary" | "ghost";
export type ControlSize = "sm" | "md";

export interface SegmentedOption {
  value: string;
  label?: string;
  /** Optional lucide component rendered before the label. */
  icon?: Component;
  /** Accessible name, required for icon-only options. */
  ariaLabel?: string;
  disabled?: boolean;
}

/* ---------------- domain enums (mirror Rust) ---------------- */

export type MouseButton = "left" | "middle" | "right" | "x1" | "x2";
export type ClickType = "single" | "double" | "hold";
export type PositionMode = "cursor" | "fixed";
export type IntervalCurve = "constant" | "linear" | "random";
export type CursorTrackMode = "always" | "running" | "off";

export interface SequenceStep {
  id: string;
  x: number;
  y: number;
  button: MouseButton;
  clickType: ClickType;
  holdMs: number;
  delayMs: number;
  enabled: boolean;
}

/**
 * Mirrors `state::Settings` on the Rust side (camelCase).
 * Coordinates are PHYSICAL pixels; the UI converts for display.
 */
export interface Settings {
  intervalMs: number;
  limitEnabled: boolean;
  clickLimit: number;
  button: MouseButton;
  clickType: ClickType;
  holdMs: number;
  positionMode: PositionMode;
  posX: number;
  posY: number;
  cursorTrackMode: CursorTrackMode;
  countdownSec: number;
  jitterIntervalMs: number;
  jitterPositionPx: number;
  curve: IntervalCurve;
  curveEndMs: number;
  curveMinMs: number;
  curveMaxMs: number;
  sequenceEnabled: boolean;
  sequenceLoop: boolean;
  sequence: SequenceStep[];
  scheduleDelaySec: number;
  scheduleStopSec: number;
  keyboardEnabled: boolean;
  keyboardKey: string;
  windowBindEnabled: boolean;
  windowTitle: string;
  windowProcess: string;
  toggleHotkey: string;
  abortHotkey: string;
  theme: ThemeMode;
  language: AppLanguage;
  autostart: boolean;
  closeToTray: boolean;
  notifyFinish: boolean;
  updateUrl: string;
}

/* ---------------- runtime events ---------------- */

export type RunPhase = "idle" | "countdown" | "running";

export interface TickPayload {
  clicks: number;
  elapsedMs: number;
  cps: number;
}

export interface CountdownPayload {
  remainingMs: number;
  totalMs: number;
}

export interface RuntimeEventPayload {
  event: string;
  params: Record<string, unknown>;
}

/** Structured error returned by every Rust command. */
export interface AppErrorPayload {
  code: string;
  params: Record<string, unknown>;
}

export interface CursorInfo {
  physicalX: number;
  physicalY: number;
  logicalX: number;
  logicalY: number;
  scale: number;
}

/** Streamed ~50 Hz by the Rust cursor sampler while tracking is active. */
export type CursorPositionPayload = CursorInfo;

export interface ForegroundInfo {
  title: string;
  process: string;
}

export interface AppInfo {
  name: string;
  version: string;
  tauriVersion: string;
  os: string;
  arch: string;
  dataDir: string;
}
