/**
 * Non-component chart helpers — colours, axis styling and number formatting.
 *
 * Split out of `chart-kit.tsx` so that file exports components only, which is
 * what the `react/only-export-components` lint rule (and Fast Refresh) wants.
 */

/** Blue-only ramp, light -> dark. Segments/series index into this. */
export const BLUE_RAMP = ['#60a5fa', '#3b82f6', '#2563eb', '#1d4ed8', '#93c5fd', '#1e40af', '#bfdbfe'] as const;

export function blueAt(index: number): string {
  return BLUE_RAMP[index % BLUE_RAMP.length];
}

/** Shared axis styling — thin, low-contrast, no gridlines. */
export const AXIS_PROPS = {
  stroke: 'transparent',
  tickLine: false,
  axisLine: false,
  tick: { fill: '#a1a1aa', fontSize: 11, fontWeight: 300 },
} as const;

export function compactNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}
