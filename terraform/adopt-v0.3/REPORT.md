# terraform/adopt-v0.3/REPORT.md — the Discipline v0.2 adoption, close-out report

_The exit artifact of [`TERRAFORM-PLAN-v0.3`](../../spec/terraforms/TERRAFORM-PLAN-v0.3.md)
(plan §5, criterion 4) and the input to **Discipline v0.3** — modeled
on the prior terraform's [`REPORT.md`](../REPORT.md). Everything here
is backed by [`LOG.md`](LOG.md) (the raid-grained account) and
[`PREDICTIONS.md`](PREDICTIONS.md) (the falsifiable-prediction
ledger); this file is the synthesis._

Executed 2026-06-11, one continuous effort, all phases 0–7 of the
plan plus the priority-cell sweep. Measurement was deferred by owner
decision throughout; every adopted card carries a recorded prediction
instead, and four of them (P2-1, P4-1, P5-1's behavior half, P6-1's
capability half) await an actual weak-agent run — they are the
pilot's open instruments, not loose ends.

## Phase ledger

| Phase | What landed | Exit |
|---|---|---|
| 0 — Adopt & shim | The Discipline became two installed vibevm packages (`flow:org.vibevm/discipline-core@0.2.0`, `stack:org.vibevm/rust-ai-native@0.2.0`) resolved from the in-repo `packages/` registry — vibevm installs the Discipline through its own tool, stack→flow transitively. Mechanisms relocated to `spec/discipline/` (URIs re-anchored, 177 edges / 0 suspects preserved); `spec/neworder/` is a shim; `vibevm.discipline.lock` pins the pilot; the prediction ledger opened. | met |
| 1 — Fast loop (E) | `cargo xtask fast-loop [--enforce-budget]` — the card's checker implemented; 18/18 cells within the 60s budget (100% vs the ≥90% prediction), worst cell ~23s. | met |
| 2 — Diagnostics (F) + doctests (G) | Every conform finding speaks `violates REQ <uri>: <why>; fix surface: <where>` (renderer + acceptor side by side, grammar-tested over a violating corpus). New rules `seam-has-doctest` + `error-enum-cites-req`; 30 undoctested seams found and closed; fingerprints hardened to line-shift-proof `context#ordinal`; frontend v2 (is_pub, has_doctest, ErrorVariant). | met |
| 3 — Typed builders (B) + contracts (C) | `CapabilityTag` types the activation seam (the silent-mismatch class became compile errors — observed live during migration); trybuild compile-fail pins it; roots-first and lockfile-uniqueness contracts witnessed at use sites; AUD-0014/0015 prose lies fixed; one false contract caught by the suite in seconds (the counter-lesson). | met |
| 4 — Differential oracles (D) | The property net (determinism / closure / roots-prefix / exact-pinning over generated acyclic worlds) + the `assert_solvers_agree` socket; rule `cell-has-oracle` (facts walk grew `tests/`); two previously-invisible unsafe frozen (6→8). | met |
| 5 — Generators (A) + simulators (H) | `fixpoint_model` — the runnable reference model of the conditional-dep loop with a stepping interface, a monotonicity witness at every step, and a model-vs-production-primitives conformance suite. Card A: the existing JTD→vibe-wire generator recognized as the complete instance; no artificial generator built (the card's misuse clause honored). | met |
| 6 — Codemods (I) | `cargo xtask codemod add-cell` — three files, atomic, rollback-on-red-post-check, `--spec-uri` required (A1 by construction). Its first live run demonstrated the rollback on a real template bug, then landed the Phase-7 SAT skeleton. | met (prototype-grade by design) |
| 7 — SAT + fixpoint | The `Sat` cell: chronological backtracking with the naive solver as branch checker (semantics cannot drift); dominance differential (the oracle found naive's first-pick trap in a generated world before any human enumerated one); DBT-0011 fixed (backtracking half). Composition predicates (`and`/`or`/`not`) ratified PROP-003 r1-planned → r2. | met |
| Sweep | Priority cells per plan §4: vibe-resolver carries the full card set (B, C, D, G, H + A recognized); the G/F gated set extended beyond the engine crates to the registry/workspace/check/publish seams (doctests + REQ edges authored per crate). | met |

## Exit criteria (plan §5)

1. **Mechanisms relocated; `spec/neworder/` a thin shim — done.**
   PROP-014 / BROWNFIELD / ENGINE-CONFORM / LEDGER-INTENT live at
   `spec/discipline/` (vibevm hosts what vibevm implements); the
   product lives in the installed packages; the shim is one README.
2. **Every priority cell carries its cards; checkers green — done.**
   The resolver (priority 1) carries B/C/D/G/H with implemented
   checkers (trybuild, contracts, the dominance differential,
   doctests, the conformance-tested model). The G/F conform rules
   gate the engine crates + the registry/workspace/check/publish
   layer; `cell-has-oracle` gates every `#[cell]` type. Full gate
   panel green at close (below).
3. **The prediction ledger is populated — done.** Twelve predictions
   (P0-1 … P7-1), each with a verdict or an explicit
   pending-by-design status tied to the deferred measurement.
4. **This REPORT — done.**

## Gate panel at adoption close (all green)

- `cargo xtask specmap --check` — clean; 0 suspects; the orphan
  ratchet 0 gated (6 DBT-0019 dispositions, 8 reasoned exemptions).
- `cargo xtask conform check` — 8 frozen / 0 new, with six rules
  active (R-001, R-002, unsafe-gate, seam-has-doctest,
  error-enum-cites-req, cell-has-oracle).
- `cargo xtask test-gate` — xfail-strict green (the suite grew by
  ~40 tests across the adoption: properties, conformance, doctests,
  sat units, codemod units).
- `cargo xtask fast-loop --enforce-budget` — every cell within 60s.
- `bash tools/self-check.sh` — fmt, workspace tests, clippy
  `-D warnings`, `vibe check` 0/0/0.

## What the adoption taught (the honest list — feeds Discipline v0.3)

1. **The discipline specifies checkers but not the gate-invocation
   pattern — and that gap bit three times.** A gate behind a pipe
   returns the pipe's exit code; a gate run from a stale cwd never
   runs; a panel run before the final commit certifies the wrong
   tree. One clippy-red commit even got pushed. The same
   cannot-silently-lie clause the engine is held to must extend to
   how callers consume verdicts: v0.3 should ship a
   **gate-invocation card** (capture the gate's own exit status;
   re-run the panel on the final tree of a series; never pipe a
   gate).
2. **The oracle out-thought its author twice, cheaply.** The strict
   naive≡sat differential was falsified by proptest within seconds
   (the generator made naive-trapping worlds the author believed
   impossible), and an input-side contract was killed by the
   existing suite just as fast. Both times the cost of a wrong
   belief was a red test in the loop, not an incident — the
   asymmetry cards C and D promise, observed from the failure side.
   v0.3 can cite this as the canonical "why runnable beats prose"
   anecdote with dates.
3. **Cards need an explicit no-tests / empty-case clause.** nextest
   exits 4 on a zero-test crate; the fast-loop card's "builds and
   tests in isolation" had no defined no-tests semantics, so four
   stub cells read RED until `--no-tests=pass` encoded "a zero-test
   cell's build IS its first signal". Every adopter will rediscover
   this unless the card says it.
4. **Class-A recognition needs a misuse counterweight, and it
   worked.** The plan named "transition tables, exhaustive matches"
   as generator candidates; honest survey found the activation
   channels differ in probe semantics — a generator there would
   encode business decisions. The card's Non-Goals clause is what
   licensed *declining* the application. Keep that clause strong;
   adopters under exit-criteria pressure will be tempted to build
   generators to tick boxes.
5. **Fingerprints must be coordinate-free from day one.** The
   line-keyed unsafe-gate fingerprints rotted on the first unrelated
   edit above a block (caught masked, in Phase 0). The
   `context#ordinal` shape survived the rest of the adoption without
   one stale entry. v0.3's conform section should mandate
   shift-stable fingerprints in the rule-authoring contract.
6. **The packaged Discipline leaves the spec-scanner's
   jurisdiction.** Once the product moved into `packages/`/
   `vibedeps/`, its documents stopped being mdspec units — cards
   cannot be `spec://`-addressed, only cited. The `discipline://`
   citation namespace (introduced for Class-F messages, recorded in
   `spec/discipline/README.md`) is the interim answer; the real one
   is the pending PROP-014 external-namespace amendment. v0.3 should
   decide whether cards are *units* (scannable, pinnable, suspects
   on revision) or *citations* (today's state).
7. **Self-hosting surfaced product gaps no review would have.**
   `vibe.lock` records machine-absolute `file:///` source_urls for
   local-registry installs (committed noise; debt candidate);
   `[[registry]]` accepts only git-cloneable URLs so a directory
   registry is flag-only (PROP-010 design input); the boot
   regeneration and `<vibevm>` splicing held perfectly. The pilot
   being the tool's own consumer is the cheapest dogfood loop the
   project has.
8. **The codemod's value showed up before any swarm existed.** The
   add-cell post-check caught its own template bug on first use and
   rolled back cleanly — the atomicity contract paid for itself
   immediately, author-side. The build/use-boundary question (can
   weak agents parameterize?) remains open (P6-1), but
   "codemods protect the STRONG author too" is a v0.3-worthy
   reframing of the card's Motivation.

## Standing state for the next effort

- **Open instruments (need a measured weak-agent run):** P2-1
  (iterations-to-green), P4-1 (the central C-7 transfer test — the
  with-oracle arm is fully built), P5-1's behavior half, P6-1's
  parameterization half. These are the Discipline's own v0.3
  evidence questions; the instrumentation ships with this repo.
- **Owner-gated:** publishing the two Discipline packages to the
  public `vibespecs` registry (token, outward-facing); resolvo
  adoption (the deviates edge on `Sat` keeps the seam open);
  production solver selection via the R-001 registry flag; the
  PROP-010 design session (now with the directory-registry input);
  `VIBEVM-SPEC.md` unit-ification (DBT-0019, unblocks vibe-cli
  item-grain); the PROP-014 external-namespace amendment (now with
  the `discipline://` precedent).
- **The debt registry** closed DBT-0011 (backtracking) and DBT-0016
  (the marker-homing dispute — its subject dissolved with the v0.2
  package); the rest of the open set is unchanged and tripwired.
- **The WAL** carries the running state; this REPORT is the
  adoption's frozen close-out.
