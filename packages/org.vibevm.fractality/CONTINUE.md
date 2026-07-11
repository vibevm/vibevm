# fractality — cold-resume checkpoint

_Written 2026-07-11 ~12:45 at session wind-down. `WAL.md` (same
directory) is the canonical living state and supersedes this
snapshot wherever they diverge._

**You, the reading session, are most likely an Opus-class model —
the handoff to non-Fable executors is deliberate (owner's order,
recorded 2026-07-11). Before doing ANYTHING in Stage B, read
`fractality/v0.1.0/spec/plans/FRACTALITY-RLM-PLAN-v0.1.md` §10
("Executor's guide") — it was written specifically for you and it
is binding.**

## TL;DR

One long session produced the complete research foundation for
Campaign 3 and the handoff package: **(1)** the postponed-work
registry (`plans/postponed.md`: PP-001 = fire MT-C2-05 once RP5 is
ruled; PP-002 = acceptance-backed credibility facts); **(2)** the
owner's VISION (`fractality/v0.1.0/spec/VISION-RECURSIVE-FABRIC.md`
— V1 recursive promotion, V2 RLM, V3 escalation, V4 advisor
ladder Mythos>Fable>Opus>GLM-5.2>Sonnet>GLM-5-Turbo>Haiku, V5
attachable headless terminals); **(3)** RLM research CLOSED — 15
sources studied clean-room, 11 notes, synthesis `RLM-SYNTHESIS.md`
with deltas RD-1…21; **(4)** Sakana Fugu research CLOSED — 4
notes, `FUGU-SYNTHESIS.md` (FD-1…16), `FUGU-FRACTALITY-MAPPING.md`
(headline: Fugu is our thesis, shipped, minus our enforcement/
transparency/escalation planes; its long-context loss argues FOR
our descent core); **(5)** the Stage B implementation-plan draft
revised with Fugu findings and finished with the §10 executor
guide. Everything is docs — zero product-code changes; floor
green (164 tests / conform 0 / specmap 170 units / 0 orphans).

## Where work stands

- Branch `main`, in sync with origin after the wind-down fan-out;
  working tree clean. Session commit chain: see WAL §Current
  state (15 commits `0af23db` … `4ccb7af` + wind-down).
- **No active blocker.** Two open decisions, both the owner's:
  - **RP-C3-1** (`spec/plans/FRACTALITY-RLM-PLAN-v0.1.md` §8):
    commission Stage B — scope **A** (descent only) / **B**
    (descent + escalation; recommended — the Silo theorem makes
    ascent part of descent's correctness) / **C** (+ minimal
    advisor); budget posture; timing; MT-C2-05 vs MT-C3-01
    firing order.
  - **RP5** (`fractality/v0.1.0/spec/manual-tests/MT-C2-05-initiative-rerun.md`):
    authorize the initiative re-run arms (recommend 3+3 GLM,
    cap 8). Exact recipe: `plans/postponed/PP-001-rule-rp5-fire-mt-c2-05-rerun.md`.

## Next-steps recipe (cold start, per decision)

**If the owner rules RP-C3-1:** record the ruling verbatim in the
plan §1 and §8 → follow §10.1's reading order (workspace CLAUDE.md
→ the plan whole → the two syntheses §3 → notes only as cited) →
execute Ф0 spikes (no commits) → slices per §10.7 (one D-C3 = one
commit, floor after each, specmap re-mint with anchored spec
edits). Never open `refs/src/*` or `refs/papers/*` while coding
(§10.4 — legally load-bearing).

**If the owner rules RP5:** open PP-001 and follow its five steps
verbatim (archive Ф6 trial dirs first; live-observation law with
first-output timeout ≤3 min).

## Non-obvious findings this session (do not rediscover)

- **Research corpus resolution:** RD-n → `spec/refs/notes/RLM-SYNTHESIS.md`
  §3; FD-n → `spec/refs/notes/FUGU-SYNTHESIS.md` §3; per-source
  evidence → the study notes in the same directory. INVENTORY
  (S1–S30) holds every license verdict; git history proves
  verdicts precede study.
- **Fugu mechanics worth citing:** orchestration collapse →
  isolation-by-default; access lists → `context_from`;
  verifier-accept completion + cold-verifier suppression;
  availability masking; soft-label routing tables from measured
  outcomes; per-step worker alternation is where its wins live.
- **Counterpoint numbers** (cite against over-recursion): Kimi K2
  86.6%→60% when wrapped; depth-2 latency 3.6s→344.5s; GPT-5-nano
  −9.5pp (small models can't drive a REPL — spawn-shaped recursion
  is the small-model path, THREAD +10–50pp).
- **opencode delegates stall silently at launch** (0-byte log ≥14
  min, twice) — kill and relaunch cures it; always arm a
  first-output alarm (`for i in $(seq 1 36); do [ -s log ] && …`).
- **Glob tool is unreliable on this tree** — use PowerShell
  listings/`git ls-files`.
- W1 deep-research of Stage A2 was stopped mid-flight (owner token
  pause) and abandoned by owner order — P-F2 honestly NOT
  EVALUATED; raw Wave-2 record: `refs/study/fugu-waves/`.
- The official `SakanaAI/fugu` repo has **no license** (README
  distance only); `Sakana-AI-labs/Sakana-Fugu` is a **squatter** —
  never intake.

## Repository map (workspace)

`packages/org.vibevm.fractality/` — CLAUDE.md (contract), WAL.md
(canonical state), this file, VIBEVM-BACKLOG.md, **plans/**
(postponed.md + postponed/PP-001,PP-002), **reports/** (IGNITION,
C2, rlmresearch drafted/started/completed, fuguresearch
started/completed dashboards); `fractality/v0.1.0/` — the Cargo
workspace: crates/{core,mission-control,pod,mc-client,
backend-claude-code,cli,initiative}; spec/ (PROP-001,
VISION-RECURSIVE-FABRIC, plans/{IGNITION✓,INITIATIVE✓,
RLM-RESEARCH✓,FUGU-RESEARCH✓,**RLM-PLAN v0.1 draft+§10**},
manual-tests (MT-01…05✓, MT-C2-01…04✓, MT-C2-05 RP5-gated),
refs/{INVENTORY.md S1–S30, notes/ = 15 RLM + 4 Fugu notes + 2
syntheses + mapping + selection}); vibedeps/ + conform/specmap
configs; `delegation-rules/v0.1.0/` — the policy package. Host
`refs/` (gitignored): src/{rlm,roma,fast-rlm,redel,recursive-llm,
openfugu…}@pins, papers/ (10 PDFs+txt), articles/ (snapshots),
study/ (wave records + delegate scratch).

## Quick-start

```sh
cd packages/org.vibevm.fractality && head -60 WAL.md
cd fractality/v0.1.0
/c/Users/olegc/gits/vibevm/packages/org.vibevm/rust-ai-native-lang/v0.7.0/target/debug/rust-ai-native.exe floor
```

Resume phrase: `восстанови сессию fractality` (report-then-wait).
Wind-down: `заверши сессию fractality`.
