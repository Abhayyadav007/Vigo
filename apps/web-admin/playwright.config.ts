import { defineConfig, devices } from "@playwright/test";

// Needs the Firebase Auth emulator (`pnpm emulators`) and a backend running
// with FIREBASE_AUTH_EMULATOR_HOST set. Playwright starts Vite itself.
export default defineConfig({
  testDir: "./e2e",
  timeout: 60_000,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["github"], ["html", { open: "never" }]] : "list",
  use: {
    baseURL: "http://localhost:5173",
    trace: "retain-on-failure",
    ...devices["Desktop Chrome"],
  },
  webServer: {
    command: "pnpm dev --strictPort",
    url: "http://localhost:5173",
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
