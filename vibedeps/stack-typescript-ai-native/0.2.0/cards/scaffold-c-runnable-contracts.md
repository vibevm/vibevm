# CARD: scaffold-c-runnable-contracts — Runnable Contracts (TypeScript)
**Discipline v0.2 · BETA · T2 · TypeScript**

## Band 1 — Identity & Recognition
Classification: layer=E (verification); mechanism=scaffold C.
Intent: Express pre/post-conditions and invariants as EXECUTING assertions attached to the unit and restated at use sites — so a paged reader gets ground truth without simulating the body. Uniquely in TypeScript, an **assertion function** (`asserts x is T`) both checks at runtime AND narrows the static type — one artifact carrying the contract in both worlds.
Also Known As: design by contract; assertion function; type guard; `invariant()`/`tiny-invariant`; runtime contract; refinement; schema validation; require/ensure.
Applicability / Recognition: Apply when — a function has a non-obvious precondition; a cross-cell invariant is relied upon far from where it is established (R3-009); a behavioral claim is currently only in prose; an `unknown`/wide value is used as if narrowed without a check. *Detector seed:* a comment asserting a property ("already validated", "non-empty", "sorted") with no adjacent runnable check, OR a value narrowed by a bare `as` instead of a guard → recognition fires (a prose claim is adversarial if it lies, R2C-004; a bare `as` is the erasure hazard, §8).

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent edits a function relying on "input already validated", stated only in a JSDoc three modules away. It cannot do the whole-program inference to confirm. An `assertIsValid(input)` — an `asserts` function — either throws at runtime OR narrows the type so downstream code statically sees the validated type. The invariant becomes local ground truth AND a compile-time fact.
Structure & Participants: *Assertion function* (`asserts x is T` — the TS-unique dual checker) · *`invariant()`/`tiny-invariant`* (throwing predicate) · *Schema* (Zod/Valibot as an executable contract at boundaries) · *Use-site witness* (assertion where the invariant is relied upon) · *Property test* (behavioral backing, `fast-check`).
Collaborations: Defines "valid"/"equivalent" for Class D oracles; pairs with Class B (types for protocol/identity, contracts for value invariants); the §2 boundary schemas ARE Class-C contracts at the erasure edge; failures emit Class F diagnostics.
Goals / Non-Goals: *Goals:* make load-bearing invariants machine-checked at the point of reliance, narrowing the type where possible. *Non-Goals:* NOT validating everything everywhere (cost) — boundary + load-bearing invariants only; NOT a substitute for the spec that JUSTIFIES the invariant; NOT a `console.assert` stripped in production — use a throwing assertion.
Consequences: (+) invariants become local, checkable, AND type-narrowing; (+) a paged reader trusts the assertion, not distant prose. (−) restatements can drift — keep them assertions (which throw, not mislead); (−) runtime cost on hot paths — scope to seams/boundaries.
Alternatives: a branded type (Class B) when the invariant is identity/protocol; a bare `as` (rejected — lies silently, the erasure hazard §8); a comment (rejected — lies silently).
Risks & Assumptions: assumes the invariant is expressible as a runnable predicate; an assertion function with a wrong predicate narrows to a FALSE type — test the predicate itself. *Sunset:* if a branded type or a schema makes the invariant statically guaranteed, the assert retires there.
Evidence & Transfer-strength: DR1-019 (contracts give success criterion without body, theory), R3-009 (use-site restatement, theory), R2C-004 (prose lies harm, benchmark). Class: benchmark + theory. Tag: **[E-mid]**.

## Band 3 — Operation
```card-ops
trigger: WHEN a prose property claim lacks an adjacent runnable check, OR a cross-cell invariant is used far from its definition, OR a wide/unknown value is used as narrowed without a guard THEN apply
mode: inline
routine:
  1. State the invariant as a boolean predicate or a schema.
  2. Express it as an `asserts`/type-guard function (narrows) or a `Result`-returning validator at the definition AND at each use site relying on it.
  3. At the erasure boundary, narrow through a single-source schema (§2), never a bare `as`.
  4. Back behavioral claims with a `fast-check` property test.
  5. Replace the prose claim with the assertion (or label it verified, linking the check).
checker: @typescript-eslint `invariant-witnessed` (flags declared invariants with unwitnessed use sites; flags `as`-narrowing where a guard is required) + `vitest`
raid_role: layer=cells; order=after:naming-and-branding; batch=cell
budget: active_rules=2; first_signal=vitest run <cell> (<60s)
```
