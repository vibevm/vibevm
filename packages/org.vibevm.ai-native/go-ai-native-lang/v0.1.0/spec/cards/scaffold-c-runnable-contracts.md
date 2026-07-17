# CARD: scaffold-c-runnable-contracts — Runnable Contracts (Go)
**Discipline v0.2 · BETA · T2 · Go**

## Band 1 — Identity & Recognition
Classification: layer=E (verification); mechanism=scaffold C.
Intent: Express pre/post-conditions and invariants as EXECUTING checks attached to the unit and restated at use sites — so a paged reader gets ground truth without simulating the body. Go has no `debug_assert!` and no `asserts`-narrowing; the projection is an explicit `invariant` helper (panicking — the invariant-violation channel IS panic, guide §5) plus property tests backing behavioral claims.
Also Known As: design by contract; invariant check; precondition; `must`-helper; property test; `testing/quick`; fuzz property.
Applicability / Recognition: Apply when — a function has a non-obvious precondition; a cross-cell invariant is relied upon far from where it is established (R3-009); a behavioral claim is currently only in a godoc comment. *Detector seed:* a comment asserting a property ("already deduplicated", "sorted by name", "non-nil after init") with no adjacent runnable check → recognition fires (a prose claim is adversarial if it lies, R2C-004).

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent edits a planner relying on "actions are sorted by resource id" stated only in a godoc three packages away. It cannot do the whole-program inference to confirm. An `invariant(slices.IsSortedFunc(actions, byID), "actions sorted by id — spec://…#req-plan-order")` at the use site either holds or panics in the loop — the invariant is local ground truth.
Structure & Participants: *Invariant helper* (a two-line `func invariant(cond bool, msg string)` that panics — cheap, greppable, uniform) · *Use-site witness* (the restated check where the invariant is relied upon) · *Property test* (`testing/quick` for simple laws; a fuzz target for parser-shaped ones) · *Boundary validator* (explicit checks on decoded DTOs, guide §1).
Collaborations: Defines "valid"/"equivalent" for Class D oracles; pairs with Class B (types for identity/protocol, contracts for value invariants); failures speak the Class F grammar (the message carries the REQ URI).
Goals / Non-Goals: *Goals:* make load-bearing invariants machine-checked at the point of reliance. *Non-Goals:* NOT checking everything everywhere (hot-path cost is real and Go has no compiled-out assert tier — scope to seams and load-bearing sites; a `//go:build debug`-tagged variant is legal where a check is truly hot); NOT a substitute for the spec that JUSTIFIES the invariant; NOT input validation of untrusted data (that is the boundary's parse step).
Consequences: (+) invariants become local and checkable; (+) a paged reader trusts the check, not distant prose; (+) the panic message cites the REQ (Class F), so a trip is navigable. (−) restatements can drift — keep them checks (which fail, not mislead); (−) always-on runtime cost — scope deliberately, tag-gate the hot ones.
Alternatives: a defined type/constructor (Class B) when the invariant is identity or construction-shape; a comment (rejected — lies silently); moving the check into the type is always preferred when possible ("restructure beats testify").
Risks & Assumptions: assumes the invariant is expressible as a cheap predicate. *Sunset:* if a type/constructor later encodes the invariant statically, the check retires there (and a lingering one is deviation debt).
Evidence & Transfer-strength: DR1-019 (contracts give success criterion without body, theory), R3-009 (use-site restatement, theory), R2C-004 (prose lies harm, benchmark). Class: benchmark + theory. Tag: **[E-mid]**.

## Band 3 — Operation
```card-ops
trigger: WHEN a prose property claim lacks an adjacent runnable check, OR a cross-cell invariant is used far from its definition THEN apply
mode: inline
routine:
  1. State the invariant as a boolean predicate.
  2. Add the `invariant(cond, "… — spec://<req-uri>")` check at the definition AND at each use site relying on it.
  3. Where the site is measured-hot, move the check behind a `//go:build debug` twin and record the deviation.
  4. Back behavioral claims with a testing/quick property or a fuzz target.
  5. Replace the prose claim with the check (or label it verified, linking the test).
checker: conform `invariant-witnessed` (flags declared invariants with unwitnessed use sites) + go test -run/-fuzz seeds
raid_role: layer=cells; order=after:naming-uniformity; batch=cell
budget: active_rules=2; first_signal=go test ./<cell>/ (<60s)
```
