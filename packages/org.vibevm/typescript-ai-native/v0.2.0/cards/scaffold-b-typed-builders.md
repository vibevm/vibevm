# CARD: scaffold-b-typed-builders — Typed Surfaces / Branding / Typestate (TypeScript)
**Discipline v0.2 · BETA · T2 · TypeScript**

## Band 1 — Identity & Recognition
Classification: layer=E (verification); mechanism=scaffold B.
Intent: Make the statistically-likely wrong call un-representable, so a hallucinated edit fails `tsc` before runtime — encoding protocol correctness AND nominal identity in types rather than docstrings. The TypeScript twist: structural typing makes same-shaped values interchangeable, so the first job is **branding** — manually recovering the nominal safety Rust's newtypes give for free.
Also Known As: branded types; nominal types; opaque types; discriminated union; phantom type; typestate builder; sealed union; `satisfies`-exhaustiveness; make-illegal-states-unrepresentable.
Applicability / Recognition: Apply when — a seam has a usage protocol (order of calls, required fields, valid states); a meaning-bearing primitive (`string`, `number`) crosses a boundary where its identity matters; an API takes multiple same-typed args or a boolean flag. *Detector seed:* a pub seam fn taking a bare `string`/`number`/`boolean` or two same-typed params, OR a runtime "is-ready" check, OR a same-shaped type used where a distinct identity is meant → recognition fires (~94% of compile errors are type-level; structural typing will NOT catch the identity swap without branding).

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent calls `transfer(fromAccount, toAccount, amount)` where `fromAccount` and `toAccount` are both `string`, and swaps them — structurally identical, `tsc` is silent, money moves the wrong way. With branded `AccountId` (or a builder requiring `.from(AccountId(...)).to(...)`), the swap or a bare `string` fails `tsc`; the error surfaces in the loop, not in production.
Structure & Participants: *Branded type* (primitive + erased brand tag) · *Discriminated union* (tagged variants; illegal states absent) · *Typestate / phantom-parameter builder* (type-mandatory required fields, encoded call order) · *Sealed union + `satisfies`* (closed extension, exhaustiveness).
Collaborations: Shrinks the input space Class D oracles must cover; the compiler under the maxed `tsconfig` (guide §1) is the Class E loop's primary checker; pairs with Class C for runtime invariants types can't express; the §2 boundary schemas PRODUCE branded types at the erasure edge.
Goals / Non-Goals: *Goals:* convert probable hallucinations — identity swaps, missing fields, wrong call order — into compile errors at seams. *Non-Goals:* NOT branding every local primitive (ergonomic cost) — scope to seam surfaces; NOT type-level wizardry (the §0 OOD tail); NOT a replacement for runtime validation at the erasure boundary (§2 — brands are erased).
Consequences: (+) a whole class of misuse becomes uncompilable; (+) the type IS the protocol doc. (−) branding ergonomics cost for human contributors (a branding helper eases it); (−) over-typing fights idiom — scope tightly; (−) brands are ERASED — they guard compile-time identity, not runtime, so they must be paired with §2 validators at any untyped boundary.
Alternatives: runtime validation (errors surface late — in production, not the loop); a contract (Class C) when the invariant is a value range, not a protocol/identity; a plain type alias (rejected — structural, gives no nominal guard).
Risks & Assumptions: assumes the protocol/identity is type-expressible; brands do not survive a runtime boundary (that is §2's job, not B's). *Sunset:* none material. Strong models may be mildly distorted by over-constraint — keep branding/typestate proportional.
Evidence & Transfer-strength: R3-008 (misuse-resistance, theory), DR2-012/R2C-005 (94% type-level errors; type-awareness cuts compile errors, benchmark). Class: benchmark + theory. Tag: **[E-mid]**.

## Band 3 — Operation
```card-ops
trigger: WHEN a pub seam fn takes a bare string/number/boolean or duplicate-same-type args, OR a runtime is-ready check exists, OR a same-shaped type is used where a distinct identity is meant THEN apply
mode: gate            # introduced at seam design; checked at merge
routine:
  1. Brand each meaning-bearing primitive at the seam (`string & { readonly __brand }`) via a helper.
  2. Encode call-order/required-field protocol as a phantom-parameter builder or typestate; model states as a discriminated union.
  3. Close unions; assert exhaustiveness with `satisfies` / `assertNever` in the default branch.
  4. Delete the now-impossible runtime validity checks.
  5. Confirm the previously-wrong call no longer type-checks (add a `tsd`/`expectTypeOf` or `@ts-expect-error` compile-fail test).
checker: @typescript-eslint `seam-protocol-typed` (+ `no-bare-primitive-at-seam`) + `tsd`/`expectTypeOf` compile-fail test
raid_role: layer=seams; order=after:none; batch=seam
budget: active_rules=1; first_signal=tsc --noEmit (<60s)
```
