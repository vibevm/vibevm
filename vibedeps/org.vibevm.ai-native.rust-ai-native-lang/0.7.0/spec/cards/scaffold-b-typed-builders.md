# CARD: scaffold-b-typed-builders — Typed Builders / Typestate {#root}

<status stage="spec" state="done"/>

@fact:status-line **Discipline v0.2 · BETA** @status:impl/done

## Band 1 — Identity & Recognition {#band-one-identity}

@fact:CLASSIFICATION Classification: layer=E (verification); mechanism=scaffold B. @status:impl/done

@fact:INTENT Intent: Make the statistically-likely wrong call un-representable, so a hallucinated edit fails `cargo check` before runtime — encoding protocol correctness in types rather than docstrings. @status:impl/done

@fact:ALSO-KNOWN-AS Also Known As: typestate; phantom types; type-state builder; sealed trait; newtype wrapper; make-illegal-states-unrepresentable. @status:spec/done

@fact:APPLICABILITY-RECOGNITION Applicability / Recognition: Apply when — a seam has a usage protocol (order of calls, required fields, valid states); a primitive (`u64`, `String`, `bool`) crosses a boundary where its meaning matters; an API takes multiple same-typed args or a bool flag. *Detector seed:* a pub seam fn taking `&str`/`bool`/multiple `u*` of the same type, OR a runtime check that a struct is "ready" → recognition fires (94% of compile errors are type-level; move the check there). @status:impl/done

## Band 2 — Justification & Tradeoffs {#band-two-justification}

@fact:MOTIVATION Motivation: A weak agent calls `connect(host, port, true, false)` and swaps the bools. With `ConnectionBuilder` requiring `.tls(Tls::Enabled)` and `.host(Host::new(...))`, the swap does not type-check; the error surfaces in the loop, not in production. @status:spec/done

@fact:STRUCTURE-AND-PARTICIPANTS Structure & Participants: *Newtype* (primitive + meaning) · *Typestate marker* (phantom state) · *Builder* (type-mandatory required fields) · *Sealed trait* (closed extension). @status:impl/done

@fact:COLLABORATIONS Collaborations: Shrinks the input space Class D oracles must cover; the compiler is the Class E loop's primary checker; pairs with Class C for runtime invariants types can't express. @status:impl/done

@fact:GOALS-AND-NON-GOALS Goals / Non-Goals: *Goals:* convert probable hallucinations to compile errors at seams. *Non-Goals:* NOT typestate everywhere (ergonomic cost) — scope to seam surfaces; NOT a replacement for contracts on value-range invariants. @status:impl/done

@fact:CONSEQUENCES Consequences: (+) a whole class of misuse becomes uncompilable; (+) the type IS the protocol doc. (−) typestate ergonomics cost for human contributors; (−) over-typing fights idiom — scope tightly. @status:spec/done

@fact:ALTERNATIVES Alternatives: runtime validation (errors surface late — in production, not the loop); a contract (Class C) when the invariant is a value property, not a protocol. @status:spec/done

@fact:RISKS-AND-ASSUMPTIONS Risks & Assumptions: assumes the protocol is type-expressible; some invariants need Class C. *Sunset:* none material. Strong models may be mildly distorted by over-constraint — keep newtype/typestate proportional. @status:spec/done

@fact:EVIDENCE-AND-TRANSFER-STRENGTH Evidence & Transfer-strength: R3-008 (misuse-resistance, theory), DR2-012/R2C-005 (94% type-level errors; type-awareness cuts compile errors, benchmark). Class: benchmark + theory. Tag: **[E-mid]**. @status:spec/done

## Band 3 — Operation {#band-three-operation}

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
