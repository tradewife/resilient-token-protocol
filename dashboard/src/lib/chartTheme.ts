/**
 * Shared EvilCharts `ChartConfig` color palette for every chart on the site.
 *
 * These mirror the design tokens in `globals.css` exactly — the EvilCharts
 * components normalize any CSS color (oklch included) through a canvas probe
 * at runtime, so the tokens stay the single source of truth. The site is
 * dark-only, so `dark` carries the real palette and `light` carries a
 * slightly deepened twin (the contract requires at least one theme key;
 * both are provided so light/dark probes never fall back to gray).
 */

/* ── Base token values (keep in sync with globals.css) ── */

export const CHART_TOKENS = {
  emerald: "oklch(55% 0.1 160)",
  emeraldBright: "oklch(66% 0.12 160)",
  emeraldDim: "oklch(35% 0.06 160)",
  coral: "oklch(75% 0.12 30)",
  coralMuted: "oklch(65% 0.08 30)",
  coralDim: "oklch(40% 0.06 30)",
  textSecondary: "oklch(72% 0.03 160)",
  textTertiary: "oklch(55% 0.025 160)",
} as const;

/**
 * Series paint for the primary "good" signal — PnL up, survivors, passes.
 * Two stops give the EvilCharts gradient treatment its emerald sweep.
 * Mutable arrays (not `as const`) — EvilCharts' ChartConfig types demand
 * plain string[].
 */
export const emeraldSeries: { light: string[]; dark: string[] } = {
  light: [CHART_TOKENS.emeraldDim, CHART_TOKENS.emerald],
  dark: [CHART_TOKENS.emerald, CHART_TOKENS.emeraldBright],
};

/** Series paint for the "risk" signal — losses, rejections, warnings. */
export const coralSeries: { light: string[]; dark: string[] } = {
  light: [CHART_TOKENS.coralDim, CHART_TOKENS.coralMuted],
  dark: [CHART_TOKENS.coralMuted, CHART_TOKENS.coral],
};

/** Neutral series paint — comparison/reference data. */
export const neutralSeries: { light: string[]; dark: string[] } = {
  light: [CHART_TOKENS.textTertiary, CHART_TOKENS.textSecondary],
  dark: [CHART_TOKENS.textTertiary, CHART_TOKENS.textSecondary],
};
