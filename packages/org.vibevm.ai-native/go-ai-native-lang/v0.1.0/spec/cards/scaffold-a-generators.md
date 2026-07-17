# CARD: scaffold-a-generators — Generators / Codegen (Go)
**Discipline v0.2 · BETA · T2 · Go**

## Band 1 — Identity & Recognition
Classification: layer=A (language-shape) + C (meta); mechanism=scaffold A.
Intent: Where an artifact is mechanically derivable from a smaller spec, ship a program that EMITS it (externalizing fragile implicit structure into named inputs and reusable functions) plus its committed output plus a determinism check — instead of hand-maintained output. Go gives the pattern its own cultural slot: `go:generate` names the emitter next to its output's home.
Also Known As: codegen; `go:generate`; `stringer`; template generation; emitter; schema-to-type generation.
Applicability / Recognition: Apply when — boilerplate repeats across cells; an artifact (a const-enum's `String()` method, a transition table, a serializer, an exhaustive dispatch map) is a mechanical function of a manifest/spec; a hand-maintained table mirrors a schema. *Detector seed:* near-duplicate Go blocks differing only by a key, OR a hand-maintained artifact that mirrors a spec/table → recognition fires (R2C-008).

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent must add a variant to a 12-arm reconcile-action set spread across the const block, the `String()` method, and the dispatch map. Hand-editing three sites desynchronizes them. A `go:generate`-invoked emitter producing all three from one table turns the task into "add one row" — the strong author's structural decision is the emitter; the weak reader fills a named input.
Structure & Participants: *Spec/manifest* (the small input — a table, a schema file) · *Generator* (a `stringer`-class tool or a small `text/template`/`go/format` program) · *`//go:generate` directive* (the culture's own invocation slot) · *Committed output* (plain idiomatic Go, marked `// Code generated … DO NOT EDIT.`) · *Determinism check* (regenerate-and-diff in CI).
Collaborations: Output is checked by the Class E loop (`go build` + per-package test); the generator emits Class G `Example`s for its products; pairs with Class B (generated types ARE the typed surface). In raids, regeneration is a batch operation; for the weakest tier, generation is exposed as a fixed Class I codemod.
Goals / Non-Goals: *Goals:* eliminate hand-maintained derivable artifacts (A2/A3); shrink edit surface to the spec. *Non-Goals:* NOT reflection-at-runtime (that is the §7 ban — generation happens at build time and emits static Go); NOT for one-off code with no repetition; NOT generating business decisions (generate structure, not logic).
Consequences: (+) edit surface collapses to the spec; (+) consistency is structural, not vigilance-based; (+) the `DO NOT EDIT` header is machine-recognized by the whole Go toolchain (lint suppression, coverage exclusion). (−) the generator itself is code to maintain; (−) over-generation hides logic — generate structure, not business decisions.
Alternatives: hand-written code (fine when non-repetitive); a defined-type constructor (Class B) when the variation is per-call not per-artifact; generics (bounded, §1) when the repetition is purely type-shaped.
Risks & Assumptions: assumes the derivation is truly mechanical; a generator encoding a business decision is misuse. *Sunset:* if a future language feature expresses the pattern natively (as generics absorbed container codegen), the generator retires. Weak agents may struggle to MODIFY a generator ([E-hyp] build/use boundary) — for the weakest tier, expose generation as a fixed codemod (Class I).
Evidence & Transfer-strength: R2C-008 (executable generators transformative, benchmark), R2C-003 (metaprogramming as adaptation, benchmark), A2/A3. Class: benchmark + production. Tag: **[E-strong]** (for generation).

## Band 3 — Operation
```card-ops
trigger: WHEN near-duplicate Go blocks differ only by a key, OR a hand-maintained artifact mirrors a spec/table THEN apply
mode: raid            # also gate when a new derivable artifact is introduced
routine:
  1. Identify the smaller spec/manifest the artifact is a function of.
  2. Write the emitter (stringer-class tool, or a small text/template + go/format program).
  3. Add the //go:generate directive beside the output's home; commit the generated output with its DO-NOT-EDIT header.
  4. Add a CI determinism check: `go generate ./...` then `git diff --exit-code`.
  5. Emit Class-G Examples for generated public items.
  6. Replace all hand-maintained copies with the generated output; tag the generator INPUT with the spec edge (outputs are orphan-exempt).
checker: conform `derivable-not-hand-maintained` + CI regenerate-and-diff + go build on output
raid_role: layer=codegen; order=after:naming-uniformity; batch=package
budget: active_rules=1; first_signal=go generate + go build (<60s/package)
```
