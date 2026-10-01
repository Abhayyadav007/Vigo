// Design tokens shared by every client. Values live in tokens.json so the
// Tailwind preset (tailwind-preset.js) and TS code read the same source.
// The palette is dark-only: text on vivid fills (brand, danger, muted) uses
// `text-background`, not white, to keep WCAG AA contrast.
import tokens from "./tokens.json";

export const colors = tokens.colors;
export const spacing = tokens.spacing;
export const radius = tokens.radius;
export const fontSize = tokens.fontSize;

export type ColorToken = keyof typeof colors;

/** Spread into React Navigation's `DarkTheme.colors` so headers and tab bars match. */
export const navColors = {
  primary: colors.brand,
  background: colors.background,
  card: colors.background,
  text: colors.ink,
  border: colors.line,
  notification: colors.danger,
};
