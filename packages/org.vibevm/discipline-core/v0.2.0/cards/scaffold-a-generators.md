# CARD: scaffold-a-generators — Generators / Codegen
**Discipline v0.2 · BETA**

## Band 1 — Identity & Recognition
Classification: layer=A (language-shape) + C (meta); mechanism=scaffold A.
Intent: Where an artifact is mechanically derivable from a smaller spec, ship a program that EMITS it (externalizing fragile implicit structure into named inputs and reusable functions) plus its committed output plus a determinism check — instead of hand-maintained output.
Also Known As: codegen; build script; metaprogramming; macro expansion; template generation; emitter.
Applicability / Recognition: Apply when — boilerplate repeats across cells; an artifact (transition table, FFI binding, serializer, exhaustive match) is a mechanical function of a manifest/spec; raw hand-authoring requires tracking implicit bookkeeping a weak reader cannot hold (the Brainfuck-generator case, R2C-008). *Detector seed:* near-duplicate code blocks differing only in a key, OR a hand-maintained table that mirrors a spec → recognition fires.

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent must add a variant to a 12-arm state machine spread across match, transition table, and error enum. Hand-editing three sites desynchronizes them. A generator emitting all three from one table turns the task into "add one row" — the strong author's structural decision is the emitter; the weak reader fills a named input.
Structure & Participants: *Spec/manifest* (the small input) · *Generator* (Rust build.rs or proc-macro / external codegen) · *Committed output* (plain idiomatic Rust) · *Determinism check* (regenerate-and-diff in CI).
Collaborations: Output is checked by Class E loop; generator emits Class G doctests for its products; pairs with Class B (generated types). In raids, regeneration is a batch operation.
Goals / Non-Goals: *Goals:* eliminate hand-maintained derivable artifacts (A2/A3); shrink edit surface to the spec. *Non-Goals:* NOT a custom compiler for production code (forbidden) — generators emit standard Rust; NOT for one-off code with no repetition.
Consequences: (+) edit surface collapses to the spec; (+) consistency is structural, not vigilance-based. (−) generator itself is code to maintain; (−) over-generation hides logic — generate structure, not business decisions.
Alternatives: hand-written code (fine when non-repetitive); a typed builder (Class B) when the variation is per-call not per-artifact.
Risks & Assumptions: assumes the derivation is truly mechanical; a generator encoding a business decision is misuse. *Sunset:* if a future language feature expresses the pattern natively, the generator retires. Weak agents may struggle to MODIFY a generator ([E-hyp] build/use boundary) — for the weakest tier, expose generation as a fixed codemod (Class I).
Evidence & Transfer-strength: R2C-008 (executable generators transformative, benchmark), R2C-003 (metaprogramming as adaptation, benchmark), A2/A3. Class: benchmark + production. Tag: **[E-strong]** (for generation).

## Band 3 — Operation
```card-ops
trigger: WHEN near-duplicate blocks differ only by a key, OR a hand-maintained artifact mirrors a spec THEN apply
mode: raid            # also gate when a new derivable artifact is introduced
routine:
  1. Identify the smaller spec/manifest the artifact is a function of.
  2. Write the emitter (build.rs / proc-macro / external) producing the artifact.
  3. Commit the generated output as plain Rust.
  4. Add a CI determinism check: regenerate and diff; drift fails.
  5. Emit Class-G doctests for generated public items.
  6. Replace all hand-maintained copies with the generated output.
checker: conform T-sem `derivable-not-hand-maintained` + CI regenerate-and-diff
raid_role: layer=codegen; order=after:naming-uniformity; batch=crate
budget: active_rules=1; first_signal=regen+compile (<60s/crate)
```
