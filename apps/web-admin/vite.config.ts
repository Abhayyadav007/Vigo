import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  // Read VITE_* from the repo-root .env shared with the backend.
  envDir: "../..",
  server: { port: 5173 },
});
