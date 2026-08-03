// Flat config at the cards' Band-3 baseline: typescript-eslint
// recommended over src/. The conform gate owns the discipline-specific
// structural rules; eslint owns the generic lint layer beneath them —
// and, through the local @org.vibevm/eslint-plugin-ai-native plugin,
// the third REQ-citing diagnostics channel (Scaffold F): the
// diagnostic-cites-req rule, whose messages reproduce the engine's
// Class-F grammar so a project-raised diagnostic cites the violated
// spec:// REQ and a one-line fix surface, never bare free text.
import tseslint from "typescript-eslint";
import aiNative from "@org.vibevm/eslint-plugin-ai-native";

export default tseslint.config(
  {
    ignores: [
      "vibedeps/",
      ".vibe/",
      "node_modules/",
      "target/",
      "eslint.config.js",
    ],
  },
  ...tseslint.configs.recommended,
  {
    plugins: { "ai-native": aiNative },
    rules: {
      "ai-native/diagnostic-cites-req": "error",
    },
  },
);
