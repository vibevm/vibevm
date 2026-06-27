# The AI-Native Code Discipline — Manifesto
**Discipline v0.2 · status: BETA · supersedes DISCIPLINE-CHARTER-v0.1**

*This is the root document of the Discipline. The Discipline is a product in its own right: a language-independent set of principles, plus per-language projections, for writing code that is optimal for COMPREHENSION and SAFE MODIFICATION by AI agents — explicitly including weak readers. vibevm is the first pilot of this product, not its scope boundary.*

*Maturity is marked everywhere. Some claims are benchmark-backed [E-strong], some are supported by adjacent evidence [E-mid], some are first-principles awaiting validation [E-hyp]. This document is a falsifiable beta, revised on pilot evidence only.*

---

## 0. What this is, in one paragraph

Code generation by frontier models is already good. The unsolved problem is the rest of the lifecycle: a model — often a *different, weaker* model than the author — must later read, understand, and safely change that code. The Discipline optimizes for that. Its single design target is to **lower the minimum model capability required to maintain code safely**, so that a swarm of small agents can maintain code that a frontier model authored.

## 1. The target function: author/reader asymmetry

The economics that justify everything below: a strong author runs **once**; weak readers run **thousands of times**. So the discipline moves cost from maintenance-time to authoring-time. The strong author materializes expensive cognition into infrastructure — meta-layer, contracts, executable scaffolds, recorded rationale — and the weak swarm lives off that capital. **The Discipline is a capability-gap compensator.** A clever construct with no materialized rationale is theft from the reader pool.

This is why "make the code simpler" is the wrong frame. We are not lowering the ceiling of what the code does; we are lowering the floor of who can safely touch it.

## 2. The six axioms, projected to the language level

The axioms are unchanged from the Charter; here is what each *means for how code and its metadata are written*.

- **A1 — Explainability.** Every artifact carries a machine-resolvable chain from code to requirement to rationale (`spec://` URIs, in-source `#[spec(...)]` edges). Unexplainable code is unmergeable. *Language-level:* identifiers, errors, and items are anchored to requirements, not free-floating.
- **A2 — Never pay twice.** Derived cognition is materialized content-addressed and dependency-tracked. *Language-level:* anything mechanically derivable (boilerplate, indexes, glue) is generated, not hand-maintained; the strong author's analysis is stored, not re-run.
- **A3 — Algorithmic floor.** Where a deterministic procedure exists, the LLM is forbidden; its outputs sink below the floor. *Language-level:* push as much intent as possible into machine-checkable form — types, exhaustiveness, contracts — so a checker, not a model, enforces it. **This is the engine of the central law (§3).**
- **A4 — Human accountability.** AI checks and proposes; the human is the accountable author; diffs stay human-reviewable. *Language-level:* no change is so clever a human cannot review it; determinism keeps diffs semantic.
- **A5 — Rule = code.** Every rule ships a checker or is explicitly a WISH. *Language-level:* a style rule with no linter is not a rule; it is documentation that decays (see §6). Rulebook health is the wish-ratio, not the page count.
- **A6 — Reality before aspiration.** Gates measure deltas against inventoried reality; debt, unimplemented intent, and contradiction are first-class tracked objects. *Language-level:* the code's actual state — not its intended state — is what tooling reasons over.

## 3. The central law: idiomatic inside the file, engineered around the file

The strongest empirical result we found, dated and controlled: models collapse on out-of-distribution *surface syntax* (EsoLang-Bench, Mar 2026: frontier models 0–11% on esoteric languages they could solve trivially in Python) — **and** current agents largely *recover* that collapse through tools and in-session strategy (the Jun 2026 follow-up: the same tasks, 87–100% with file-editing + interpreter loops). In-context learning could **not** teach the unfamiliar surface (few-shot gave ~0 benefit); the recovery came from the verification loop and from building executable helpers, not from prose.

The law that follows:

> **Keep the code surface inside the training distribution. Put all the strictness into the meta-layer, the type system, and the verification loop — which sit AROUND the code, not in its syntax.**

Concretely: AI-Native Rust looks like *ordinary idiomatic Rust* at the token level (no invented notation, no exotic dialect — that would inherit the OOD penalty). What is unusual is everything around it: dense machine-checkable metadata, contract-bearing types, executable scaffolds, and a fast per-unit verification loop. We do not make the language stranger; we make its envelope stricter.

## 4. Stricter, not simpler — and where the strictness lives

"AI-native" is **more** discipline, not less — but the added strictness lives exclusively in machine-checkable form. More types, more contracts, more verification, more metadata — never more exotic syntax. Every constraint we add must be either enforced by a checker (A5) or it does not exist. A rule a model must *remember* is a rule that decays; a rule a compiler *enforces* is a rule that holds. The discipline's strictness is the compiler's strictness, extended.

**Bans carry escape hatches.** Forbidden-by-default constructs (raw `unwrap` in domain logic, inline asm, proc-macro magic, stringly-typed protocols) remain legal *with machinery and a recorded reason* — the `unsafe` / `#[spec(deviates, reason)]` pattern. A ban with no escape hatch is a bug in the discipline; a deviation with no reason is a bug in the code.

## 5. Runnable capital: explanation must be executable

The second decisive result: weak agents given a *written* distillation of a strong agent's strategy barely improved; given an *executable* helper library carrying the same strategy, they leapt (Sonnet 4.6 on Brainfuck: 12→12 with text, 12→64 with runnable helpers). The mechanism: mid-tier models do not lack the idea; they cannot build the reusable code to carry it out.

Therefore: **explanation capital must be runnable capital.** The meta-layer ships *executable scaffolds* — generators, typed builders, runnable contracts, differential oracles, compiled examples, local simulators (the nine classes; see `02-EXECUTABLE-SCAFFOLDS.md`). Prose that *could* be a checker, a doctest, or a typed API is a WISH until it becomes one. A spec that says "use the registry pattern" is weaker than one that ships the registry macro plus a working example.

**The honest boundary on this:** that result is about *generation* against an unfamiliar target. Transfer to *comprehension and modification* of in-distribution Rust is [E-mid] — plausible, not yet measured on our codebase. It is the primary thing the pilot must validate.

## 6. Delivery: the discipline is not "know N rules"

The owner's central worry is correct: a weak model cannot apply forty rules at once, in the right order. The resolution is that **rules are never all active at once.** Each rule/pattern is a *card* (`01-PATTERN-CARD-FORMAT.md`) carrying a **Trigger** (when to switch on), a short **Routine** (≤7 steps), a **Checker** (machine verification), and a **Budget** (attention cost). The harness delivers only the cards whose triggers fire, as a small activation-matched set (lazy-push). Triggers escalate by cost:

1. **Inline (edit-time)** — lint-detectable, fires in the per-cell loop. Most cards aim here.
2. **Gate (merge-time)** — heavier checks (oracles, proofs) that need not run per keystroke.
3. **Raid (scheduled)** — swept periodically across a layer when per-edit triggers cannot keep up (`03-RAID-PLAYBOOK.md`).
4. **Review (human/strong-agent)** — needs judgment a weak reader lacks.

This is grounded in the AGENTbench result (Feb 2026): bloated context *hurts* weak agents; minimal, sufficient context helps. The discipline therefore proselytizes minimal sufficiency — and obeys it: this package is a full authoring/review artifact, but runtime delivery to a weak reader is an *extract* (the card's ops block), never the whole corpus.

## 7. The honest boundary (what we do not yet know)

This is a beta. Stated plainly so the pilot can falsify it:
- **Transfer is unproven.** The executable-scaffold result is generation, not modification. [E-mid].
- **There is a floor.** Even with executable scaffolds, the weakest models (Haiku-4.5-class, and our target Qwen-32B may sit lower on some axes) did *not* recover — scaffolds amplify capability, they do not create it. The discipline lowers the floor; it does not remove it.
- **Surface-distribution is current.** The "stay in-distribution" law is tied to today's model generation; it carries a sunset (R-050) and must be re-checked as models change.
- **Measurement is deferred by design.** We build the core on internal logic plus others' published evidence, and instrument later, at a buyer's expense. Every card therefore carries a falsifiable `prediction` in place of a present measurement.

A discipline that names its own failure modes is more trustworthy than one that hides them. This one names them.

## 8. The package map

**Guiding layer (T1, language-independent):**
- `00-MANIFESTO.md` — this document.
- `01-PATTERN-CARD-FORMAT.md` — the format every pattern is written in (GoF × JEP × operational layer).
- `02-EXECUTABLE-SCAFFOLDS.md` — the nine scaffold classes; the runnable-capital catalog.
- `03-RAID-PLAYBOOK.md` — layered, scheduled refactoring sweeps.

**Language projections (T2):**
- `rust/GUIDE-AI-NATIVE-RUST.md` — the law and scaffolds projected onto Rust; supersedes GUIDE-RUST-v0.1. (Pilot language.)
- `rust/tools/vibe-tcg.md` — type-aware constrained generation for Rust (a from-scratch, multi-year bet on rust-analyzer).
- `typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md` — projected onto TypeScript (typed language only; JS guide separate). The language where the generation-time type oracle already exists and codemods are mature.
- `typescript/tools/vibe-tcg-ts.md` — type-aware constrained generation for TypeScript (a wrap-and-extend of existing PLDI'25 work, not from scratch).
- Other languages (Python, C++, Go, Java, Kotlin) projected after Rust + TypeScript are validated.

**Cards (the patterns):**
- `cards/INDEX.md` — registry, axes, trigger modes.
- `cards/scaffold-{a..i}-*.md` — the nine scaffold patterns in card format. The core ships the Rust pilot's reference cards; each language stack ships its own `cards/` projection, so the weak-reader runtime surface for an edit is a language-matched Band-3 block. A future symmetry pass may unify both languages' Band-3 in the core.

**Appendix (synthesis provenance):**
- `appendix/CONTRADICTION-MAP.md` — where sources and hypotheses conflict, and the resolutions.
- `appendix/ATLAS.md` — the findings ledger rendered for humans (generated from `findings.jsonl`).

The vibevm-specific adoption plan lives OUTSIDE this package, in `vibevm-terraform/`, because the Discipline is the product and vibevm is its pilot.
