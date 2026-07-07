# CARD: scaffold-b-typed-builders — Typed Builders / Typestate
**Discipline v0.2 · BETA**

## Band 1 — Identity & Recognition
Classification: layer=E (verification); mechanism=scaffold B.
Intent: Make the statistically-likely wrong call un-representable, so a hallucinated edit fails `cargo check` before runtime — encoding protocol correctness in types rather than docstrings.
Also Known As: typestate; phantom types; type-state builder; sealed trait; newtype wrapper; make-illegal-states-unrepresentable.
Applicability / Recognition: Apply when — a seam has a usage protocol (order of calls, required fields, valid states); a primitive (`u64`, `String`, `bool`) crosses a boundary where its meaning matters; an API takes multiple same-typed args or a bool flag. *Detector seed:* a pub seam fn taking `&str`/`bool`/multiple `u*` of the same type, OR a runtime check that a struct is "ready" → recognition fires (94% of compile errors are type-level; move the check there).

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent calls `connect(host, port, true, false)` and swaps the bools. With `ConnectionBuilder` requiring `.tls(Tls::Enabled)` and `.host(Host::new(...))`, the swap does not type-check; the error surfaces in the loop, not in production.
Structure & Participants: *Newtype* (primitive + meaning) · *Typestate marker* (phantom state) · *Builder* (type-mandatory required fields) · *Sealed trait* (closed extension).
Collaborations: Shrinks the input space Class D oracles must cover; the compiler is the Class E loop's primary checker; pairs with Class C for runtime invariants types can't express.
Goals / Non-Goals: *Goals:* convert probable hallucinations to compile errors at seams. *Non-Goals:* NOT typestate everywhere (ergonomic cost) — scope to seam surfaces; NOT a replacement for contracts on value-range invariants.
Consequences: (+) a whole class of misuse becomes uncompilable; (+) the type IS the protocol doc. (−) typestate ergonomics cost for human contributors; (−) over-typing fights idiom — scope tightly.
Alternatives: runtime validation (errors surface late — in production, not the loop); a contract (Class C) when the invariant is a value property, not a protocol.
Risks & Assumptions: assumes the protocol is type-expressible; some invariants need Class C. *Sunset:* none material. Strong models may be mildly distorted by over-constraint — keep newtype/typestate proportional.
Evidence & Transfer-strength: R3-008 (misuse-resistance, theory), DR2-012/R2C-005 (94% type-level errors; type-awareness cuts compile errors, benchmark). Class: benchmark + theory. Tag: **[E-mid]**.

## Band 3 — Operation
```card-ops
trigger: WHEN a pub seam fn takes &str/bool/duplicate-same-type args, OR a runtime "is-ready" check exists THEN apply
mode: gate            # introduced at seam design; checked at merge
routine:
  1. Wrap each meaning-bearing primitive at the seam in a newtype.
  2. Encode call-order/required-field protocol as typestate or a type-mandatory builder.
  3. Seal extension traits; add #[must_use] where ignoring the result is a defect.
  4. Delete the now-impossible runtime validity checks.
  5. Confirm the previously-wrong call no longer compiles (add a trybuild ui test).
checker: conform T-sem `seam-protocol-typed` + trybuild compile-fail test
raid_role: layer=seams; order=after:none; batch=seam
budget: active_rules=1; first_signal=cargo check (<60s)
```
