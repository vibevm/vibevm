import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import tailwindcss from "@tailwindcss/vite";

// The Solid chrome bundle Electron loads as the shell window.
// The engine is NOT bundled here — it is compiled by tsc (tsconfig.engine.json)
// into dist/engine and imported by main.cjs (the render-free core lives in the
// main process; the chrome is a one-way projection — PROP-047 §3).
export default defineConfig({
  plugins: [solid(), tailwindcss()],
  root: ".",
  base: "./",
  build: {
    outDir: "dist/chrome",
    emptyOutDir: true,
    target: "esnext",
  },
  resolve: {
    alias: {
      "@vibeterm/engine": "/src/engine/index.ts",
    },
  },
});
