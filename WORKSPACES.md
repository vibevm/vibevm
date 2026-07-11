# Workspaces

Sub-projects hosted in this repository that are worked on as
independent projects, each with its own boot contract, WAL, and
cold-resume file. Canon: `flow:org.vibevm/wal-workspaces`
(authored in `packages/org.vibevm/wal-workspaces/`); the local
grammar lives in `CLAUDE.md` §Workspaces.

| name | root | wal | continue | status |
|---|---|---|---|---|
| fractality | packages/org.vibevm.fractality/ | WAL.md | CONTINUE.md | 2026-07-11 — **Campaign 3 Stage B — Ф0/Ф1/Ф2 COMPLETE** (the need-gate decision core is in; floor green 184 tests / conform 0 / specmap clean; ~23 commits, pushed). **RP-C3-1 → Option B**; advisor → PP-003. **Ф0** spikes closed (jsonschema 0.47.0 confirmed on rustc 1.93.1). **Ф1** D-C3-2 packet + budget surface (`context_from`, `output_schema` + collect-seam validation, six-axis budget lattice). **Ф2** the need-gate: `decide` §10.3 procedure (inline\|route\|fold-local\|spawn\|escalate) + capability-class routing policy + profile class — a pure tested library, wiring → Ф3. **Next: Ф3** descent verbs (await/merge/isolation) + gate wiring (invocation, admission depth-guard, availability masking). **RP-C3-2 (paid trial arms) PRE-AUTHORIZED; §10 BINDING.** Prior: research foundation (RLM + Fugu), VISION V1–V5, C2, IGNITION closed. |
