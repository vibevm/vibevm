// Flat config at the cards' Band-3 baseline: typescript-eslint
// recommended over the two source roots. The conform gate owns the
// discipline-specific structural rules; eslint owns the generic lint
// layer beneath them.
//
// The discipline's own plugin (@org.vibevm/eslint-plugin-ai-native, the
// third REQ-citing diagnostics channel) is NOT mounted here. It is
// reachable only as a `file:` dependency on a path relative to this
// repository's layout, and this package is published as source: a
// dependency that resolves only inside one checkout would break the
// moment the package is built anywhere else. The rule it carries —
// diagnostic-cites-req — guards diagnostics that cite a violated
// spec:// REQ, and a site shell raises none: the citations on these
// pages arrive already resolved, inside the island the Rust pipeline
// renders.
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: [
      "node_modules/",
      "dist/",
      "server/",
      "tmp/",
      "discipline/",
      "eslint.config.js",
      "tools/",
      "design/audit/",
      "design/fonts/",
      "site/src/fixtures/",
      "site/src/generated/",
    ],
  },
  ...tseslint.configs.recommended,
);
