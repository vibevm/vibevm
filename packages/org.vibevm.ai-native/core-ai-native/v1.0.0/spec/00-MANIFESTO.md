# The AI-Native Code Discipline — Manifesto {#root}

<status stage="spec" state="done"/>

@fact:status-line **Discipline v0.2 · status: BETA · supersedes DISCIPLINE-CHARTER-v0.1** @status:impl/done

@fact:root-document *This is the root document of the Discipline.* @status:impl/done

@fact:DISCIPLINE-IS-A-PRODUCT *The Discipline is a product in its own right: a language-independent set of principles, plus per-language projections, for writing code that is optimal for COMPREHENSION and SAFE MODIFICATION by AI agents — explicitly including weak readers.* @status:impl/done

@fact:VIBEVM-IS-FIRST-PILOT *vibevm is the first pilot of this product, not its scope boundary.* @status:impl/done

@fact:MATURITY-MARKED-EVERYWHERE *Maturity is marked everywhere.* @status:impl/done

@fact:MATURITY-CLAIM-CLASSES *Some claims are benchmark-backed [E-strong], some are supported by adjacent evidence [E-mid], some are first-principles awaiting validation [E-hyp].* @status:impl/done

@fact:FALSIFIABLE-BETA *This document is a falsifiable beta, revised on pilot evidence only.* @status:impl/done

---

## 0. What this is, in one paragraph {#what-this-is}

@fact:generation-is-already-good Code generation by frontier models is already good. @status:spec/done

@fact:unsolved-problem-is-the-lifecycle The unsolved problem is the rest of the lifecycle: a model — often a *different, weaker* model than the author — must later read, understand, and safely change that code. @status:spec/done

@fact:discipline-optimizes-for-that The Discipline optimizes for that. @status:impl/done

@fact:SINGLE-DESIGN-TARGET Its single design target is to **lower the minimum model capability required to maintain code safely**, so that a swarm of small agents can maintain code that a frontier model authored. @status:impl/done

## 1. The target function: author/reader asymmetry {#target-function}

@fact:economics-that-justify The economics that justify everything below: a strong author runs **once**; weak readers run **thousands of times**. @status:spec/done

@fact:MOVES-COST-TO-AUTHORING-TIME So the discipline moves cost from maintenance-time to authoring-time. @status:impl/done

@fact:AUTHOR-MATERIALIZES-CAPITAL The strong author materializes expensive cognition into infrastructure — meta-layer, contracts, executable scaffolds, recorded rationale — and the weak swarm lives off that capital. @status:impl/done

@fact:CAPABILITY-GAP-COMPENSATOR **The Discipline is a capability-gap compensator.** @status:impl/done

@fact:CLEVER-CONSTRUCT-IS-THEFT A clever construct with no materialized rationale is theft from the reader pool. @status:impl/done

@fact:simpler-is-the-wrong-frame This is why "make the code simpler" is the wrong frame. @status:spec/done

@fact:LOWERING-THE-FLOOR-NOT-THE-CEILING We are not lowering the ceiling of what the code does; we are lowering the floor of who can safely touch it. @status:impl/done

## 2. The six axioms, projected to the language level {#six-axioms}

@fact:axioms-lead The axioms are unchanged from the Charter; here is what each *means for how code and its metadata are written*. @status:impl/done

- @fact:AXIOM-A1-EXPLAINABILITY **A1 — Explainability.** Every artifact carries a machine-resolvable chain from code to requirement to rationale (`spec://` URIs, in-source `#[spec(...)]` edges). Unexplainable code is unmergeable. *Language-level:* identifiers, errors, and items are anchored to requirements, not free-floating. @status:impl/done
- @fact:AXIOM-A2-NEVER-PAY-TWICE **A2 — Never pay twice.** Derived cognition is materialized content-addressed and dependency-tracked. *Language-level:* anything mechanically derivable (boilerplate, indexes, glue) is generated, not hand-maintained; the strong author's analysis is stored, not re-run. @status:impl/done
- @fact:AXIOM-A3-ALGORITHMIC-FLOOR **A3 — Algorithmic floor.** Where a deterministic procedure exists, the LLM is forbidden; its outputs sink below the floor. *Language-level:* push as much intent as possible into machine-checkable form — types, exhaustiveness, contracts — so a checker, not a model, enforces it. **This is the engine of the central law (§3).** @status:impl/done
- @fact:AXIOM-A4-HUMAN-ACCOUNTABILITY **A4 — Human accountability.** AI checks and proposes; the human is the accountable author; diffs stay human-reviewable. *Language-level:* no change is so clever a human cannot review it; determinism keeps diffs semantic. @status:impl/done
- @fact:AXIOM-A5-RULE-IS-CODE **A5 — Rule = code.** Every rule ships a checker or is explicitly a WISH. *Language-level:* a style rule with no linter is not a rule; it is documentation that decays (see §6). Rulebook health is the wish-ratio, not the page count. @status:impl/done
- @fact:AXIOM-A6-REALITY-BEFORE-ASPIRATION **A6 — Reality before aspiration.** Gates measure deltas against inventoried reality; debt, unimplemented intent, and contradiction are first-class tracked objects. *Language-level:* the code's actual state — not its intended state — is what tooling reasons over. @status:impl/done

## 3. The central law: idiomatic inside the file, engineered around the file {#the-central-law}

@fact:strongest-empirical-result The strongest empirical result we found, dated and controlled: models collapse on out-of-distribution *surface syntax* (EsoLang-Bench, Mar 2026: frontier models 0–11% on esoteric languages they could solve trivially in Python) — **and** current agents largely *recover* that collapse through tools and in-session strategy (the Jun 2026 follow-up: the same tasks, 87–100% with file-editing + interpreter loops). @status:spec/done

@fact:in-context-learning-could-not-teach In-context learning could **not** teach the unfamiliar surface (few-shot gave ~0 benefit); the recovery came from the verification loop and from building executable helpers, not from prose. @status:spec/done

@fact:law-that-follows-lead The law that follows: @status:impl/done

> @fact:CENTRAL-LAW **Keep the code surface inside the training distribution. Put all the strictness into the meta-layer, the type system, and the verification loop — which sit AROUND the code, not in its syntax.** @status:impl/done

@fact:RUST-LOOKS-ORDINARY Concretely: AI-Native Rust looks like *ordinary idiomatic Rust* at the token level (no invented notation, no exotic dialect — that would inherit the OOD penalty). @status:impl/done

@fact:unusual-is-everything-around-it-lead What is unusual is everything around it: @status:impl/done

- @fact:ENVELOPE-DENSE-METADATA dense machine-checkable metadata, @status:impl/done
- @fact:ENVELOPE-CONTRACT-BEARING-TYPES contract-bearing types, @status:impl/done
- @fact:ENVELOPE-EXECUTABLE-SCAFFOLDS executable scaffolds, @status:impl/done
- @fact:ENVELOPE-VERIFICATION-LOOP and a fast per-unit verification loop. @status:impl/done

@fact:NOT-STRANGER-BUT-STRICTER We do not make the language stranger; we make its envelope stricter. @status:impl/done

## 4. Stricter, not simpler — and where the strictness lives {#stricter-not-simpler}

@fact:MORE-DISCIPLINE-NOT-LESS "AI-native" is **more** discipline, not less — but the added strictness lives exclusively in machine-checkable form. @status:impl/done

@fact:MORE-TYPES-NEVER-MORE-SYNTAX More types, more contracts, more verification, more metadata — never more exotic syntax. @status:impl/done

@fact:CONSTRAINT-NEEDS-A-CHECKER Every constraint we add must be either enforced by a checker (A5) or it does not exist. @status:impl/done

@fact:remembered-rule-decays A rule a model must *remember* is a rule that decays; a rule a compiler *enforces* is a rule that holds. @status:spec/done

@fact:STRICTNESS-IS-THE-COMPILERS The discipline's strictness is the compiler's strictness, extended. @status:impl/done

@fact:BANS-CARRY-ESCAPE-HATCHES **Bans carry escape hatches.** @status:impl/done

@fact:FORBIDDEN-LEGAL-WITH-REASON Forbidden-by-default constructs (raw `unwrap` in domain logic, inline asm, proc-macro magic, stringly-typed protocols) remain legal *with machinery and a recorded reason* — the `unsafe` / `#[spec(deviates, reason)]` pattern. @status:impl/done

@fact:BAN-WITHOUT-HATCH-IS-A-BUG A ban with no escape hatch is a bug in the discipline; a deviation with no reason is a bug in the code. @status:impl/done

@fact:PARITY-ACROSS-PROJECTIONS **The strictness is equal across projections.** No language projection enforces the discipline more weakly than another; a rule the pilot enforces is either enforced in every projection or its absence carries a recorded reason. @status:impl/done

@fact:PARITY-IS-THE-PROJECTION-TWIN-OF-THE-HATCH This is the projection-level twin of `##BAN-WITHOUT-HATCH-IS-A-BUG`: an unexplained asymmetry between projections is a bug in the discipline exactly as a reasonless deviation is a bug in the code. A projection is weaker *with machinery and a recorded reason* — a language genuinely lacking an idiom's analogue (the compiler already enforces it; the idiom does not exist in that language) records that, and the recorded reason is the escape hatch. @status:impl/done

@fact:PARITY-PILOT-IS-A-BAR-NOT-A-PRIVILEGE The pilot language is the current reference bar because it is furthest along, not because it is privileged; as a projection matures past the pilot on some axis, the bar rises to it. A new language inherits the law on arrival — it is a projection of the one discipline, held to the same floor. @status:impl/done

@fact:PARITY-GAP-IS-NEVER-SILENT A projection weaker on some rule with no recorded reason is not a smaller stack; it is the discipline silently decaying — the failure mode A5 and A6 exist to make impossible. Weakening a rule for a projection because building its checker is harder there is the same category of error as dropping the rule for being unused (§4): the checker is built or the reason is recorded, never the rule quietly relaxed. @status:spec/done

## 5. Runnable capital: explanation must be executable {#runnable-capital}

@fact:second-decisive-result The second decisive result: weak agents given a *written* distillation of a strong agent's strategy barely improved; given an *executable* helper library carrying the same strategy, they leapt (Sonnet 4.6 on Brainfuck: 12→12 with text, 12→64 with runnable helpers). @status:spec/done

@fact:mechanism-cannot-build-the-code The mechanism: mid-tier models do not lack the idea; they cannot build the reusable code to carry it out. @status:spec/done

@fact:EXPLANATION-MUST-BE-RUNNABLE-CAPITAL Therefore: **explanation capital must be runnable capital.** @status:impl/done

@fact:META-LAYER-SHIPS-SCAFFOLDS The meta-layer ships *executable scaffolds* — generators, typed builders, runnable contracts, differential oracles, compiled examples, local simulators (the nine classes; see `02-EXECUTABLE-SCAFFOLDS.md`). @status:impl/done

@fact:PROSE-IS-A-WISH Prose that *could* be a checker, a doctest, or a typed API is a WISH until it becomes one. @status:impl/done

@fact:spec-weaker-than-shipped-macro A spec that says "use the registry pattern" is weaker than one that ships the registry macro plus a working example. @status:spec/done

@fact:boundary-result-is-generation **The honest boundary on this:** that result is about *generation* against an unfamiliar target. @status:spec/done

@fact:boundary-transfer-is-e-mid Transfer to *comprehension and modification* of in-distribution Rust is [E-mid] — plausible, not yet measured on our codebase. @status:spec/done

@fact:boundary-pilot-must-validate It is the primary thing the pilot must validate. @status:spec/done

## 6. Delivery: the discipline is not "know N rules" {#delivery}

@fact:owner-worry-is-correct The owner's central worry is correct: a weak model cannot apply forty rules at once, in the right order. @status:spec/done

@fact:RULES-NEVER-ALL-ACTIVE-AT-ONCE The resolution is that **rules are never all active at once.** @status:impl/done

@fact:CARD-CARRIES-FOUR-PARTS Each rule/pattern is a *card* (`01-PATTERN-CARD-FORMAT.md`) carrying a **Trigger** (when to switch on), a short **Routine** (≤7 steps), a **Checker** (machine verification), and a **Budget** (attention cost). @status:impl/done

@fact:HARNESS-DELIVERS-LAZY-PUSH The harness delivers only the cards whose triggers fire, as a small activation-matched set (lazy-push). *Specified, not built at card grain: no harness reads a card. Lazy-push itself is real one level up — `vibe` implements `DeliveryMode::LazyPush` for **subskills**, matching an agent's task description against a subskill `description`, with `vibe check` enforcing that a lazy-push unit carries one and warning on activation overlap between siblings. Nothing applies that machinery to cards: no reader of `card-ops`, of a card `trigger`, or of `cards/INDEX.md` exists in any language anywhere in the repository. Card delivery today is a boot instruction a session follows by hand, not an activation match the harness computes.* @status:spec/done

@fact:triggers-escalate-lead Triggers escalate by cost: @status:impl/done

1. @fact:TRIGGER-INLINE **Inline (edit-time)** — lint-detectable, fires in the per-cell loop; the cheapest mode and the one that fires most often. Each stack's `cards/INDEX.md` is the roster: of the nine scaffold cards, 2 sit here (C, F) against 5 at gate. @status:impl/done
2. @fact:TRIGGER-GATE **Gate (merge-time)** — heavier checks (oracles, proofs) that need not run per keystroke. @status:impl/done
3. @fact:TRIGGER-RAID **Raid (scheduled)** — swept periodically across a layer when per-edit triggers cannot keep up (`03-RAID-PLAYBOOK.md`). @status:impl/done
4. @fact:TRIGGER-REVIEW **Review (human/strong-agent)** — needs judgment a weak reader lacks. @status:impl/done

@fact:grounded-in-agentbench This is grounded in the AGENTbench result (Feb 2026): bloated context *hurts* weak agents; minimal, sufficient context helps. @status:spec/done

@fact:MINIMAL-SUFFICIENCY-OBEYED The discipline therefore proselytizes minimal sufficiency — and obeys it: this package is a full authoring/review artifact, but runtime delivery to a weak reader is an *extract* (the card's ops block), never the whole corpus. @status:impl/done

## 7. The honest boundary (what we do not yet know) {#honest-boundary}

@fact:beta-status This is a beta. @status:impl/done

@fact:stated-plainly-lead Stated plainly so the pilot can falsify it: @status:spec/done
- @fact:BOUNDARY-TRANSFER-UNPROVEN **Transfer is unproven.** The executable-scaffold result is generation, not modification. [E-mid]. @status:spec/done
- @fact:BOUNDARY-THERE-IS-A-FLOOR **There is a floor.** Even with executable scaffolds, the weakest models (Haiku-4.5-class, and our target Qwen-32B may sit lower on some axes) did *not* recover — scaffolds amplify capability, they do not create it. The discipline lowers the floor; it does not remove it. @status:spec/done
- @fact:BOUNDARY-SURFACE-IS-CURRENT **Surface-distribution is current.** The "stay in-distribution" law is tied to today's model generation; it carries a sunset (R-050) and must be re-checked as models change. *The obligation stands; the sunset it names has no carrier. R-050 is authored in no document that ships — every occurrence across this package, the language stacks, the engine crates and the host is a citation, and the ATLAS roster it would live in holds only `BLD-` / `DR1-` / `DR2-` / `R2C-` / `R3-` ids. Nothing schedules the re-check, expires the law, or records when it was last examined. The sunset **mechanism** does exist one grain over — every one of the 22 entries in the host's debt registry carries a `sunset` field, and cards carry a `Sunset:` clause in their Risks band — so the pattern is proven and simply not applied to this law.* @status:spec/done
- @fact:BOUNDARY-MEASUREMENT-DEFERRED **Measurement is deferred by design.** We build the core on internal logic plus others' published evidence, and instrument later, at a buyer's expense. Every card therefore carries a falsifiable `prediction` in place of a present measurement. @status:spec/done

@fact:names-its-own-failure-modes A discipline that names its own failure modes is more trustworthy than one that hides them. @status:spec/done

@fact:this-one-names-them This one names them. @status:spec/done

## 8. The package map {#package-map}

@fact:guiding-layer-lead **Guiding layer (T1, language-independent):** @status:impl/done
- @fact:MAP-MANIFESTO `00-MANIFESTO.md` — this document. @status:impl/done
- @fact:MAP-PATTERN-CARD-FORMAT `01-PATTERN-CARD-FORMAT.md` — the format every pattern is written in (GoF × JEP × operational layer). @status:impl/done
- @fact:MAP-EXECUTABLE-SCAFFOLDS `02-EXECUTABLE-SCAFFOLDS.md` — the nine scaffold classes; the runnable-capital catalog. @status:impl/done
- @fact:MAP-RAID-PLAYBOOK `03-RAID-PLAYBOOK.md` — layered, scheduled refactoring campaigns (raids). @status:impl/done
- @fact:MAP-SWEEP-PLAYBOOK `04-SWEEP-PLAYBOOK.md` — the standing sweep that holds a tree inside the Discipline between campaigns. @status:impl/done
- @fact:MAP-CAMPAIGN-FORM `05-CAMPAIGN-FORM.md` — the campaign paper trail: cold-executable plans, baselines, predictions, logs, reports. @status:impl/done
- @fact:MAP-WAL-CONVENTION `06-WAL-CONVENTION.md` — session-durable project state (optional but preferred). @status:impl/done

@fact:mechanisms-lead **Mechanisms (T1, language-independent; implemented per-stack):** @status:impl/done
- @fact:MAP-ENGINE-CONFORM `mechanisms/ENGINE-CONFORM-v0.1.md` — the conformance engine: fact store, rules-as-queries, SARIF, ratchet baseline. @status:impl/done
- @fact:MAP-PROP-014-SPECMAP `mechanisms/PROP-014-specmap-bidirectional-traceability.md` — spec↔code traceability: anchors, revisions, tags, the index. @status:impl/done
- @fact:MAP-BROWNFIELD-PROTOCOL `mechanisms/BROWNFIELD-PROTOCOL-v0.1.md` — terraforming unfinished projects: inventory-not-gate, the registries, xfail-strict, characterization. @status:impl/done
- @fact:MAP-LEDGER-INTENT `mechanisms/LEDGER-INTENT-v0.1.md` — the intent ledger: facts vs interpretations, epoch-keyed cache. @status:impl/done

@fact:SPEC-UNIT-URI-FORM Spec-unit URIs for this package read `spec://org.vibevm.ai-native/core-ai-native/<docpath>#<anchor>` (e.g. `spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules`); the Rust implementations ship in `stack:org.vibevm.ai-native/rust-ai-native-lang` (`rust-ai-native-conform`, `rust-ai-native-specmap`, `rust-ai-native`). @status:impl/done

@fact:language-projections-lead **Language projections (T2):** @status:impl/done
- @fact:MAP-RUST-GUIDE `spec/rust/GUIDE-AI-NATIVE-RUST.md` in `stack:org.vibevm.ai-native/rust-ai-native-lang` — the law and scaffolds projected onto Rust; supersedes GUIDE-RUST-v0.1. (Pilot language.) @status:impl/done
- @fact:MAP-RUST-TCG `rust/tools/rust-ai-native-tcg.md` — token-level type-aware constrained generation for Rust (very-far-future; carries the family prefix per PROP-028 §2.4). @status:idea/plan
- @fact:MAP-RUST-TCG-AGENTIC The AGENTIC delivery shipped first: `rust/tools/vibe-agentic-tcg-rust.md` — the consultation oracle over rust-analyzer. @status:impl/done
- @fact:MAP-TYPESCRIPT-GUIDE `typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md` — projected onto TypeScript (typed language only; JS guide separate). The language where the generation-time type oracle already exists and codemods are mature. @status:impl/done
- @fact:MAP-TYPESCRIPT-TCG `typescript/tools/typescript-ai-native-tcg.md` — type-aware constrained generation for TypeScript (a wrap-and-extend of existing PLDI'25 work, not from scratch). @status:unknown
- @fact:MAP-OTHER-LANGUAGES Other languages (Python, C++, Go, Java, Kotlin) projected after Rust + TypeScript are validated. @status:spec/done

@fact:cards-lead **Cards (the patterns) — shipped per-language by each stack:** @status:impl/done
- @fact:MAP-CARDS-INDEX `cards/INDEX.md` — registry, axes, trigger modes (one per language stack). @status:impl/done
- @fact:MAP-CARDS-SCAFFOLDS `cards/scaffold-{a..i}-*.md` — the nine scaffold patterns in their per-language card shape. The core (T1) defines the card FORMAT (`01-PATTERN-CARD-FORMAT.md`) and the scaffold CATALOG (`02-EXECUTABLE-SCAFFOLDS.md`), both language-neutral; each language stack ships the concrete `cards/` (Rust cards in `stack-rust-ai-native`, TypeScript cards in `stack-typescript-ai-native`), so the weak-reader runtime surface for an edit is a language-matched Band-3 block. @status:impl/done

@fact:appendix-lead **Appendix (synthesis provenance):** @status:impl/done
- @fact:MAP-CONTRADICTION-MAP `appendix/CONTRADICTION-MAP.md` — where sources and hypotheses conflict, and the resolutions. @status:impl/done
- @fact:MAP-ATLAS `appendix/ATLAS.md` — the findings ledger rendered for humans (generated from `findings.jsonl`). *Correction: the ledger is real and is the appendix itself — 87 `#FINDING-*` records, whose ids the cards cite and resolve against. The generator is not: no `findings.jsonl` is tracked anywhere in this repository, in any package, or in any consumer, so ATLAS is authored directly rather than rendered from a source. Read the parenthetical as the intended pipeline, not as the current one (F-088).* @status:impl/done

@fact:ADOPTION-PLAN-LIVES-OUTSIDE The vibevm-specific adoption plan lives OUTSIDE this package, in the host's `terraform/`, because the Discipline is the product and vibevm is its pilot. @status:impl/done
