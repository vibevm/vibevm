# Pilot Prediction Ledger — Discipline v0.2 adoption (TERRAFORM-PLAN-v0.3)

Measurement is deferred by owner decision; every adopted card and
every phase records a **falsifiable prediction** instead
(Manifesto §7). This ledger is exit-criterion 3 of the adoption
(plan §5) and the raw input for the closing REPORT and the
Discipline's v0.3 revision. Predictions are recorded when the card
is adopted (phase start) and verdicts are filled in when evidence
arrives — `pending` until then. Never edit a recorded prediction;
append a correction entry instead.

| ID | Source (card / phase) | Prediction | Recorded | Verdict |
|---|---|---|---|---|
| P0-1 | Phase 0 (adopt & shim) | Relocation is behavior-neutral; conform-baseline unchanged | 2026-06-11 | **held, with a correction**: the frozen set is unchanged (the same six unsafe findings) but one entry needed a line correction 33→35 — the relocate's cache invalidation exposed a pre-existing conform engine defect (stale facts cache had masked the `a9dc160` line shift; see LOG 2026-06-11). Behavior-neutrality itself held: no test, no resolver output, no boot artifact changed semantically. |
| P0-2 | Phase 0 (adopt & shim) | specmap regenerates deterministically; 177 edges / 0 suspects preserved through the relocate | 2026-06-11 | **held**: 352 units / 170 items / 177 edges / 0 suspects, `--check` clean on repeated runs across the relocate and the fmt pass. |
