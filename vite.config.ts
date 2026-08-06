import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  build: {
    // Every route in App.tsx (besides the default Dashboard) is
    // React.lazy()-loaded, so each already gets its own chunk, fetched only
    // when that section is opened - the one exception is the three.js-based
    // GR2 viewer (Gr2Canvas, shared by the Model Viewer and the Shop
    // Editor's NPC preview): three.js's renderer/scene-graph/math core is
    // legitimately ~530kB minified on its own, no further splitting reduces
    // that. Raise the warning limit past it rather than chase an
    // unreachable number for a dependency that's already isolated into its
    // own lazy chunk.
    chunkSizeWarningLimit: 600,
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
