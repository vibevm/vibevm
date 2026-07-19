import { defineConfig } from "vitest/config";
import solid from "vite-plugin-solid";

// Engine cells are render-free TS (node environment). Chrome component tests,
// if added later, switch their suite to environment: "jsdom".
export default defineConfig({
  plugins: [solid()],
  test: {
    include: ["src/**/*.test.ts", "test/**/*.test.ts"],
    environment: "node",
  },
  resolve: {
    alias: {
      "@vibeterm/engine": "/src/engine/index.ts",
    },
  },
});
