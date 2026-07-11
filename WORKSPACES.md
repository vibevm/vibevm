# Workspaces

Sub-projects hosted in this repository that are worked on as
independent projects, each with its own boot contract, WAL, and
cold-resume file. Canon: `flow:org.vibevm/wal-workspaces`
(authored in `packages/org.vibevm/wal-workspaces/`); the local
grammar lives in `CLAUDE.md` §Workspaces.

| name | root | wal | continue | status |
|---|---|---|---|---|
| fractality | packages/org.vibevm.fractality/ | WAL.md | CONTINUE.md | 2026-07-12 — **Campaign 3 Stage B — Ф0–Ф3 COMPLETE; the descent core is in** (floor green test-gate 203 / conform 0 / specmap clean; pushed both remotes). **Ф3** wired the Ф2 need-gate from an uncalled library into a working descent core across 9 floor-green slices: depth-guard (D-C3-3), `fractality gate` invocation + decision journal (D-C3-8), await `--any`, refuse-near-duplicate, availability masking (FD-8), retry-on-violation (D-C3-2), merge-node designation (D-C3-4/5), plus a real `max_depth=0` overload fix (routing "no-spawn" vs need-gate "unlimited" → `GateInputs.can_spawn`). Phase report: `reports/2026-12-07-02-40-campaign3-f3-descent-core.md`. **Next: Ф4 escalation (D-C3-6)** — a terminal `RunState::Escalated` + `EscalationRecord` climbing the parent edges (Ф0 s4 design). **RP-C3-2 (paid trial arms) PRE-AUTHORIZED; §10 BINDING.** Prior: Ф0–Ф2 (need-gate core), research (RLM + Fugu), VISION V1–V5, C2, IGNITION. |
