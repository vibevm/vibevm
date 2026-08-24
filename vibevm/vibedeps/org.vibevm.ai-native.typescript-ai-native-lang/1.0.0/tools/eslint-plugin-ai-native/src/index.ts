/**
 * index.ts — the public surface of the AI-Native TypeScript eslint
 * plugin. Exposes the one rule the Scaffold F card names
 * (`diagnostic-cites-req`) for flat-config consumers:
 *
 *     import aiNative from "@org.vibevm/eslint-plugin-ai-native";
 *     // …
 *     { plugins: { "ai-native": aiNative },
 *       rules:   { "ai-native/diagnostic-cites-req": "error" } }
 *
 * The rule resolves `@typescript-eslint/utils` from the consumer's own
 * typescript-eslint install at runtime (the ts-extract precedent); node
 * >= 22.6 type-strips this source on import, so no build step ships.
 */

import { diagnosticCitesReq } from "./diagnostic-cites-req.ts";

import type { ESLint } from "eslint";

const plugin = {
  meta: { name: "@org.vibevm/eslint-plugin-ai-native", version: "0.1.0" },
  rules: {
    "diagnostic-cites-req": diagnosticCitesReq,
  },
} satisfies ESLint.Plugin;

export default plugin;
