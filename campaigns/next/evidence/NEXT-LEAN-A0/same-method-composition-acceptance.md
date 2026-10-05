# A0.3 accepted: one same-method Rust + Lean consumer

Integrated source commits: `55b23c986` (complete declaration map),
`48bcda3b3` (two fresh checker results and typed join) and `0bf036ab9`
(complete-map recheck after checker execution). The selected exact subject is
`spec://org.example/same-subject/Protocol#identity` in the fixture.

One opt-in `[composition]` declaration causes the host to scan Rust and Lean
together, write one complete map atomically, and require complete fresh input
for relation reads. The map retains distinct `Rust implements` and `Lean
verifies` edges to the same subject. Legacy Rust and TypeScript writers reject
the unknown composition table before touching an existing map; actual binary
probes preserved a sentinel. A Rust-only consumer still uses the old core
map. Missing, malformed, duplicate or foreign scanner inputs refuse before
partial publication. After the Rust and Lean checkers run, the complete map
is rebuilt; changed scan-root membership returns `composition_unstable`
instead of publishing an obsolete map digest.

The selected proof operation runs exact named Rust libtests from a captured,
dependency-free local source closure and a separately named Lean G1 model
check. It saves a versioned joined witness with distinct execution IDs,
source/spec/dependency/toolchain/test/axiom identities and
`correspondence=not_established`. `proof-verify` reruns both checkers and
rechecks saved bytes. CLI and MCP use the same engine and return the same
typed result. Their successful conjunction is **not** a theorem that the
Rust implementation refines the Lean model or a task acceptance decision.

Research receipts: `/fast/git/v/research/2026-10-05-lean-a0-3-implementation-receipt.json`
(SHA-256 `3847366c1b1cc9e771a7ae6bf06effa466c479384d3f2e420390146f1ca7c544`)
and `/fast/git/v/research/2026-10-05-lean-a0-3-final-map-recheck-receipt.json`
(SHA-256 `7867f522ff6feb71e4042ef1ca8535dada4b491c089cfad0107196dcee4c01f9`).
Exact checks in the implementation worktree passed: composition 5/5,
fast proof guards 6/6 after the final map fix, 18 negative assertions,
real check/verify 1/1, real CLI/MCP parity 1/1 and actual legacy-writer
refusals. Independent read-only review found no concrete P1/P2. On the
integrated NEXT tree, the parent reran only composition 5/5, real fresh
check/verify 1/1 and CLI/MCP real parity 1/1; all passed.

The supported slice is a selected native Linux, dependency-free default Rust
library plus local declared Lean modules. Source execution and toolchains are
trusted, not sandboxed. G1 is the maximum Lean grade; cross-package inbound
coverage is explicitly unsupported. The host currently links the Lean engine
by a repository source path; an installed-slot-only consumer has **not** been
proved. A0-GATE must retain that limitation and decide the exact installed
package proof before declaring the broader capability ready. No full product
gate, mutation campaign, public release or recheck of the historical 36 was
performed.
