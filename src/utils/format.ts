/** mm:ss for the status bar; tabular numerals are applied by CSS. */
export function formatDuration(ms: number): string {
  const totalSeconds = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

export function formatCps(cps: number): string {
  if (!Number.isFinite(cps) || cps <= 0) return "0.0";
  return cps.toFixed(1);
}

/** Numeric compare for dotted versions; non-numeric suffixes are ignored. */
export function compareVersions(a: string, b: string): number {
  const normalize = (value: string): number[] =>
    value
      .replace(/^v/i, "")
      .split(/[.-]/)
      .map((part) => Number.parseInt(part, 10))
      .map((part) => (Number.isFinite(part) ? part : 0));

  const left = normalize(a);
  const right = normalize(b);
  const length = Math.max(left.length, right.length);

  for (let index = 0; index < length; index += 1) {
    const l = left[index] ?? 0;
    const r = right[index] ?? 0;
    if (l !== r) return l > r ? 1 : -1;
  }
  return 0;
}
