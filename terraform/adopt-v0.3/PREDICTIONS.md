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
| P1-1 | Phase 1 / card scaffold-e-fast-loop | Per-cell first-signal latency < 60s for ≥90% of cells; cells failing this reveal hidden coupling (debt logged) | 2026-06-11 | **held immediately, stronger than predicted**: 18/18 cells (100%) within the 60s budget on a warm target — worst cell `vibe-cli` at ~23s, a 2.6× margin. No hidden coupling surfaced; every cell builds and tests in isolation. The four initially-RED cells were a checker artifact (nextest exit 4 on zero-test crates), not coupling — fixed with `--no-tests=pass`; a zero-test cell's build IS its first signal. |
| P2-1 | Phase 2 / cards scaffold-f + scaffold-g | Iterations-to-green on a sample modification task drop vs the Phase 1 baseline | 2026-06-11 | **pending** — measurement deferred by owner decision; the instrumentation hook exists (every conform finding now carries a REQ URI + fix surface, so an agent transcript can be scored for how directly the fix followed the hint). To be scored when a measured pilot task runs. |
| P2-2 | Phase 2 / card scaffold-g-doctests | Doctested seams stay truthful: a doctest that drifts from reality fails the loop rather than misleading the next reader | 2026-06-11 | **standing** — structural by construction (30 compiled doctests across vibe-resolver, conform-core, specmap-core run in fast-loop via `cargo test --doc` and in self-check); falsified the day a seam change ships green while its doctest still shows the old idiom. |
