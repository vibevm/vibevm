# CARD: scaffold-d-differential-oracle — Differential / Characterization Oracle (Go) {#root}

<status stage="spec" state="done"/>

##status-line **Discipline v0.2 · BETA · T2 · Go** @impl/done

##reference-instance-note *Reference instance of the AI-Native Pattern Card format, Go projection.* @impl/done

##demonstrates-all-three-bands *Demonstrates
all three bands, especially the operational Band 3.* @impl/done

##card-is-beta *This card is itself BETA (its
conform checker is specified; the pilot instance is the `research/go-demo` fuzz
differential).* @impl/done

## Band 1 — Identity & Recognition {#band-one-identity}

##CLASSIFICATION **Classification:** layer = E (Verification coupling); mechanism = scaffold class D. @impl/done

##INTENT **Intent:** When code is replaced or refactored, pin its observable behavior with a
runnable check that compares the new implementation against the old one (differential)
or against a captured baseline (characterization), so that a reader — especially a weak
one — can change code freely and receive a pass/fail signal on whether behavior moved.
Go's projection stands on **native fuzzing**: the language ships the input generator,
the corpus store, and the minimizer in the standard toolchain. @impl/done

##ALSO-KNOWN-AS **Also Known As:** golden test; characterization test (Feathers); approval test;
back-to-back test; differential fuzzing; `FuzzXxx` target; `testdata/` corpus. @spec/done

##applicability-recognition-lead **Applicability / Recognition:** Apply when ANY of these signals are present — @impl/done
- ##SIGNAL-CELL-IS-BEING-REPLACED a cell is being *replaced* or its internals *rewritten* while its contract is meant to
  stay fixed (the replacement protocol, R-040, guide §10); @impl/done
- ##SIGNAL-LEGACY-BEHAVIOR-IS-UNDERSTOOD-BY-NOBODY legacy behavior exists that nobody fully understands but must be preserved; @impl/done
- ##SIGNAL-REFACTOR-SPANS-MULTIPLE-FILES a refactor spans multiple files and the reader cannot prove by inspection that
  behavior is unchanged; @impl/done
- ##SIGNAL-WEAK-AGENT-NEEDS-A-SAFETY-NET a weak agent is assigned a modification task and needs a safety net it cannot derive. @impl/done

##DETECTOR-SEED *Detector seed:* a diff that modifies the body of an item carrying
`//spec:implements …` without a corresponding oracle artifact (a `Fuzz`/differential
test or a golden) in the cell's test files → recognition fires. @impl/done

## Band 2 — Justification & Tradeoffs {#band-two-justification}

##MOTIVATION **Motivation:** A weak agent is asked to optimize the naive planner cell. It rewrites
the diff loop. By inspection, neither the agent nor a fast reviewer can be sure the
change preserved behavior across edge states. With a differential fuzz target —
`FuzzPlannersAgree` feeding identical generated (desired, actual) state pairs to
`naiveplanner` and `batchplanner` and asserting equal action sets — the agent gets an
immediate mechanical verdict: behavior held, or here is a minimized counterexample the
fuzzer already shrank. The expensive cognition ("what are all the edge states?") was
materialized once as a harness plus a committed seed corpus; the weak agent consumes
the verdict. @spec/done

##structure-and-participants-lead **Structure & Participants:** @impl/done
- ##PARTICIPANT-SUBJECT-OLD *Subject-old* — the prior implementation (kept temporarily, or captured as goldens in
  `testdata/`). @impl/done
- ##PARTICIPANT-SUBJECT-NEW *Subject-new* — the replacement. @impl/done
- ##PARTICIPANT-INPUT-SOURCE *Input source* — a `FuzzXxx` target's generated inputs + the committed `testdata/`
  seed corpus (runs deterministically in plain `go test`; `-fuzz` explores locally). @impl/done
- ##PARTICIPANT-COMPARATOR *Comparator* — the equality/equivalence predicate (deep-equal, or a documented
  divergence list). @impl/done
- ##PARTICIPANT-ORACLE-HARNESS *Oracle harness* — the fuzz/differential test in the cell's `_test.go`, run with
  `-race`. @impl/done

##COLLABORATIONS **Collaborations:** Pairs with Class B (defined types shrink the input space the oracle
must cover) and Class C (contracts define what "equivalent" means). Consumes Class E
(the per-package loop runs the seeds). Emits Class F diagnostics (a failure cites the
violated REQ + the minimized counterexample). In a raid, this card is the
*differential-safety* gate every behavior-changing card application must pass. @impl/done

##goals-and-non-goals-lead **Goals / Non-Goals:** @impl/done
- ##GOALS *Goals:* detect unintended behavior change during replacement/refactor; give weak
  readers a modification safety net; make "behavior preserved" a machine fact. @impl/done
- ##NON-GOALS *Non-Goals:* NOT a correctness proof (new-vs-old agreement inherits the old bugs);
  NOT a substitute for the spec; NOT for greenfield code with no prior behavior;
  NOT open-ended CI fuzzing (CI runs the committed seeds — deterministic; exploration
  is a local/scheduled activity). @impl/done

##consequences-lead **Consequences:** @impl/done
- ##CONSEQUENCE-REFACTORING-BECOMES-SAFE (+) Aggressive refactoring becomes safe; drift is caught mechanically, with shrunk
  counterexamples for free (the toolchain minimizes). @spec/done
- ##CONSEQUENCE-IMPLEMENTATION-AND-CONTRACT-VARY-INDEPENDENTLY (+) "Change the implementation" and "preserve the contract" vary independently. @spec/done
- ##CONSEQUENCE-ENCODING-AND-COMPARATOR-COST-EFFORT (−) Authoring the input encoding for fuzz args (fuzzing takes primitive-typed args;
  structured states need a decode step) and the comparator costs effort. @spec/done
- ##CONSEQUENCE-GOLDENS-ENSHRINE-CURRENT-BEHAVIOR (−) Characterization goldens enshrine current behavior including bugs — pair with a
  spec edge marking intentional vs incidental; goldens must fail loudly, never
  auto-update (the `-update` flag never runs in CI). @spec/done

##alternatives-lead **Alternatives:** @impl/done
- ##ALTERNATIVE-FORMAL-PROOF *Formal proof:* Go has no mainstream Kani/Creusot analogue — the differential/property
  oracle carries proportionally more of the modification-safety load here (the same
  honest asymmetry the TS card records). @spec/done
- ##ALTERNATIVE-MANUAL-REVIEW *Manual review:* fails exactly where needed (large multi-file edits, weak readers). @spec/done
- ##ALTERNATIVE-FRESH-UNIT-TESTS *Fresh unit tests:* test what the author thought to test; the differential oracle
  tests behavior the author never enumerated. @spec/done

##risks-and-assumptions-lead **Risks & Assumptions:** @impl/done
- ##RISK-OLD-IMPLEMENTATION-IS-AVAILABLE Assumes the old implementation is available or capturable. @spec/done
- ##RISK-FUZZ-ENCODING-REACHES-REPRESENTATIVE-STATES Assumes the fuzz encoding reaches representative states; a weak encoding gives false
  confidence — seed the corpus with the known hard cases. @spec/done
- ##RISK-SUNSET *Sunset:* if generation-time tooling plus contracts ever make behavior-preservation
  statically provable for a class of cells, the oracle retires for that class. @spec/done
- ##RISK-TRANSFER Transfer risk: executable-scaffold value for *modification* is [E-mid]; this card is
  a prime pilot validation target on `research/go-demo`. @spec/done

##EVIDENCE-AND-TRANSFER-STRENGTH **Evidence & Transfer-strength:** R-040 (replacement protocol, production), R2C-008
(executable scaffolds transformative, benchmark), Feathers characterization
(production). Class: production + benchmark. Tag: **[E-mid]**. @spec/done

## Band 3 — Operation {#band-three-operation}

##TRIGGER **Trigger:** WHEN a diff modifies the body of an item bearing `//spec:implements …`,
OR a cell is marked for replacement (`replaces=` in its manifest directive), OR a
refactor touches > 1 file in a cell whose contract is unchanged — THEN apply before
merge. @impl/done

##MODE **Mode:** gate. @impl/done

##routine-lead **Routine** (≤7 steps, each verifiable): @impl/done
1. ##ROUTINE-IDENTIFY-THE-BEHAVIORAL-SURFACE Identify the behavioral surface to preserve (the seam's methods). @impl/done
2. ##ROUTINE-KEEP-OLD-REACHABLE Keep `old` reachable (the replaced cell stays in the tree until the oracle is green),
   or capture goldens from it into `testdata/`. @impl/done
3. ##ROUTINE-WRITE-THE-DIFFERENTIAL-TARGET Write/extend a `FuzzXxx` differential target decoding fuzz bytes into representative
   seam inputs; seed `testdata/` with the known hard cases. @impl/done
4. ##ROUTINE-BIND-OLD-VS-NEW Bind `old` vs `new` under the comparator (documented divergence list otherwise). @impl/done
5. ##ROUTINE-RUN-SEEDS-IN-THE-LOOP Run seeds in the per-package loop (`go test -race`); on a counterexample, fix `new`
   (NOT the oracle) until green; let `-fuzz` explore locally before landing. @impl/done
6. ##ROUTINE-REMOVE-OLD-ONCE-GREEN Once green, remove `old` (or commit the goldens) and leave the oracle + corpus in
   place. @impl/done
7. ##ROUTINE-TAG-THE-ORACLE Tag the oracle `//spec:verifies <uri> r=<N>`. @impl/done

##CHECKER **Checker:** conform rule `replacement-has-oracle` — flags a modified
`//spec:implements` item whose cell lacks a differential/characterization test
referencing it; backed by `go test ./<cell>/ -race` running the seeds. *(Status:
specified; pilot task.)* @spec/done

##RAID-ROLE **Raid role:** layer = *behavior-preserving* (wraps every behavior-changing card in any
raid); batch = per-cell. @impl/done

##BUDGET **Budget:** gate-time, does not crowd the edit-time active set; first-signal = one
per-package seed run (target < 60s; corpus size tuned to stay in budget). @impl/done
