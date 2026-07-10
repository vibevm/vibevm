# Phase 1 report — workspace skeleton + mission-control core (retrospective)

_Campaign: FRACTALITY-IGNITION v0.1 · Phase 1 · executed 2026-07-09/10
· written retroactively at campaign close from the §14 ledger (commits
`2502f68`…`dd895d0`); the ledger stays canonical._

## What the phase built

The six-crate Cargo workspace (core, mission-control, pod, mc-client,
backend-claude-code, cli) with the journal-backed daemon: append-only
JSONL + a pure replay fold shared verbatim with the live write path
(disk and memory cannot drift), lockfile + bearer discovery, pod
register/heartbeat/event legs, run registry and the reaper, `mc
start|stop|status` / `ps` / `show` with D17 output rules from day one.
The workspace simultaneously became the vibevm pilot consumer (redbook
^0.2.0 + rust-ai-native ^0.7.0, 26 packages in vibedeps, generated
boot lane) and adopted the FULL discipline at birth (owner directive:
DEF-9 resolved early — 6/6 crates conform-gated at an empty baseline
after a 51-finding one-pass drain; specmap 31 edges, 0 orphans).

## The exit proof

Real-binary lifecycle green on this box, and the P8 early signal
CONFIRMED the hard way: a real daemon process killed mid-run — the pod
kept supervising, re-registered with the new generation, and the run
completed with zero manual repair (adoption is a protocol feature, not
journal archaeology).

## Strange things / paid-for lessons

- **F11:** `Notify::notify_waiters` loses wakeups against
  not-yet-polled waiters — a zero-CPU permanent hang, hit live.
  Lifecycle signals are *state* (`watch`) everywhere since.
- **F12:** aborting an embedded server's accept loop does NOT sever
  pooled keep-alive connections — a pod once delivered its exit report
  to a "dead" generation. True crash tests kill real processes.
- **F13:** delegate context economics measured — a GLM delegate reads
  only its targets (~15k cached prefix), stdout end-buffers under
  redirection (telemetry rides file mtimes + PROGRESS markers); the
  live-observation protocol and the two context scenarios became
  contract law after the profiles module failed twice at GLM
  (poisoned cwd; silent planning).
- **P7 drift:** 3 planned commits became 7 — owner-directive scope
  (discipline adoption, pilot posture, vendoring) folded mid-phase,
  recorded rather than absorbed.
