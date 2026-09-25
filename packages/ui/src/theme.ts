// Design tokens shared by every client. Phase 2 feeds these into the
// NativeWind/Tailwind config of each app.
// TODO(phase-2): shared RN components (Button, Screen, TextField) + NativeWind preset.

export const colors = {
  brand: "#0C8346",
  brandDark: "#07542D",
  accent: "#F5B301",
  background: "#FFFFFF",
  surface: "#F4F6F5",
  text: "#111827",
  textMuted: "#6B7280",
  border: "#E5E7EB",
  success: "#16A34A",
  warning: "#D97706",
  danger: "#DC2626",
} as const;

export const spacing = { xs: 4, sm: 8, md: 12, lg: 16, xl: 24, xxl: 32 } as const;
export const radius = { sm: 6, md: 10, lg: 16, pill: 999 } as const;
export const fontSize = { xs: 12, sm: 14, md: 16, lg: 20, xl: 24, xxl: 32 } as const;

export type ColorToken = keyof typeof colors;
