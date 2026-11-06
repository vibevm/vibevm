# F18 — Shared automatic-basis policy (foundation)

Accepted local implementation commit: `497f58a48` (10 source/map/feature paths). Root verified committed blobs against the acceptance receipt. The temporary isolated verification worktree was removed and its absence checked. This is not delivery of package defaults/profile review.

The existing app evaluator now consumes one pure core policy for automatic reliance, retained-history carry and review decisions. Ordinary applications keep their existing results and wire records. The shared owner will support the package adapter without copying the app's choice/review logic or moving app state into a global store.

Consumer reliance (profile, feature defaults, choice groups) is distinct from merged-instance closure reliance. Dormant/absent/removed groups keep their relevant history; explicit takeover, actual review and replacement preserve the prior app rules. Parameter-specific reasons, host/path handling and ephemeral secret values remain in the app adapter. The helper grants no operation authority and performs no I/O or persistence.

## Evidence

Eight exact source/test files are bound in the root acceptance receipt. Core policy4 and doctests10 pass. App evaluator15 (including4 differential functions) and existing transfer7 pass. The differential oracle is the frozen old evaluator, not an alternate production path: full serialized records and ephemeral parameter vectors are compared across fresh/reuse/review/replacement, profile overrides, dormant/removal/default changes, parameter origins, secrets and path realization. Root independently confirmed function equality after only test visibility/name/format normalization; the oracle carries no production implements annotations.

Strict combined core/app clippy with lib+tests passed. Scoped conform has0new findings (the existing app ambient-environment observation remains), and exact owned formatting passed. Earlier test-only oracle import failure and compiler-owned dead-code interruption are retained as failures; they were corrected without weakening the old behavior oracle. Isolated HEAD-plus-eight-files specmap/check passed:9086units/5209tagged items/4369edges,0gated orphans/0unresolved host edges,43existing warnings. No unrelated full-workspace runtime claim.

The source contract remains PROP-069#APP-CONFIGURATION-BASIS with direct implementation/test links. No authored vibe.toml, install-g, app index/steps, lock or public JSON schema changes belong to this atom. Its only API addition is the shared Rust policy under the existing core manifest owner.

## Remaining feature work

Ordinary package provisional/default/profile evaluation, retained package basis, versioned lock readers, actual shown-review acknowledgment and successful human/agent retry remain separate active work. F18 does not increase the count of fully transferred old113 tasks or close the broader common-configuration feature.
