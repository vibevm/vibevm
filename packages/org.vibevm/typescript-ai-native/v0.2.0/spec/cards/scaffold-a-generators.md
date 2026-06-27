# CARD: scaffold-a-generators — Generators / Codegen (TypeScript)
**Discipline v0.2 · BETA · T2 · TypeScript**

## Band 1 — Identity & Recognition
Classification: layer=A (language-shape) + C (meta); mechanism=scaffold A.
Intent: Where an artifact is mechanically derivable from a smaller spec, ship a program that EMITS it (externalizing fragile implicit structure into named inputs and reusable functions) plus its committed output plus a determinism check — instead of hand-maintained output. TypeScript has the most mature codegen ecosystem of any language; favor generation.
Also Known As: codegen; build script; metaprogramming; template generation; emitter; `ts-morph`/Compiler API generation; schema-to-type generation; `satisfies`/`as const` tables.
Applicability / Recognition: Apply when — boilerplate repeats across cells; an artifact (a discriminated union, its exhaustive `switch`, a serializer, a route table, an FFI/IPC binding) is a mechanical function of a schema/manifest; types duplicate a source of truth (a Zod schema, an OpenAPI/GraphQL/Prisma definition); raw hand-authoring requires tracking implicit bookkeeping a weak reader cannot hold (R2C-008). *Detector seed:* near-duplicate TS blocks differing only by a key, OR a hand-maintained type/table that mirrors a schema → recognition fires.

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent must add a variant to a 12-case state machine spread across the discriminated union, the exhaustive `switch`, and the error type. Hand-editing three sites desynchronizes them. A generator emitting all three from one `as const` table (or a `z.infer` from one Zod schema) turns the task into "add one row" — the strong author's structural decision is the emitter; the weak reader fills a named input.
Structure & Participants: *Schema/manifest* (the small input — a Zod schema, an `as const` table, an OpenAPI/GraphQL/Prisma file) · *Generator* (`ts-morph` / TypeScript Compiler API / a codegen script) · *Committed output* (plain idiomatic TypeScript) · *Determinism check* (regenerate-and-diff in CI).
Collaborations: Output is checked by the Class E loop (`tsc --noEmit`); the generator emits Class G Twoslash examples for its products; pairs with Class B (generated types ARE the typed surface). In raids, regeneration is a batch operation; for the weakest tier, generation is exposed as a fixed Class I codemod.
Goals / Non-Goals: *Goals:* eliminate hand-maintained derivable artifacts (A2/A3); shrink edit surface to the schema. *Non-Goals:* NOT a custom transform of production logic (generators emit standard TypeScript, never a private dialect); NOT for one-off code with no repetition; NOT generating business decisions (generate structure, not logic).
Consequences: (+) edit surface collapses to the schema; (+) consistency is structural, not vigilance-based; (+) the single-source schema also feeds the §2 runtime validator. (−) the generator itself is code to maintain; (−) over-generation hides logic — generate structure, not business decisions.
Alternatives: hand-written code (fine when non-repetitive); a typed builder (Class B) when the variation is per-call not per-artifact; a schema library's built-in `infer` (the canonical, lowest-effort instance — prefer it).
Risks & Assumptions: assumes the derivation is truly mechanical; a generator encoding a business decision is misuse. *Sunset:* if a future language/tooling feature expresses the pattern natively, the generator retires. Weak agents may struggle to MODIFY a generator ([E-hyp] build/use boundary) — for the weakest tier, expose generation as a fixed codemod (Class I), not a free-form script to edit.
Evidence & Transfer-strength: R2C-008 (executable generators transformative, benchmark), R2C-003 (metaprogramming as adaptation, benchmark), A2/A3. Class: benchmark + production. Tag: **[E-strong]** (for generation). TypeScript note: the codegen ecosystem (`ts-morph`, Compiler API, schema generators) is the most mature of any language, so the build-side cost is lower here than the Rust baseline.

## Band 3 — Operation
```card-ops
trigger: WHEN near-duplicate TS blocks differ only by a key, OR a hand-maintained type/table mirrors a schema THEN apply
mode: raid            # also gate when a new derivable artifact is introduced
routine:
  1. Identify the smaller schema/manifest the artifact is a function of (Zod schema, as const table, OpenAPI/GraphQL/Prisma).
  2. Write the emitter (ts-morph / Compiler API / codegen script) producing the artifact.
  3. Commit the generated output as plain idiomatic TypeScript.
  4. Add a CI determinism check: regenerate and diff; drift fails.
  5. Emit Class-G Twoslash examples for generated public items.
  6. Replace all hand-maintained copies with the generated output.
checker: eslint/conform `derivable-not-hand-maintained` + CI regenerate-and-diff + `tsc --noEmit` on output
raid_role: layer=codegen; order=after:naming-and-branding; batch=package
budget: active_rules=1; first_signal=regen+tsc (<60s/package)
```
