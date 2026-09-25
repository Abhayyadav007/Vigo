// Shared NativeWind/Tailwind preset: `presets: [require("@vigo/ui/tailwind-preset")]`.
const tokens = require("./src/tokens.json");

const px = (scale) => Object.fromEntries(Object.entries(scale).map(([k, v]) => [k, `${v}px`]));

/** @type {import('tailwindcss').Config} */
module.exports = {
  presets: [require("nativewind/preset")],
  theme: {
    extend: {
      colors: tokens.colors,
      borderRadius: px(tokens.radius),
    },
  },
};
