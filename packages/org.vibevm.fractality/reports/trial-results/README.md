# Trial results — durable evidence for paid runs

**Owner directive (2026-07-12):** «Все результаты прогонов наших тестов
стоит сохранить куда-нибудь в notes, иначе работа как будто бы будет
потрачена зря.»

The trial runner writes each run's artifacts to
`fractality/v0.1.0/target/trial-results/arm-<arm>-run-<n>/` — but `target/`
is **gitignored** (cargo build output), so a `cargo clean` or a fresh
checkout would erase the evidence of paid GLM runs. This directory is the
**committed** home that preserves it, so the paid work is never wasted.

## What is here (and what is not)

Per run, the small evidential subset is copied verbatim and the transcript
is gzipped (jsonl compresses ~10:1, keeping the repo lean):

- `run-info.txt` — the arm/run/boss line + `boss_exit` + wall seconds.
- `runs.json` — every worker run the boss spawned (`ps --json`).
- `decisions.json` — the need-gate decision journal (arm g/g2 only).
- `escalations.json` — escalated runs, if any.
- `stats.json`, `scoreboard.txt`, `sessions.txt`, `forest.json` — bus facts.
- `boss-stderr.log` — the boss session's stderr.
- `boss-transcript.jsonl.gz` — the full boss stream-json, gzipped (the
  richest evidence: the boss's reasoning + every tool call). `zcat` to read.

**Excluded:** `proj-final/` (a whole-repo copy of the staged project after
the run) — tens of MB per run and fully reproducible, so it stays only in
the ephemeral `target/` dir, never committed.

## Arm legend

| prefix | campaign | pre-registration | what it measures |
|---|---|---|---|
| `arm-a-run-*` | C2 Ф6 | MT-C2-01 / MT-C2-04 | cold-boss delegation, snippet-only baseline |
| `arm-b-run-*` | C2 Ф6 | MT-C2-04 | same + `harness install` (initiative hooks) |
| `arm-g-run-*` | C3 Ф6 | MT-C3-01 | the RLM gated arm (44.4% delegation) |
| `arm-g2-run-*` | C3 (PP-004) | MT-C3-03 | gated re-run, extended menu (schema + Silo) |
| `advise-*-run-*` | C3 (Stage C) | MT-C3-02 | the advisor help/hurt paired arms |

The **scored verdicts** live in each MT document's "Recorded runs" section
(the canonical narrative); this directory is the **raw evidence** behind
those numbers, auditable and re-scorable.

## The convention (standing rule)

After every paid trial fire, run the durable-save helper and commit:

```sh
cd packages/org.vibevm.fractality/fractality/v0.1.0
bash spec/manual-tests/trial/save-results.sh     # target/ → reports/trial-results/
cd ../.. && git add reports/trial-results && git commit -m "test(fractality): preserve <arm> paid-run evidence"
```

Never wait to preserve: a paid run whose evidence lives only in `target/` is
one `cargo clean` from wasted.
