# CARD: scaffold-d-differential-oracle — Differential / Characterization Oracle (TypeScript) {#root}

<status stage="spec" state="done"/>

##status-line **Discipline v0.2 · BETA · T2 · TypeScript** @impl/done

##reference-instance-note *Reference instance of the AI-Native Pattern Card format, TypeScript projection.* @impl/done

##demonstrates-all-three-bands *Demonstrates all three bands, especially the operational Band 3.* @impl/done

##card-is-beta *This card is itself BETA (its checker is specified but not yet implemented — there is no TypeScript pilot codebase yet).* @impl/done

## Band 1 — Identity & Recognition {#band-one-identity}

##CLASSIFICATION **Classification:** layer = E (Verification coupling); mechanism = scaffold class D. @impl/done

##INTENT **Intent:** When code is replaced or refactored, pin its observable behavior with a runnable check that compares the new implementation against the old one (differential) or against a captured baseline (characterization), so that a reader — especially a weak one — can change code freely and receive a pass/fail signal on whether behavior moved. @impl/done

##ALSO-KNOWN-AS **Also Known As:** golden test; snapshot test; characterization test (Feathers); approval test; back-to-back test; differential testing; `fast-check` model-based / property-based test. @spec/done

##applicability-recognition-lead **Applicability / Recognition:** Apply when ANY of these signals are present — @impl/done
- ##SIGNAL-CELL-IS-BEING-REPLACED a cell is being *replaced* or its internals *rewritten* while its contract is meant to stay fixed (the replacement protocol, R-040, guide §11); @impl/done
- ##SIGNAL-LEGACY-BEHAVIOR-IS-UNDERSTOOD-BY-NOBODY legacy behavior exists that nobody fully understands but must be preserved (no spec, only observed behavior); @impl/done
- ##SIGNAL-REFACTOR-SPANS-MULTIPLE-FILES a refactor spans multiple files and the reader cannot prove by inspection that behavior is unchanged; @impl/done
- ##SIGNAL-WEAK-AGENT-NEEDS-A-SAFETY-NET a weak agent is assigned a modification task and needs a safety net it cannot derive itself. @impl/done

##DETECTOR-SEED *Detector seed:* a diff that modifies the body of an item carrying `@implements spec://…` (or its sidecar edge) without a corresponding oracle artifact in the cell's test module → recognition fires. @impl/done

## Band 2 — Justification & Tradeoffs {#band-two-justification}

##MOTIVATION **Motivation:** A Qwen-32B-class agent is asked to optimize a parser cell authored by Opus. It rewrites the hot loop. By inspection, neither the agent nor a fast human reviewer can be sure the 200-line change preserved behavior across edge cases. With a differential oracle — `fast-check` feeding identical generated inputs to `oldParse` and `newParse` and asserting equal outputs — the agent gets an immediate, mechanical verdict: behavior held, or here is a minimized (shrunk) counterexample. The expensive cognition ("what are all the edge cases?") was materialized once, by the author, as a runnable harness; the weak agent consumes the verdict instead of re-deriving the edge-case analysis. @spec/done

##structure-and-participants-lead **Structure & Participants:** @impl/done
- ##PARTICIPANT-SUBJECT-OLD *Subject-old* — the prior implementation (kept temporarily as `oldParse`, or captured as `vitest` snapshots). @impl/done
- ##PARTICIPANT-SUBJECT-NEW *Subject-new* — the replacement. @impl/done
- ##PARTICIPANT-INPUT-SOURCE *Input source* — a `fast-check` arbitrary, a recorded production-input set, or a snapshot corpus. @impl/done
- ##PARTICIPANT-COMPARATOR *Comparator* — the equality/equivalence predicate (deep-equal, or domain-specific tolerance). @impl/done
- ##PARTICIPANT-ORACLE-HARNESS *Oracle harness* — the runnable `vitest`/`fast-check` test binding these, living in the cell's test module. @impl/done

##COLLABORATIONS **Collaborations:** Pairs with Class B (branded/typed surfaces shrink the input space the oracle must cover) and Class C (contracts define what "equivalent" means). Consumes Class E (the per-cell fast loop runs the oracle). Emits Class F diagnostics (a failure cites the violated REQ + the minimized counterexample). In a raid (§3 of the format), this card is the *differential-safety* gate that every behavior-changing card application must pass. @impl/done

##goals-and-non-goals-lead **Goals / Non-Goals:** @impl/done
- ##GOALS *Goals:* detect unintended behavior change during replacement/refactor; give weak readers a modification safety net; make "behavior preserved" a machine fact, not a claim. @impl/done
- ##NON-GOALS *Non-Goals:* NOT a correctness proof (it checks new-vs-old agreement, so it inherits any bug the old code had); NOT a substitute for the spec (it pins behavior, it does not justify it); NOT for greenfield code with no prior behavior to differ against. @impl/done

##consequences-lead **Consequences:** @impl/done
- ##CONSEQUENCE-REFACTORING-BECOMES-SAFE (+) The reader can refactor aggressively; the net catches behavior drift mechanically. @spec/done
- ##CONSEQUENCE-IMPLEMENTATION-AND-CONTRACT-VARY-INDEPENDENTLY (+) Decouples "change the implementation" from "preserve the contract" — they vary independently. @spec/done
- ##CONSEQUENCE-ARBITRARY-AND-COMPARATOR-COST-EFFORT (−) Cost: authoring the `fast-check` arbitrary and comparator; maintaining snapshots (which can rot — they must fail loudly when stale, run under `--ci`, never `--update` auto-rewriting silently). @spec/done
- ##CONSEQUENCE-CHARACTERIZATION-ENSHRINES-CURRENT-BEHAVIOR (−) Characterization variant *enshrines current behavior including its bugs* — must be paired with a spec edge that says which behaviors are intentional vs incidental. @spec/done

##alternatives-lead **Alternatives:** @impl/done
- ##ALTERNATIVE-FORMAL-PROOF *Full formal proof:* in Rust this is the Kani/Creusot option; **TypeScript has no comparable mainstream formal-verification tool**, so the differential/property oracle carries proportionally more of the modification-safety load here — a genuine TS-vs-Rust asymmetry, not a gap in this card. @spec/done
- ##ALTERNATIVE-MANUAL-REVIEW *Manual review:* the status quo; fails exactly where we need it (large multi-file edits, weak readers). @spec/done
- ##ALTERNATIVE-FRESH-UNIT-TESTS *Unit tests written fresh:* test what the author thought to test; the differential oracle tests behavior the author never enumerated. Prefer differential when preserving opaque legacy behavior. @spec/done

##risks-and-assumptions-lead **Risks & Assumptions:** @impl/done
- ##RISK-OLD-IMPLEMENTATION-IS-AVAILABLE Assumes the old implementation is available or its behavior is capturable (as a snapshot). @spec/done
- ##RISK-INPUTS-ARE-GENERATABLE-WITH-COVERAGE Assumes inputs are *generatable* with enough coverage; a weak `fast-check` arbitrary gives false confidence. @spec/done
- ##RISK-SUNSET *Sunset condition:* if generation-time tools (`typescript-ai-native-tcg`, the stack's shipped type oracle) plus full contracts ever make behavior-preservation statically provable for a class of cells, the differential oracle becomes redundant for that class and retires there. @spec/done
- ##RISK-TRANSFER Transfer risk: the value of executable scaffolds for *modification* (vs generation) is [E-mid], not yet measured on a TypeScript codebase — this card is a prime pilot validation target on the forthcoming VibeVM TypeScript surface. @spec/done

##EVIDENCE-AND-TRANSFER-STRENGTH **Evidence & Transfer-strength:** findings R-040 (replacement protocol, production), R2C-008 (executable scaffolds transformative for weak agents, benchmark), Feathers characterization method (production). Evidence class: production + benchmark. Transfer tag: **[E-mid]** (executable-scaffold value shown for generation; modification transfer to be validated in the TypeScript pilot). @spec/done

## Band 3 — Operation {#band-three-operation}

##TRIGGER **Trigger:** WHEN a diff modifies the body of an item bearing `@implements spec://…`, OR a cell is marked for replacement, OR a refactor touches > 1 file in a cell whose contract is unchanged — THEN apply this card before merge. @impl/done

##MODE **Mode:** gate (runs at the cell's verification gate, not per keystroke). @impl/done

##routine-lead **Routine** (≤7 steps, each verifiable): @impl/done
1. ##ROUTINE-IDENTIFY-THE-BEHAVIORAL-SURFACE Identify the behavioral surface to preserve (the seam's public functions). @impl/done
2. ##ROUTINE-KEEP-OLD-REACHABLE Keep `old` reachable (rename to `oldParse`, or capture `vitest` snapshots from it on a fixed input set). @impl/done
3. ##ROUTINE-WRITE-THE-ARBITRARY Write/extend a `fast-check` arbitrary generating representative inputs for that surface. @impl/done
4. ##ROUTINE-BIND-OLD-VS-NEW Bind `old` vs `new` (or `snapshot` vs `new`) under an equality/equivalence comparator. @impl/done
5. ##ROUTINE-RUN-IN-THE-LOOP Run under the per-cell loop; on a shrunk counterexample, fix `new` (NOT the oracle) until green. @impl/done
6. ##ROUTINE-REMOVE-OLD-ONCE-GREEN Once green, remove `old` (or commit the snapshots) and leave the oracle in the test module. @impl/done
7. ##ROUTINE-CITE-THE-ORACLE Cite the oracle from the replacement's `@verifies spec://…` edge. @impl/done

##CHECKER **Checker:** `@typescript-eslint`/conform rule `replacement-has-oracle` — flags any modification of an `@implements`-bearing item body whose cell lacks a differential/characterization test referencing it. Backed by `vitest run <cell>` running the `fast-check`/snapshot oracle. *(Status: specified, NOT yet implemented → this card is BETA.)* @spec/done

##RAID-ROLE **Raid role:** layer = *behavior-preserving* phase (runs in any raid that rewrites implementations); order = applied AS A GATE around every other behavior-changing card (no ordering dependency of its own, but nothing that changes behavior may merge in a raid without it); batch = per-cell. @impl/done

##BUDGET **Budget:** competes with few rules (it is gate-time, not inline, so it does not crowd the edit-time active set); first-signal latency = one per-cell `fast-check` run (target < 60s; tune the run count / `numRuns` to stay in budget). @impl/done
