# Workspaces

Sub-projects hosted in this repository that are worked on as
independent projects, each with its own boot contract, WAL, and
cold-resume file. Canon: `flow:org.vibevm/wal-workspaces`
(authored in `packages/org.vibevm/wal-workspaces/`); the local
grammar lives in `CLAUDE.md` §Workspaces.

| name | root | wal | continue | status |
|---|---|---|---|---|
| fractality | packages/org.vibevm.fractality/ | WAL.md | CONTINUE.md | 2026-07-10 — **Campaign 2 CLOSED, rulings recorded, DEF-C2 slice landed** (floor: 164 tests, conform 0, specmap 19/63/63/0). Trial: six GLM cold-boss runs, **arm A 16.7% · arm B 0%** — P1 confirmed, P3 falsified with mechanics (F23/F24/F25); P2/P4–P8 confirmed. Owner signed MT-C2-01…04; RP2 ON, RP3 settings.local.json — **RP1–RP4 all resolved**. The falsifier repairs shipped same-day on owner order: runner toolchain passthrough (repro-verified), cold-start board (no zero-counter anti-proof), mid-work nudge via PostToolUse (shared cooldown, staged smoke green, P95 50 ms), and **MT-C2-05 — the pre-registered re-run, paid arms gated on RP5 (OPEN)**. Next decision: rule RP5, or DEF-C2-2b-full, or Campaign 3 (RLM). Prior: IGNITION CLOSED (MT-01…05 signed off). |
