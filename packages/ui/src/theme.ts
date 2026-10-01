// Design tokens shared by every client. Values live in tokens.json so the
// Tailwind presets (tailwind-preset.js, tailwind-preset-light.js) and TS code
// read the same source.
// `colors` is the dark palette (picker, rider, admin). `lightColors` is the
// customer app's light palette: same token names, so shared components work in
// both. Text on vivid fills (brand, danger) uses `text-background` and keeps
// WCAG AA contrast in either palette.
import tokens from "./tokens.json";

export const colors = tokens.colors;
export const lightColors: Palette = tokens.lightColors;
export const spacing = tokens.spacing;
export const radius = tokens.radius;
export const fontSize = tokens.fontSize;

export type Palette = typeof colors;
export type ColorToken = keyof Palette;

/** Spread into React Navigation's theme colours so headers and tab bars match. */
export const navColorsFor = (c: Palette) => ({
  primary: c.brand,
  background: c.background,
  card: c.surface,
  text: c.ink,
  border: c.line,
  notification: c.danger,
});

/** Dark navigation colours (picker, rider). */
export const navColors = { ...navColorsFor(colors), card: colors.background };
