# fractality — WAL (project continuation state)

_Updated: 2026-07-10 later (**Campaign 2 OPEN and EXECUTING** — owner:
«Начинай Campaign 2» + «Goal set: сделать Campaign 2». The plan is
authored and committed:
[`fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md`](fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md)
— initiative system per DEF-1: sessions+attribution → scoreboard → CC
hook adapter → nudges+routing-as-data → answer-rules slice → measured
cold trial (RP1 owner-gated). Prerequisite done: barkain deep-studied →
`spec/refs/notes/barkain-study.md` (BD1–BD6; survey delegated to GLM
over a sandboxed copy, boss spot-checked). New field law (measured
twice): opencode `run` auto-rejects reads outside launch cwd — delegate
inputs must be copied under the scratch cwd; heartbeats must be `echo`
commands (contract updated). Drift found: `fractality fetch` is
referenced by playbook/D12 but absent from the binary — repaired in
plan Ф3. Session scoreboard so far: delegated 2 (barkain survey 5.2;
cc-docs hooks extraction 5.2), kept: contract fix, spot-check reviews,
note decisions, plan authoring (never-delegate set). Next in-session:
Ф0 spikes (hook live-probes, statusline doc capture, trial menu,
attribution seam, settings ownership)._
_Prior: 2026-07-10 (IGNITION **CLOSED** — Phases 0–6 executed and
ledgered in one campaign; the plan's §2 execution record carries the
final verdicts). The delegate-out / collect-back loop, the swarm with
budgets and recursive kill, the ask_boss interaction layer, the
delegation-rules policy package, and the boss integration are all
live and proven on paid GLM workers. Manual tests MT-01…MT-05 recorded
green and **signed off by the owner (2026-07-10)** — the required
index is fully passed. The RP1 dogfood merged: the host's seven EULA
straggler manifests are UPL-1.0, relicensed THROUGH the fabric.
Campaign findings this session: **F17** (detached daemon inherited the
caller's `$()` pipe — `HANDLE_FLAG_INHERIT` stripped around autostart,
regression-pinned), **F18** (worktree workers' `result.md` collides at
multi-branch merge — procedure fix, product knob deferred), **F19**
(host-repo worktrees overflow Windows MAX_PATH — `core.longpaths=true`
in provisioning). Predictions: P1–P6, P8 CONFIRMED (P4 ratio 1.00;
P5 tree-dead 1025 ms; P6 2/2 with the cold-session caveat);
**P7 FALSIFIED** (~2× the commit estimate; every drift ledgered at its
boundary). Delegation scoreboard this session: **delegated 4,
delivered 4** (acceptance-runner tests turbo; admission-primitive
tests 5.2; two dogfood batches turbo) / kept with reasons (seam
design, spec/policy authoring, review, the F17/F19 fixes). Campaign
tally: delegated 8, delivered 7 (one Phase-2-era GLM failure was
re-landed by the boss). Reports for every phase + the campaign close:
[`reports/`](reports/)._
_Prior: 2026-07-10 late (Phase 3 EXECUTED — collect-back proven live,
MT-01 recorded; F16 profiles home-scoped). Prior: Phase 2 EXECUTED
(F14 Windows spawn seam, F15 exe-lock dev law). Prior: Phase 1
EXECUTED (F11 watch-not-Notify, F12 abort≠crash, F13 delegate context
economics). Prior: 2026-07-09 Phase 0 EXECUTED (F1–F10)._

## Current state

- **Campaign 2 (initiative system) is EXECUTING** — plan above; Ф0
  (spikes, no commits) is the current phase. IGNITION remains the
  closed prior campaign:
  [`fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md`](fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md)
  — §2 execution record + §14 per-phase ledgers are the canonical
  history; [`reports/`](reports/) carries the owner-facing narratives
  (one per phase + campaign-close).
- **Code:** six crates, three binaries. Live surface: `run` / `spawn`
  / `wait` / `ps` / `show` / `tree` / `kill [--tree]` / `questions` /
  `answer` / `stats` / hidden `mcp-broker`; admission (per-profile
  `max_concurrent`, FIFO, atomic claim); budgets (wall + tokens →
  `killed(budget)`, 1 s heartbeat kill delivery, taskkill fallback);
  Collected on the bus with D19 FileRefs (MC-minted etags); the
  ask_boss park/resume loop; `--allowed-tools`/`ask_boss` profile
  knobs; F17/F19 hardening.
- **Floor at close: all green** — conform 0 findings (6/6 gated; two
  recorded `#[spec(deviates)]` testimonies on the F17 kernel32 FFI),
  specmap 16 units / 47 items / 47 edges / 0 orphans, test-gate
  xfail-strict, ~120 tests.
- **Packages:** `org.vibevm.fractality/fractality` v0.1.0 (boot
  snippet 75, `fractality-delegate` skill, 3 [[binary]] decls) +
  `org.vibevm.fractality/delegation-rules` v0.1.0 (matrix, playbooks,
  boot snippet 77) — both vibe consumers with their own vibedeps.
- **This box:** real `~/.fractality` untouched by the MTs (scratch
  homes throughout). MC daemon **stopped**. Host repo carries the two
  dogfood merge commits; vendored vibedeps mirrors refreshed after the
  merge.

## Constraints (do not violate without discussion)

- Host Rules 1–4 bind every commit. The delegation law +
  live-observation protocol + two context scenarios; scoreboard in
  every WAL checkpoint. Clean-room law for refs. Worker env never
  inherits `ANTHROPIC_*`/`CLAUDE_*` (I1). MC is the bus; files are the
  persistence plane (I2). Publish is owner-word-only (RP3 still OPEN).
- **F15 dev law:** stop MC daemons before builds; corollary from this
  session — long-running manual tests and floors do not share a
  timeline.
- **Cwd law now binds the boss too:** pin the working directory in
  every gate/tool invocation — a misplaced floor once gated the HOST
  tree and reported green while five fractality findings sat unseen.

## Next (Campaign 2 is authorized and running)

1. **Execute Campaign 2 phases in order** (plan §8): Ф0 spikes →
   Ф1 sessions/attribution → Ф2 scoreboard → Ф3 CC adapter (+ `fetch`
   repair) → Ф4 nudges/routing-as-data/question push → Ф5 answer-rules
   slice → Ф6 trial (**RP1: owner must authorize the paid arms before
   they run**) → Ф7 close. RP2/RP3 defaults recommended in plan §13.
2. **Campaign 3 — RLM** (DEF-2) — unchanged, after C2.
3. Leftovers not absorbed by C2: server-side long-poll wait, F18
   result-path knob, `wait --verbose`, POSIX fallback kill semantics
   (DEF-8), `vibe skill install` projection on this box. (The `stats`
   monthly quota rollup and `fractality fetch` ARE absorbed — plan
   Ф2/Ф3.)
