# F15 — Consistent package admission and exact in-place preparation

Accepted at the safety boundary by the local commit containing this record. Configured-package runtime delivery remains open.

Two real scoped-update failures exposed independent mutation paths that bypassed the normal install guard. A valid common-configured new version and sibling authored input both returned success, ran hooks and changed lock/slots/boot without consuming the configuration. The legacy update and normal refusal controls distinguished the defect from malformed input.

The correction reuses one install-layer workspace/candidate policy with existing typed errors. It covers the actual consumed world before publication, checkout, pruning, hooks or boot/lock changes. In-place source preparation is ephemeral and bound to the actual Git source and immutable revision, rather than stale slot metadata or another metadata provider. The same URL was insufficient: a real local/conditional transport counterexample selected different commits until native original-repository acquisition was used.

Preflight and post-acquisition failures remain distinct. Missing or malformed acquired manifests and attribute-read failures cannot fall through to a valid mirror; source acquisition failures retain their mirror controls. Planning publication now follows admission, including the real Fresh source-delta failure. The format/API account is [007](../../../formats/breaks/007.md).

## Evidence and limits

- Install consumers: 69 top-level tests; orchestrator install: 32; new CLI guards: 12. Legacy Git/update/reinstall controls: 4/2/5, plus the raw/finalized delta unit. Nested fixture-helper output is not additional coverage.
- Prepared Git source: 23 tests; prior incumbent ShellGit/in-place panels: 39/2; prepared docs: 2. Consumer docs: install11, orchestrator76, package-source6. Exact source/gate qualifications remain in the archived reports.
- Strict clippy passed; conform reports zero new findings across the five affected crates; scoped formatting and diff checks passed.
- Root verified 49 source hashes and 32 consumer log hashes, plus the separately frozen 15-path registry/source manifest. Fresh map has 9080 units, 5124 tagged items, 4279 edges, zero gated orphans and zero unresolved host edges; 43 existing warnings remain.

Tests use real local Git, actual CLI/project mutation snapshots, and explicit cache/object exceptions. No native Windows/macOS or hostile-process authentication guarantee is inferred. Fixture errors, ENOSPC, the wrong transport counterexample and subsequent phase-routing failures are retained in LOG/evidence.

This closes a safety prerequisite. It does not implement optional package activation, cross-package configured solving/forwarding, rich package lock replay or the full F04/all-kind matrix, and does not close an old M05 task by itself. The positive A1–A5 route remains required.

Evidence: `/tmp/nst-common-admission-report.md`, `/tmp/nst-common-admission-files.json`, `/tmp/nst-common-admission-evidence.json`; `/tmp/nst-prepared-inplace/`; exact user-local `evidence/2026-11-05/common-*` and `prepared-inplace-*`.
