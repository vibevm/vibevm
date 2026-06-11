# vibevm Terraform Plan v0.3 — Adopt Discipline v0.2 (AI-Native)
**status: PLAN · vibevm-specific · supersedes PLAYBOOK-TERRAFORM-VIBEVM-v0.2**

*This plan is NOT part of the Discipline. The Discipline (in `discipline-v0.2/`) is the product; vibevm is its pilot. This document tells vibevm how to adopt the new Discipline. Per the owner's constraint, the Discipline files are NOT modified for vibevm's sake — vibevm consumes them as-is. Where the Discipline must change, that happens in the Discipline product on its own evidence, not here.*

*Executed as a sequence of RAIDs (`discipline-v0.2/03-RAID-PLAYBOOK.md`), not one diff. Each phase is gated green before the next. Measurement is deferred by owner decision; each phase records falsifiable predictions instead.*

---

## 0. The core move: vibevm dogfoods the Discipline as a package

vibevm is a package manager for cognitive artifacts. The cleanest adoption is **self-hosting**: the Discipline becomes a vibevm-installed package (`flow:discipline-core` + `stack:rust-ai-native`), and vibevm consumes it the way any project would. This closes the loop (the Discipline's first carrier installs the Discipline through the Discipline's own tool) and is the strongest possible pilot.

Consequence for `spec/neworder/`: files that were **generic discipline** content move OUT of vibevm-specific space (they become the Discipline product); files that are **vibevm-specific** stay. The mapping below makes this explicit.

## 1. File mapping: old `spec/neworder/` → new

| Old vibevm file | Disposition | New location / replacement |
|---|---|---|
| `DISCIPLINE-CHARTER-v0.1.md` | **superseded** | `discipline-v0.2/00-MANIFESTO.md` (the new charter; axioms retained, projected to language level) |
| `GUIDE-RUST-v0.1.md` | **superseded** | `discipline-v0.2/rust/GUIDE-AI-NATIVE-RUST.md` (absorbs all prior rules) |
| `GUIDE-SPEC-AUTHORING-v0.1.md` | **superseded + extended** | `discipline-v0.2/01-PATTERN-CARD-FORMAT.md` (spec authoring = card authoring now) |
| `PROP-014-specmap-bidirectional-traceability.md` | **retained as-is, relocated** | becomes part of the Discipline meta-layer (referenced by Guide §7; mechanism unchanged — it is already AI-native) |
| `BROWNFIELD-PROTOCOL-v0.1.md` | **retained, relocated** | Discipline mechanism (debt/intent/contradiction as first-class objects; cited by A6 and the Raid Playbook) |
| `ENGINE-CONFORM-v0.1.md` | **retained, relocated + extended** | Discipline checker infrastructure; extended to run card checkers (the conform tiers named in card Band-3 `checker` fields) |
| `LEDGER-INTENT-v0.1.md` | **retained, relocated** | Discipline mechanism (epoch-keyed interpretation cache; provenance lines) |
| `PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md` | **superseded** | `discipline-v0.2/03-RAID-PLAYBOOK.md` (generalized) + THIS plan (vibevm-specific instance) |
| `README.md` | **rewritten** | points to the installed Discipline package + this plan |

**What stays vibevm-specific** (does NOT move into the Discipline): this terraform plan; `conform-baseline.json`; vibevm's own spec modules (`spec/modules/vibe-resolver/PROP-003-*` etc.); vibevm's per-cell cards for its own code; the vibevm `ROADMAP`/`WAL`.

**Net:** `spec/neworder/` shrinks to a thin shim that (a) declares the installed Discipline package and version, and (b) links this plan. The discipline content it used to hold now lives in the product and is consumed.

## 2. New vibevm-specific artifacts this adoption creates
- **`vibevm.discipline.lock`** — pins the Discipline package version vibevm is piloting (so the pilot is reproducible).
- **vibevm cell cards** — one card per significant vibevm cell, authored in the new format as the codebase is swept (priority order in §4).
- **A pilot prediction ledger** — the falsifiable `prediction` from each adopted card, recorded for later cheap verification (measurement deferred).
- **`vibe-tcg` roadmap entry** — the tool spec (`discipline-v0.2/rust/tools/vibe-tcg.md`) enters vibevm's tool roadmap (Stage 1 first: syntactic profile).

## 3. Phased raids

Each phase is a raid: scope+freeze → card-set+order → per-layer phases → batch+checkpoints → differential safety → exit REPORT.

**Phase 0 — Adopt & shim (no code change).**
Install the Discipline as a vibevm package; rewrite `spec/neworder/README` to the shim; relocate the retained mechanisms (PROP-014, brownfield, conform, intent-ledger) under the Discipline; pin `vibevm.discipline.lock`. Exit: vibevm builds; specmap index regenerates deterministically; 0 orphans (the existing terraform already achieved 177 edges / 0 suspects — preserve that).
*Prediction:* relocation is behavior-neutral; conform-baseline unchanged.

**Phase 1 — Substrate: the fast loop (Class E) everywhere.**
Make every vibevm cell independently buildable + testable < ~60s (`scaffold-e-fast-loop`). This is the precondition for all other scaffolds. Batch per cell; checkpoint = cell builds+tests in isolation.
*Prediction:* per-cell first-signal latency < 60s for ≥90% of cells; cells failing this reveal hidden coupling (debt logged).

**Phase 2 — Cheapest runnable capital: diagnostics (F) + doctests (G).**
Add `spec://`-citing structured diagnostics to conform and `thiserror` (Class F); add one compiled doctest of canonical use per public seam (Class G). Inline + gate triggers.
*Prediction:* iterations-to-green on a sample modification task drop vs Phase 1 baseline (recorded, not yet measured).

**Phase 3 — Seam hardening: typed builders (B) + runnable contracts (C).**
Newtype/typestate the resolver and lockfile seams (Class B); witness cross-cell invariants at use sites (Class C), especially in `vibe-resolver` activation/fixpoint code. Gate + inline.
*Prediction:* a class of previously-runtime config/protocol errors becomes compile-time; `cargo check` catches them.

**Phase 4 — Modification safety net: differential oracles (D).**
Wrap the algorithmic core (`vibe-resolver`) replacements in proptest differential oracles; characterize opaque legacy behavior with goldens (Class D). Gate.
*Prediction (the central pilot test):* a Qwen-32B-class agent modifying a resolver cell WITH the oracle + scaffolds achieves higher behavior-preservation than WITHOUT — the generation→modification transfer question (C-7 in the contradiction map).

**Phase 5 — High-ceiling scaffolds: generators (A) + simulators (H) for the resolver.**
Generate derivable resolver artifacts (transition tables, exhaustive matches) from spec (Class A); ship a runnable reference model of the conditional-dependency fixpoint the swarm can step through (Class H). Raid + gate.
*Prediction:* edit surface for resolver changes collapses to the spec; weak agents predict fixpoint behavior via the simulator instead of mental simulation.

**Phase 6 — Codemods (I), pilot-gated.**
Prototype codemods for the recurring multi-file changes (add-cell, register-variant, rename-seam) ([E-hyp]). Expose fixed-parameter invocations to the weakest tier ONLY; measure whether agents can parameterize them (the build/use boundary). Raid.
*Prediction:* fixed-parameter codemods lift the weakest tier on multi-file edits; free parameterization may not — this phase tests exactly that, and its REPORT decides card `scaffold-i-codemods`'s graduation from WISH.

**Phase 7 — Algorithmic-core debt (vibevm-specific, parallel track).**
Independent of scaffolding: resolve the SAT-solver debt (DBT-0011: resolvo primary per spec; tree has only `NaiveDepSolver` without backtracking) and formalize the conditional-dependency + virtual-capability fixpoint (PROP-003 §2.5.3/§2.6, `composition` predicates `and/or/not` currently `planned`). The fixpoint's monotone-lattice structure (LLM emits virtual capabilities AFTER the static solve, entering via monotone activation) is the place where AI-meets-classical lives in vibevm. Author it under the new discipline (cards: B for the solver's typed states, C for fixpoint invariants, D for solver-vs-naive differential, H for the fixpoint simulator).
*Prediction:* a backtracking solver + formalized fixpoint passes the differential oracle against `NaiveDepSolver` on all current resolver tests.

## 4. Cell sweep priority
1. `vibe-resolver` — the algorithmic core and the AI×classical locus; highest value, gets the full scaffold set (A,B,C,D,H).
2. lockfile / registry cells — seams that benefit most from Class B typing.
3. MCP-server / CLI edges — Class F diagnostics + Class G doctests first.
4. Remaining cells — swept by the standard per-layer raid.

## 5. Exit criteria for the whole adoption
- All retained mechanisms relocated under the Discipline; `spec/neworder/` is a thin shim.
- Every priority cell carries its cards; checkers green across scope.
- The pilot prediction ledger is populated (every adopted card's `prediction` recorded).
- A closing **terraform REPORT** (modeled on the prior one) listing what the adoption taught — including cards that misfired, routines that overloaded weak readers, and any Discipline document that the pilot suggests should change. **That REPORT is the input to the next iteration of the Discipline** (we return to this when results are in).

## 6. What this plan deliberately does NOT do
- Does NOT modify Discipline files to fit vibevm (the product stays canonical).
- Does NOT build measurement infrastructure (deferred; predictions recorded instead).
- Does NOT introduce a custom compiler for production code (only `vibe-tcg` as a generation-time tool, staged, standing on rust-analyzer).
- Does NOT change the surface language — AI-Native Rust stays idiomatic Rust (the central law).
