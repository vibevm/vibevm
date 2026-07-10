# fractality — WAL (project continuation state)

_Updated: 2026-07-10 (IGNITION **CLOSED** — Phases 0–6 executed and
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

- **The campaign is CLOSED.** Plan:
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

## Next (candidates for the owner to choose from — not authorisation)

1. **Campaign 2 — the initiative system** (plan §15 DEF-1): scoreboard
   -driven delegation nudges for a cold boss; the P6=100%-warm number
   is the floor to beat; routing-as-data + question push-notifications
   + D18 layer 2 (dynamic permission brokering) are its natural cargo.
2. **Campaign 3 — RLM** (DEF-2, owner hypothesis recorded in the
   plan).
3. Small named leftovers: server-side long-poll wait, F18 result-path
   knob, monthly quota rollup in `stats`, `wait --verbose`, POSIX
   fallback kill semantics (DEF-8), `vibe skill install` projection of
   fractality-delegate on this box.
