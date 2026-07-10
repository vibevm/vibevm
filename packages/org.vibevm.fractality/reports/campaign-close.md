# Campaign close report — FRACTALITY-IGNITION v0.1

_Executed 2026-07-09 → 2026-07-10 (three working sessions). This
report is the owner-facing narrative; the plan's §2 execution record
and §14 ledgers stay canonical. Reports per phase live beside this
file._

## The verdicts on all eight predictions

| # | Prediction | Verdict |
|---|---|---|
| P1 | nested headless spawn works on this box | **CONFIRMED** (Ф0.s2; product path since) |
| P2 | GLM stream-json is metering-complete | **CONFIRMED** (Ф0.s4 early; product transcript Phase 2; result event authoritative — provider's assistant events under-report, folded into D14) |
| P3 | ≥9/10 first real packets produce result.md unnudged | **CONFIRMED 10/10** (every completed packet through MT-02 firing #3 wrote worker-provenance result.md; killed runs excluded as deliberate interruptions) |
| P4 | 3-worker swarm, zero conflicts, wall < 1.6× slowest | **CONFIRMED** (MT-02: ratio 1.00 — swarm wall == slowest single; zero worktree conflicts; the one merge conflict was the F18 report-file collision, a procedure fix) |
| P5 | recursive kill < 2 s, zero orphans | **CONFIRMED** (MT-03: depth-2 live tree dead in 1025 ms; orphan sweeps clean; Job Object guarantee held) |
| P6 | boss delegates ≥50% of eligible dogfood grunt | **CONFIRMED 2/2 = 100%**, with the honest caveat recorded (the measuring session built the fabric; a cold session's propensity is Campaign 2's baseline to establish) |
| P7 | campaign lands in ≤17 commits matching planned subjects | **FALSIFIED** (≈2× over). Causes, all recorded at their boundaries: owner-directive scope folds (full discipline adoption, pilot posture, vendoring), found-work fix commits (F14–F19), and house flows that require their own docs commits (manual tests, ledgers, WAL). The drift mechanism worked — every deviation is in a ledger — but the estimate itself did not survive contact. Lesson: commit-count predictions must either budget directive scope or predict subjects, not counts. |
| P8 | pod overhead < 1 s; MC kill-and-restart loses nothing | **CONFIRMED** (Phase 1 boundary: real process kill, pod re-registered, run completed; adoption-as-protocol held through every later scratch-daemon generation) |

## What exists now (the §4 exit state, reconciled)

- **fractality v0.1.0**: 6 crates, 3 binaries — the full loop live:
  register → admission (per-profile slots, FIFO) → worktree/dir
  provisioning → detached pod → clean-slate worker env → GLM worker →
  stream metering → collection with provenance + acceptance verdicts →
  Collected on the bus with a D19 FileRef → semantic exit codes; async
  verbs (spawn/wait/tree/kill --tree), budgets (wall/token →
  killed(budget)), the ask_boss broker (park/resume, questions/answer),
  stats over /v0/metrics.
- **delegation-rules v0.1.0**: the decidable matrix + two
  field-calibrated playbooks + template (boot slot 77).
- **Boss integration**: boot snippet 75, the fractality-delegate
  skill, [[binary]] declarations.
- **5 manual tests** recorded with live outputs (MT-01…MT-05), all
  green on their final firings and **signed off by the owner
  (2026-07-10)** — the index required for the shipped features is
  fully executed and passed.
- **Dogfood delivered**: the seven EULA straggler manifests (+ licence
  texts) relicensed to UPL-1.0 through the fabric itself, reviewed and
  merged per RP1's acceptance.
- Floor at close: all green — conform 0 findings (6/6 gated, one
  recorded F17 deviation testimony ×2), specmap 16 units / 47 items /
  47 edges / 0 orphans, test-gate xfail-strict.

## The findings ledger (F1–F19 in one glance)

Phase 0: F1–F10 (env, spawn, provider facts, GLM smoke, kill-tree
mechanism, permission surface, clean-room intake, landscape, MSRV,
host gate). Phase 1: F11 (Notify lost-wakeup → watch), F12 (in-process
abort ≠ crash), F13 (delegate context economics → the two-scenario
law). Phase 2: F14 (the Windows spawn seam: PATHEXT resolve, stdin
prompt, env casing), F15 (daemon holds the exe against builds).
Phase 3: F16 (profiles are home-scoped). Phase 4: **F17** (detached
daemon inherits the caller's substitution pipe — `$(fractality
spawn)` hung; HANDLE_FLAG_INHERIT stripped around the spawn, pinned
by test), **F18** (worktree-mode worker reports collide at
multi-branch merge — procedure fix, product knob deferred). Phase 6:
**F19** (host-repo worktrees overflow Windows MAX_PATH —
`core.longpaths=true` on provisioning/removal).

## Deferrals seeding the next campaigns (§15 unchanged + new)

Campaign 2 (initiative system) inherits: the P6 cold-baseline
question, routing-as-data, question push-notifications, dynamic
permission brokering (D18 layer 2). Campaign 3: RLM. New named
deferrals from execution: server-side long-poll wait (DEF-6 family),
result-path knob for worktree mode (F18), monthly quota rollup in
stats, `wait --verbose` parked echo, admission spawn_blocking
refactor, POSIX fallback kill group semantics (DEF-8).

## One-line honest summary

The mandate's phases 1–4 (delegate, collect, swarm, rules) plus the
scheduler backbone are **live and proven on paid workers**; the
delegation calculus routes real tasks decidably; the dogfood shipped
real host value through the fabric; and the campaign's two falsified
expectations (commit count; a "floor" run from a poisoned cwd) both
produced recorded, mechanism-level lessons rather than silent drift.
