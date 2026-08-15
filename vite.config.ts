import path from "node:path";
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";

const currentDirectory = path.dirname(fileURLToPath(import.meta.url));
const host = process.env.TAURI_DEV_HOST;
const syntheticClientCanary = process.env.STT_SYNTHETIC_CLIENT_CANARY;

const syntheticClientCanaryPlugin: Plugin = {
  name: "stt-synthetic-client-canary",
  generateBundle() {
    if (!syntheticClientCanary) return;
    this.emitFile({
      type: "asset",
      fileName: "assets/synthetic-client-canary.js",
      source: `globalThis.__sttValidationCanary="${Buffer.from(syntheticClientCanary).toString("base64")}";`,
    });
  },
};

export default defineConfig({
  plugins: [react(), tailwindcss(), syntheticClientCanaryPlugin],
  clearScreen: false,
  resolve: {
    alias: {
      "@": path.resolve(currentDirectory, "./src"),
    },
  },
  server: {
    host: host || false,
    port: 1420,
    strictPort: true,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
