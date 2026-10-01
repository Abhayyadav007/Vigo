// Light variant of the shared preset (customer app):
// `presets: [require("@vigo/ui/tailwind-preset-light")]`.
const tokens = require("./src/tokens.json");
const base = require("./tailwind-preset");

/** @type {import('tailwindcss').Config} */
module.exports = {
  presets: [base],
  theme: { extend: { colors: tokens.lightColors } },
};
