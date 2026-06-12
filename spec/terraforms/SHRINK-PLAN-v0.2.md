# vibevm Shrink Plan v0.2 — the three reserved moves
**status: EXECUTED 2026-06-12 (same day as authored) · vibevm-specific · the follow-up to SHRINK-PLAN-v0.1**

*Execution record: all three moves ran to completion in one pass, as directed. (1) The unsafe-gate posture (AUD-0016): frontend v5 gives `UnsafeUse` the same test/deviates scoping `UnwrapUse` earned in v4 (and starts seeing unsafe impl methods); the rule honors fn-grain testimony per ENGINE-CONFORM §4; **`env-audit`** lands as the designated audit crate — one serialized, restoring `EnvGuard` behind a safe API replaces three hand-rolled test guards whose own SAFETY comment admitted a race; the two immovable production boundaries (vibe-cli startup env promotion, vibe-index `libc::kill`) testify; baseline 10 → 2 by pure drain. (2) `CONFORM_GATED` grew to vibe-core and vibe-index with **zero frozen entries**: all 40 entry findings (4 enum REQ edges, 21 Class-F messages, 15 unwraps — two more latent `VersionReq::parse("={v}")` build-metadata panics among them) were drained before the gate flipped; the baseline never widened. (3) The **`vibe-install` orchestrator crate** rebuilt from the CLI's pipeline: plan → confirm → apply over the `InstallSource` seam, typed `PlanEvent`s, R-001 cell construction left at the CLI's composition root, born conforming inside `CONFORM_GATED` (now 11 crates), `[lib] test = false` + safely-named integration tests against the os-740 UAC trap, and `PROP-003#req-conditional-fixpoint` finally carries its implements edge. Five-gate panel green on the final tree: specmap 442/407/417/0, conform 2 frozen / 0 new, test-gate 1132/0/3 xfail-strict, fast-loop 20/20, self-check all four steps. The WAL carries the full checkpoint.*

---

The owner's directive, verbatim:

1. What to do

- Еxpand CONFORM_GATED to vibe-core / vibe-index (their error enums and seams would freeze new findings).
- Redesign the unsafe-gate posture (as in AUD-0016).
- Build the vibe-install orchestrator crate the audit sketched — Phase 4c's install.rs split keeps the door open

Do it all in one pass/goal, whithout interruptions for questions.

2. What this plan deliberately does NOT do

Does NOT touch DBT-0020 or the two MCP files (owner instruction, 2026-06-12) — they park at exit as the baseline's residual 2.
