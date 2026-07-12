# Postponed work — the registry

Work we decided, for a stated reason, not to do (yet): deferred
tasks, unfired protocols, dropped scope, parked ideas. This file is
the index — one line per item. The item itself is a **self-contained
markdown document** in [`postponed/`](postponed/), written so a cold
session can plan from it alone: the full task, why it was postponed,
what unblocks it, first steps when unblocked, canonical pointers.

Law (workspace [`CLAUDE.md`](../CLAUDE.md) §Postponed-work registry):
file the entry in the same session the postponement is decided; keep
the registry current-state, not a journal (picked up → flip status;
done → prune the entry, git keeps history; superseded → name by
what). This is an owner-facing dashboard — on any divergence the
spec tree (plan deferral ledgers, MT files, WAL) wins.

| id | title | filed | status | unblock |
|---|---|---|---|---|
| [PP-001](postponed/PP-001-rule-rp5-fire-mt-c2-05-rerun.md) | Rule RP5 — authorize and fire the MT-C2-05 re-run | 2026-07-10 | **RP5 RESOLVED 2026-07-12** (owner: 3+3 GLM cold boss cap 8; all paid runs authorized this goal) — ARMED, ready to fire (pre-reg frozen, harness arm a/b exists) | fire + score (this goal) |
| [PP-002](postponed/PP-002-def-c2-2b-worker-credibility.md) | DEF-C2-2b-full — acceptance-backed worker credibility on the boss surface | 2026-07-10 | **DONE 2026-07-12** — the credibility QUERY (`core::worker_credibility` → `CredibilityFact`, D7-factual, dated) + the SURFACE: `render_board` now shows "workers self-verify here: acceptance N/N green, last proven <age> (profile X)" on the cold board (SessionStart hook) and `fractality scoreboard`, only when a real completed-green acceptance backs it. The F24 keep-reason answered on the surface the boss reads. Optional follow-up: the mid-work nudge could cite it too | drained |
| [PP-003](postponed/PP-003-option-c-advisor-slice.md) | Option C — the advisor slice (V4) | 2026-07-11 | **CORE LANDED 2026-07-12** — `advice` marker + the RD-10 caller-class bar built + floor-green ([`FRACTALITY-ADVISOR-PLAN-v0.1.md`](../fractality/v0.1.0/spec/plans/FRACTALITY-ADVISOR-PLAN-v0.1.md)); trigger + ladder-data + help/hurt trial deferred to a validated Stage C | owner commissions the Stage C trial |
| [PP-004](postponed/PP-004-next-trial-improvements.md) | Next-trial improvements (worker caps, schema+Silo menu tasks, `decisions` verb) | 2026-07-12 | **DONE 2026-07-12** — all four items landed + the gated re-run fired & scored (MT-C3-03: P-C3-a CONFIRMED as a hard count, P-C3-b SUPPORTED, P-C3-d CONFIRMED). Minor open: run-3 re-fire for a clean N=3; a completed schema-worker to close P-C3-b | drained (kept for the two minor follow-ups) |
