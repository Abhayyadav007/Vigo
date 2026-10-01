import { lightColors, navColorsFor } from "@vigo/ui";

// The customer app uses the light palette; tailwind.config.js uses the matching
// light preset, so `bg-brand` and `colors.brand` agree.
export const colors = lightColors;
export const navColors = navColorsFor(lightColors);
