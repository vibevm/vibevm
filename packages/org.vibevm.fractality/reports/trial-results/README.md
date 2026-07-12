# Trial results — durable evidence for paid / important runs

**Owner directive (2026-07-12):** «Все результаты прогонов наших тестов
стоит сохранить куда-нибудь в notes, иначе работа как будто бы будет
потрачена зря» — and its follow-ons (save non-harness runs too, by judgment;
date every run; group related runs; give each group a README).

The trial runner writes each run's artifacts to
`fractality/v0.1.0/target/trial-results/`, but `target/` is **gitignored**,
so a `cargo clean` or fresh checkout would erase the evidence of paid GLM
runs. This committed directory preserves it, so the paid work is never wasted.

## Layout — dated groups of dated runs

Related runs live under ONE **dated group** directory, and each run dir is
**also dated**, in the report convention `<год-число-месяц>-<HH-MM>-<name>`
(reversedate-forwardtime-description):

```
reports/trial-results/
  2026-12-07-05-49-c3-f6-gated/            # a group (one trial / campaign phase)
    README.md                              # what it was + summary results
    2026-12-07-05-49-arm-g-run-1/          # a dated run
      run-info.txt, runs.json, decisions.json, escalations.json,
      stats.json, scoreboard.txt, sessions.txt, forest.json,
      boss-stderr.log, boss-transcript.jsonl.gz   # transcript gzipped ~10:1
    2026-12-07-06-17-arm-g-run-2/  …
```

**Excluded** from every run: `proj-final/` (a whole-repo copy of the staged
project) — tens of MB and fully reproducible, so it stays only in `target/`.
290 MB of raw runs becomes ~6 MB of durable evidence.

## A README wherever there is meaning

A `README.md` describes **what the test was** (pre-registration / MT id, the
arms, what it measured) and **its summary results** once understood, and is
**amended** when a later analysis pass produces verdicts. One lives at every
level that carries its own meaning:

- a **group** of related runs → a group README (the helper scaffolds a stub);
- a **standalone test** (no group) → its own README;
- an **individually-meaningful run** inside a group → its own README too;
- a **pure replicate** (one of N equivalent runs) → none; the group covers it.

The MT document's "Recorded runs" section stays the canonical scored
narrative; the README here is the at-a-glance summary that travels with the
raw evidence.

## Current groups

| group | campaign | pre-reg | headline |
|---|---|---|---|
| `2026-10-07-14-40-c2-f6-initiative` | C2 Ф6 | MT-C2-01 / -04 | arm A 16.7%, arm B 0% (hooks falsified; F23/F24/F25) |
| `2026-12-07-05-49-c3-f6-gated` | C3 Ф6 | MT-C3-01 | gated 44.4% vs 16.7% naive (~2.7×); ran end to end |

## The convention (standing rule — see workspace CLAUDE.md)

After every trial fire (and, by judgment, any important/long run whose
results carry value):

```sh
cd fractality/v0.1.0
bash spec/manual-tests/trial/save-results.sh <group-description>
# fill in the scaffolded reports/trial-results/<dated-group>/README.md
cd ../.. && git add reports/trial-results && git commit -m "test(fractality): preserve <group> paid-run evidence"
```

A run is not "done" until its evidence is committed.
