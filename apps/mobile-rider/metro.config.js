// Learn more: https://docs.expo.dev/guides/customizing-metro/
const path = require("node:path");
const { getDefaultConfig } = require("expo/metro-config");

// Expo only reads .env from this app's folder. Also load the shared repo-root
// .env so EXPO_PUBLIC_* values (e.g. EXPO_PUBLIC_API_URL) reach the bundle.
// Existing variables (shell or app-local .env) take precedence.
try {
  const { parseEnv } = require("node:util");
  const fs = require("node:fs");
  const vars = parseEnv(fs.readFileSync(path.resolve(__dirname, "../../.env"), "utf8"));
  for (const [key, value] of Object.entries(vars)) {
    if (process.env[key] === undefined) process.env[key] = value;
  }
} catch (err) {
  if (err.code !== "ENOENT") throw err;
}

module.exports = getDefaultConfig(__dirname);
