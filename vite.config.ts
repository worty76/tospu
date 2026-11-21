import { defineConfig } from "vitest/config";

// Tauri serves the built files; the dev server port must match tauri.conf.json.
export default defineConfig({
  clearScreen: false,
  server: { port: 1430, strictPort: true },
  build: { target: "safari15", outDir: "dist", emptyOutDir: true },
  test: { environment: "jsdom", globals: true },
});
