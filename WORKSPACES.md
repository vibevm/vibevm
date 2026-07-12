# Workspaces

Sub-projects hosted in this repository that are worked on as
independent projects, each with its own boot contract, WAL, and
cold-resume file. Canon: `flow:org.vibevm/wal-workspaces`
(authored in `packages/org.vibevm/wal-workspaces/`); the local
grammar lives in `CLAUDE.md` §Workspaces.

| name | root | wal | continue | status |
|---|---|---|---|---|
| fractality | packages/org.vibevm.fractality/ | WAL.md | CONTINUE.md | 2026-07-12 — **Campaign 3 Stage B COMPLETE — the RLM is built and it runs** (all D-C3 landed, floor green throughout, pushed both remotes). Ф4 escalation (D-C3-6: terminal `RunState::Escalated` + `EscalationRecord` climbing parent edges, exit code 5, `escalations` inbox, `/escalate` endpoint + broker `escalate` MCP tool); Ф5 acceptance (FD-9: `output.verifier` marker + cold-verifier suppression, verifier-accept surfaced); **Ф6 trial FIRED** — MT-C3-01, 3 paid GLM gated-boss runs: **delegation 44.4% vs 16.7% C2 baseline (~2.7×), and fractality ran end to end as a product for the first time** (3 workers completed with results, 1 acceptance 1/1; P-C3-c CONFIRMED, P-C3-a SUPPORTED, P-C3-b/d inconclusive = menu gaps → PP-004). Delegation mechanism switched opencode→CC+z.ai (works — it IS the trial mechanism). Campaign-close: `reports/2026-12-07-06-48-campaign3-close.md`. **PP-003 advisor CORE also landed** (D-C3-7): `output.advice` marker + the RD-10 caller-class bar (`check_advisor_caller_class`) + denorm/surfacing/tests, floor green (test-gate 215); plan `FRACTALITY-ADVISOR-PLAN-v0.1`. **Next: a validated Stage C** (advisor help/hurt trial + uncertainty trigger + ladder-data) + PP-004 trial follow-ups, when the owner commissions them. Prior: Ф0–Ф3 (need-gate + descent core), research, VISION, C2, IGNITION. |
