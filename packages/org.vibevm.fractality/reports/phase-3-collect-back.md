# Phase 3 report — collect-back (retrospective)

_Campaign: FRACTALITY-IGNITION v0.1 · Phase 3 · executed 2026-07-10 ·
written retroactively at campaign close from the §14 ledger (commits
`799dba3`, `1fb9517`, `01b22d3`, `eb8e7d9`); the ledger stays
canonical._

## What the phase built

Returns as first-class artifacts: the tolerant stream-json parser
(D14/R2 — unknown kinds counted, never fatal; the `result` event is
authoritative because this provider's assistant events under-report);
the pod's tee pump with watch-channel live metering (`PodEvent::Usage`
snapshots — a run is meterable mid-flight); result provenance
(`worker` | `extracted` | `none`, with the path) + `usage.json`; the
pod-side acceptance runner (packet `task.acceptance` → per-command
verdicts in status.json, evidence in acceptance.log, 600 s per-command
cap, skipped-with-reason on failed workers); exit-code families
(`killed(pod_lost)` → 2 infra; policy kills keep 3); `run`/`show`
rendering usage + cost + result + acceptance.

## The exit proof

MT-01 (manual-test #1) pre-run green on a scratch home against live
GLM-5.2: run `01KX4JRBNQ774N0G9VYG218TKD`, 36 s, 599 events, cost
$0.1336, result provenance `worker`, acceptance 1/1 (`cargo test`
green over the worker's own four unit tests in 366 ms). P3 running
count 3/3 at the boundary.

## Strange things / paid-for lessons

- **F16 — profiles are home-scoped:** a scratch `--home` needs its own
  `profiles.toml`; found by MT-01's first pre-run, and the D14 error
  contract earned its keep in the field — mission-control refused the
  run with a 400 naming the spec anchor and the exact fix.
- The stream parser + goldens were delegated (scenario 1: exact API +
  golden numbers compiled in) and landed green first try — one
  misleading doc sentence fixed at review.
- conform forced the pod's `collect` cell split at the 600-line budget
  — the discipline's cell economics started steering the architecture,
  a rhythm that continued through Phase 4b's splits.
- Deliberate deferral, by name: the `Collected` bus event + FileRef
  rendering — the swarm's remote reads need them, the sync loop does
  not (they landed in Phase 4 exactly as scheduled).
