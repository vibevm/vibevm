# CARD: scaffold-b-typed-builders — Typed Surfaces / Defined Types / Constructors (Go)
**Discipline v0.2 · BETA · T2 · Go**

## Band 1 — Identity & Recognition
Classification: layer=E (verification); mechanism=scaffold B.
Intent: Make the statistically-likely wrong call un-representable, so a hallucinated edit fails `go build` before runtime — encoding identity and protocol in types and constructors rather than docstrings. The Go twist: **defined types are nominal for free** (`type AccountID string` does not interchange with `string` or a same-shaped sibling) — the identity safety TS must brand away by hand and Rust buys with newtypes is a one-line declaration here; the discipline's job is to make writing that line the reflex at every seam.
Also Known As: defined type; named type; newtype (Go form); functional options; staged builder; constructor-enforced invariants; loud conformance assertion.
Applicability / Recognition: Apply when — a seam has a usage protocol (required fields, valid states, call order); a meaning-bearing primitive (`string`, `int64`, `bool`) crosses a boundary where its identity matters; an API takes multiple same-typed args or a boolean flag; a struct can be constructed half-initialized via a literal. *Detector seed:* a pub seam func taking bare `string`/`bool`/duplicate-same-type args, OR an exported struct whose zero value is invalid but constructible, OR a runtime "is-ready" check → recognition fires (~94% of compile errors are type-level; move the check there).

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent calls `transfer(from, to string, amount int64)` and swaps the accounts — same shape, `go build` is silent, money moves the wrong way. With `type AccountID string` on the seam, the swap of an `AccountID` for an `OrderID` (or a bare `string`) fails the build; with unexported fields + `New(...)` as the only construction path, the half-initialized literal is impossible outside the package.
Structure & Participants: *Defined type* (primitive + meaning, nominal by language rule) · *Constructor* (`New` validates; unexported fields make it the only path) · *Functional options* (optional knobs without boolean soups) · *Staged builder* (call-order protocols, rare) · *Conformance assertion* (`var _ Seam = (*Impl)(nil)` — structural typing made loud, guide §2).
Collaborations: Shrinks the input space Class D oracles must cover; `go build` is the Class E loop's primary checker; pairs with Class C for value-range invariants types can't express; boundary DTO validation (guide §1) PRODUCES defined types at the erasure of the wire.
Goals / Non-Goals: *Goals:* convert probable hallucinations — identity swaps, missing required fields, invalid states — into compile errors at seams. *Non-Goals:* NOT defining a type for every local primitive (ergonomic cost) — scope to seam surfaces; NOT phantom-generic typestate as a default (possible since generics, but out-of-culture — use only where a protocol genuinely demands compile-time ordering, and mark it); NOT a replacement for runtime validation of external data.
Consequences: (+) a whole class of misuse becomes uncompilable at zero runtime cost; (+) the type IS the protocol doc; (+) conversions are explicit (`AccountID(s)`), so the remaining risk sites are grep-able. (−) explicit conversions add ceremony at boundaries; (−) Go's implicit zero values mean an unexported-field struct still has a zero form INSIDE the package — constructors must be the internal habit too.
Alternatives: runtime validation (errors surface late — in production, not the loop); a contract (Class C) when the invariant is a value range, not identity/protocol; a bare type alias (`type X = string` — rejected: aliases are NOT nominal, they are the same type).
Risks & Assumptions: assumes the protocol/identity is type-expressible; zero-value traps remain for in-package literals. *Sunset:* none material.
Evidence & Transfer-strength: R3-008 (misuse-resistance, theory), DR2-012/R2C-005 (94% type-level errors; type-awareness cuts compile errors, benchmark). Class: benchmark + theory. Tag: **[E-mid]**.

## Band 3 — Operation
```card-ops
trigger: WHEN a pub seam func takes a bare string/bool/duplicate-same-type args, OR an exported struct with an invalid-but-constructible zero value, OR a runtime is-ready check exists THEN apply
mode: gate            # introduced at seam design; checked at merge
routine:
  1. Define a named type for each meaning-bearing primitive at the seam (`type AccountID string`).
  2. Unexport struct fields; make `New(...)` (validating) the only construction path; use functional options for optional knobs.
  3. Encode genuine call-order protocols as a staged builder; keep phantom-generic typestate exceptional and marked.
  4. Add the loud-conformance assertion `var _ Seam = (*Impl)(nil)` beside the impl.
  5. Delete the now-impossible runtime validity checks.
  6. Confirm the previously-wrong call no longer builds (a compile-fail asset under testdata exercised by the loop, or an Example showing the blessed path).
checker: conform `seam-protocol-typed` (bare meaning-bearing primitive at a seam; missing conformance assertion) + go build
raid_role: layer=seams; order=after:none; batch=seam
budget: active_rules=1; first_signal=go build (<60s)
```
