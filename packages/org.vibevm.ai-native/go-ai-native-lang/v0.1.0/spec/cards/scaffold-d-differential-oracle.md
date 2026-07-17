# CARD: scaffold-d-differential-oracle — Differential / Characterization Oracle (Go)
**Discipline v0.2 · BETA · T2 · Go**

*Reference instance of the AI-Native Pattern Card format, Go projection. Demonstrates
all three bands, especially the operational Band 3. This card is itself BETA (its
conform checker is specified; the pilot instance is the `research/go-demo` fuzz
differential).*

## Band 1 — Identity & Recognition

**Classification:** layer = E (Verification coupling); mechanism = scaffold class D.

**Intent:** When code is replaced or refactored, pin its observable behavior with a
runnable check that compares the new implementation against the old one (differential)
or against a captured baseline (characterization), so that a reader — especially a weak
one — can change code freely and receive a pass/fail signal on whether behavior moved.
Go's projection stands on **native fuzzing**: the language ships the input generator,
the corpus store, and the minimizer in the standard toolchain.

**Also Known As:** golden test; characterization test (Feathers); approval test;
back-to-back test; differential fuzzing; `FuzzXxx` target; `testdata/` corpus.

**Applicability / Recognition:** Apply when ANY of these signals are present —
- a cell is being *replaced* or its internals *rewritten* while its contract is meant to
  stay fixed (the replacement protocol, R-040, guide §10);
- legacy behavior exists that nobody fully understands but must be preserved;
- a refactor spans multiple files and the reader cannot prove by inspection that
  behavior is unchanged;
- a weak agent is assigned a modification task and needs a safety net it cannot derive.
*Detector seed:* a diff that modifies the body of an item carrying
`//spec:implements …` without a corresponding oracle artifact (a `Fuzz`/differential
test or a golden) in the cell's test files → recognition fires.

## Band 2 — Justification & Tradeoffs

**Motivation:** A weak agent is asked to optimize the naive planner cell. It rewrites
the diff loop. By inspection, neither the agent nor a fast reviewer can be sure the
change preserved behavior across edge states. With a differential fuzz target —
`FuzzPlannersAgree` feeding identical generated (desired, actual) state pairs to
`naiveplanner` and `batchplanner` and asserting equal action sets — the agent gets an
immediate mechanical verdict: behavior held, or here is a minimized counterexample the
fuzzer already shrank. The expensive cognition ("what are all the edge states?") was
materialized once as a harness plus a committed seed corpus; the weak agent consumes
the verdict.

**Structure & Participants:**
- *Subject-old* — the prior implementation (kept temporarily, or captured as goldens in
  `testdata/`).
- *Subject-new* — the replacement.
- *Input source* — a `FuzzXxx` target's generated inputs + the committed `testdata/`
  seed corpus (runs deterministically in plain `go test`; `-fuzz` explores locally).
- *Comparator* — the equality/equivalence predicate (deep-equal, or a documented
  divergence list).
- *Oracle harness* — the fuzz/differential test in the cell's `_test.go`, run with
  `-race`.

**Collaborations:** Pairs with Class B (defined types shrink the input space the oracle
must cover) and Class C (contracts define what "equivalent" means). Consumes Class E
(the per-package loop runs the seeds). Emits Class F diagnostics (a failure cites the
violated REQ + the minimized counterexample). In a raid, this card is the
*differential-safety* gate every behavior-changing card application must pass.

**Goals / Non-Goals:**
- *Goals:* detect unintended behavior change during replacement/refactor; give weak
  readers a modification safety net; make "behavior preserved" a machine fact.
- *Non-Goals:* NOT a correctness proof (new-vs-old agreement inherits the old bugs);
  NOT a substitute for the spec; NOT for greenfield code with no prior behavior;
  NOT open-ended CI fuzzing (CI runs the committed seeds — deterministic; exploration
  is a local/scheduled activity).

**Consequences:**
- (+) Aggressive refactoring becomes safe; drift is caught mechanically, with shrunk
  counterexamples for free (the toolchain minimizes).
- (+) "Change the implementation" and "preserve the contract" vary independently.
- (−) Authoring the input encoding for fuzz args (fuzzing takes primitive-typed args;
  structured states need a decode step) and the comparator costs effort.
- (−) Characterization goldens enshrine current behavior including bugs — pair with a
  spec edge marking intentional vs incidental; goldens must fail loudly, never
  auto-update (the `-update` flag never runs in CI).

**Alternatives:**
- *Formal proof:* Go has no mainstream Kani/Creusot analogue — the differential/property
  oracle carries proportionally more of the modification-safety load here (the same
  honest asymmetry the TS card records).
- *Manual review:* fails exactly where needed (large multi-file edits, weak readers).
- *Fresh unit tests:* test what the author thought to test; the differential oracle
  tests behavior the author never enumerated.

**Risks & Assumptions:**
- Assumes the old implementation is available or capturable.
- Assumes the fuzz encoding reaches representative states; a weak encoding gives false
  confidence — seed the corpus with the known hard cases.
- *Sunset:* if generation-time tooling plus contracts ever make behavior-preservation
  statically provable for a class of cells, the oracle retires for that class.
- Transfer risk: executable-scaffold value for *modification* is [E-mid]; this card is
  a prime pilot validation target on `research/go-demo`.

**Evidence & Transfer-strength:** R-040 (replacement protocol, production), R2C-008
(executable scaffolds transformative, benchmark), Feathers characterization
(production). Class: production + benchmark. Tag: **[E-mid]**.

## Band 3 — Operation

**Trigger:** WHEN a diff modifies the body of an item bearing `//spec:implements …`,
OR a cell is marked for replacement (`replaces=` in its manifest directive), OR a
refactor touches > 1 file in a cell whose contract is unchanged — THEN apply before
merge. **Mode:** gate.

**Routine** (≤7 steps, each verifiable):
1. Identify the behavioral surface to preserve (the seam's methods).
2. Keep `old` reachable (the replaced cell stays in the tree until the oracle is green),
   or capture goldens from it into `testdata/`.
3. Write/extend a `FuzzXxx` differential target decoding fuzz bytes into representative
   seam inputs; seed `testdata/` with the known hard cases.
4. Bind `old` vs `new` under the comparator (documented divergence list otherwise).
5. Run seeds in the per-package loop (`go test -race`); on a counterexample, fix `new`
   (NOT the oracle) until green; let `-fuzz` explore locally before landing.
6. Once green, remove `old` (or commit the goldens) and leave the oracle + corpus in
   place.
7. Tag the oracle `//spec:verifies <uri> r=<N>`.

**Checker:** conform rule `replacement-has-oracle` — flags a modified
`//spec:implements` item whose cell lacks a differential/characterization test
referencing it; backed by `go test ./<cell>/ -race` running the seeds. *(Status:
specified; pilot task.)*

**Raid role:** layer = *behavior-preserving* (wraps every behavior-changing card in any
raid); batch = per-cell.

**Budget:** gate-time, does not crowd the edit-time active set; first-signal = one
per-package seed run (target < 60s; corpus size tuned to stay in budget).
