// Flat config at the cards' Band-3 baseline: typescript-eslint
// recommended over src/. The conform gate owns the discipline-specific
// structural rules; eslint owns the generic lint layer beneath them.
import tseslint from "typescript-eslint";

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
);
