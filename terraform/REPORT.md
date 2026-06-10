# terraform/REPORT.md — the Discipline terraform, close-out report

_2026-06-10, branch `new`. The beta-exit deliverable of
[`PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md`](../spec/neworder/PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md)
Phase 6. Metrics are measured against
[`BASELINE.md`](BASELINE.md) (Phase −1, same day — the whole terraform
ran in one continuous effort). The honest-mistakes list at the end is
the §6 requirement: what the discipline got wrong feeds v0.2 of every
package document._

## Phase ledger

| Phase | Status | Evidence |
|---|---|---|
| −1 inventory | done | BASELINE.md; debt 18→19 entries; intent 31; conflict scan 5 disputes (4 adjudicated same day); golden 5 flows |
| 0 tooling skeleton | done | specmark-grammar / specmark / specmap-core crates; `specmap [--check]` / `test-gate` / `tripwire`; first committed index (408 units) |
| 1 pilot + drill | done | PROP-003 §2.6.1 × conditional.rs; drill commits `b3a947c`/`73b6e81`/`4afe716` live in history; PHASE1-PILOT.md |
| 2 backfill vibe-resolver | done | 54 proposals (all owner-APPROVED), 6 affirmation commits, PRP-0054 ratchet catch, orphan ratchet flipped blocking |
| 3 cells v0 | done | `#[cell]` manifests on the DepSolver/DepProvider cells; selection registry (R-001) in vibe-cli; hermetic differential oracle over real file:// git repos; conform-lite |
| 4 conform MVP | done | conform-core + conform-frontend-rust; 3 rules; SARIF; baseline (6 frozen unsafe); 1-file-diff = 1 re-extract proven; conform-lite retired |
| 5 ledger MVP | done | `.ledger/` interpretations store; `trace explain --prose` epoch-keyed cache + provenance line; telemetry; facts proven epoch-immune |
| 6 expansion + reconciliation + report | done | ratchet 15→8 exemptions (each with reason); intent unaccounted = 0; instrumented category-C audit; this report |

## Metrics vs BASELINE

| Metric | Phase −1 | Close-out |
|---|---|---|
| Tests (xfail-strict) | 998 passed / 3 skipped | **1075 passed / 3 skipped** (+77: specmark grammar+cell, ratchet, oracle, conform, ledger) |
| Tests-baseline entries | 3 (live trio) | 3 — **zero shrinkage, zero growth**: no test was quarantined or promoted during the whole terraform |
| Spec units | 0 indexed (no index existed) | **489** (incl. 6 owner language-guide drops mid-session: Go, 4× Java, Kotlin) |
| Tagged code items / edges | 0 / 0 | **170 / 177** (79 item-grain incl. the pilot's 19; 98 module-grain scope markers) |
| Suspects / dangling edges | n/a | **0 / 0** |
| Orphan gate | n/a | **0 gated orphans** over 10 gated crates; 6 dispositioned (DBT-0019); 8 exempt with reasons |
| Conform findings | n/a | 6 workspace-wide (all unsafe-gate, frozen); **0 in scope crates/vibe-resolver; 0 new** |
| Debt registry | 18 entries (1 P1 / 7 P2 / 10 P3) | **19** (+DBT-0019); P1 unchanged (AUDIT P1 partially addressed: hermetic git harness brick landed via the oracle) |
| Intent registry | 31 open | **0 unaccounted**: 3 done / 27 rescoped / 1 rejected |
| Disputes | 5 found, 4 adjudicated | 1 open by design (DBT-0016 — feeds package v0.2) |
| Golden characterization | 5 flows, byte-deterministic | **byte-identical** after every phase (re-captured after Phase 3's registry-factory refactor and Phase 6's sweep) |
| Wish-ratio of activated rules | n/a | 3 conform rules shipped, 3 enforced (R-001, R-002, unsafe-gate) — wish-ratio 0 |
| Ledger hit rate | n/a | live smoke 1 hit / 1 miss; counters in `.ledger/telemetry.json`; rot-rate plumbing in place, no data yet |
| LLM-$ per merged change | n/a | not instrumented — no `vibe-llm` runtime exists; the ledger's cost field is plumbed and zero-valued for the template producer |

## What the discipline got wrong (feeds v0.2)

1. **The playbook's Phase 2 sweep prompt says "every public item" but
   its own exclusion shorthand bites:** conditional.rs was skipped as
   "already tagged" and its public `PredicateError` went untagged —
   only the freshly-flipped ratchet caught it (PRP-0054). v0.2 should
   say: exclusions apply to *items*, never to *files*.
2. **`pin_preferences` lives in PROP-003 §2.1 as a trait method that
   implementation reality routed around** (PROP-011 Phase 3 holds pins
   via constraint-tightening). The deviates edge on `DepSolver` records
   it now, but the PROP text still reads as if the method is imminent —
   a §2.1 revision is owed when SatDepSolver lands.
3. **The orphan-gate grain had to be invented mid-flight.** PROP-014
   names orphans as an index table; the playbook flips a gate; neither
   says items-vs-modules or how dispositions work. The v0 answer
   (top-level pub items; scope! inheritance; per-symbol debt
   dispositions in the ratchet file) worked, but it is engine
   convention, not ratified spec — PROP-014 unit-ification should
   absorb it.
4. **`if_os` is specified as "inert until the activation engine is
   built" — the engine got built and the sentence silently became a
   gap.** The deviates edge on `evaluate` keeps it visible (PRP-0042),
   but spec text that self-expires on an event needs the event to flip
   a status somewhere; lifecycle `planned` would have caught it.
5. **Two doc-string lies survived every prior gate** (cycles
   "rejected" in features.rs; the §2.3-vs-§2.2 citation on
   ResolvedNode) — found only by the human-grade reading the sweep
   forces. Recorded as AUD-0014/0015; cheap fixes, deliberately not
   smuggled into affirmation commits.
6. **The fact-store / specmap split duplicates the module-path
   scheme** (conform-core re-implements rscan's module mapping). Both
   documents claim the same "facts" concept; the engines should share
   one walker when conform grows specmap-aware rules.
7. **The discipline package's own documents are not unit-disciplined
   yet** (PROP-014 unmarked, six pin-into-unmarked warnings stand by
   design). The terraform tagged production code against guide-grade
   anchors (ENGINE-CONFORM, BROWNFIELD, LEDGER) — useful, but
   ratification debt for the package itself.
8. **The CI bullets repeat in three phases against a repo whose owner
   decision is no-CI.** v0.2 should parameterise the carrier ("gate
   command runs in CI *where CI exists*; locally otherwise") instead
   of re-deferring per phase.

## Standing state for the next effort

- Gates, in run order: `cargo xtask specmap --check` (index + orphan
  ratchet) → `cargo xtask conform check` → `cargo xtask test-gate` →
  `bash tools/self-check.sh`. All green at close-out.
- The proposals→affirmation protocol (PROP-014 §2.7) is proven
  end-to-end and is the template for every future crate's item-grain
  backfill (vibe-cli first, once VIBEVM-SPEC.md becomes scannable —
  DBT-0019's resolution path).
- Owner inputs that remain open: the PROP-010 design session
  (INT-0003), the SAT solver (DBT-0011), the next full PROP-013 audit
  window (INT-0001), VIBEVM-SPEC unit-ification (DBT-0019), the
  package-v0.2 revision fed by this report.
