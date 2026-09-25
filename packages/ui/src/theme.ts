// Design tokens shared by every client. Values live in tokens.json so the
// Tailwind preset (tailwind-preset.js) and TS code read the same source.
import tokens from "./tokens.json";

export const colors = tokens.colors;
export const spacing = tokens.spacing;
export const radius = tokens.radius;
export const fontSize = tokens.fontSize;

export type ColorToken = keyof typeof colors;
