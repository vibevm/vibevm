# CARD: scaffold-g-doctests — Executable Examples (Go)
**Discipline v0.2 · BETA · T2 · Go**

## Band 1 — Identity & Recognition
Classification: layer=C (meta) + G (empirics); mechanism=scaffold G.
Intent: Ship one compiled, RUNNING example per public seam showing the ONE canonical way to use it — a few-shot usage signal that cannot drift into a lie. Go's `Example` functions are the strongest doctest form of the three stacks: `go test` compiles them AND executes them, diffing stdout against the `// Output:` comment — a behavioral guarantee, not just compilation.
Also Known As: doctest; Example function; `// Output:`; usage example; golden usage; runnable spec-by-example.
Applicability / Recognition: Apply when — a public seam has no `Example` of canonical use; usage is documented only in godoc prose; multiple usage idioms coexist with no canonical one. *Detector seed:* an exported seam item with no `ExampleXxx` demonstrating construction+use → recognition fires (the reference-library result, R2C-008; examples are the executable half of "primitives + notes").

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent imitates whatever usage it sees nearby (R3-006). If the nearest example is a godoc snippet that has drifted, it imitates a lie. An `Example` with `// Output:` that lies FAILS `go test` — the imitated signal is guaranteed truthful at the behavioral level — and it shows the single canonical idiom (construct via `New`, consume the seam, handle the closed error set), suppressing idiom divergence. godoc renders Examples beside the item, so the carrier doubles as the doc surface.
Structure & Participants: *`ExampleXxx` function* (in `example_test.go`, package `foo_test` — exercising the PUBLIC surface only) · *`// Output:` comment* (the executed assertion; `// Unordered output:` where order is not contractual) · *canonical idiom* (the one blessed usage).
Collaborations: Encodes the canonical idiom Class B's types/constructors enforce; runs in the Class E loop; its truthfulness backs §9 prose discipline; the seam-error Example doubles as a Class-F navigability demo (the printed message cites its REQ).
Goals / Non-Goals: *Goals:* every public seam item carries ≥1 Example of canonical construction+use, with `// Output:` wherever output is deterministic. *Non-Goals:* NOT exhaustive examples (one canonical each); NOT a replacement for tests (examples show usage; tests check behavior matrices); NOT Examples for unexported helpers.
Consequences: (+) the imitated few-shot signal cannot lie — it executes; (+) one canonical idiom suppresses divergence; (+) godoc renders it (unlike the hidden `//spec:` directives — the two carriers split human-doc and machine-trace duties). (−) Examples are code to maintain; (−) output-free Examples (no `// Output:`) only compile — prefer deterministic output or a wrapping assertion; (−) over-exampling bloats — one canonical per seam.
Alternatives: godoc prose snippets (drift silently); separate example programs under `examples/` (fine for larger scenarios, still built in CI). Prefer executed, co-located.
Risks & Assumptions: assumes the seam has a canonical usage worth blessing and deterministic-enough output. *Sunset:* none material.
Evidence & Transfer-strength: R2C-008 (executable reference material transformative, benchmark), R3-006 (codebase as few-shot prompt, theory), H4 (lying prose harms). Class: benchmark + theory. Tag: **[E-strong]**.

## Band 3 — Operation
```card-ops
trigger: WHEN an exported seam item lacks an Example of canonical construction+use THEN apply
mode: gate
routine:
  1. Write the single canonical usage as ExampleXxx in example_test.go (package foo_test — public surface only).
  2. Construct via the blessed path (New + options, Class B) and consume the seam.
  3. End with deterministic output and the // Output: comment (or // Unordered output:).
  4. For seam error types, print the rendered error — the REQ-citing message becomes the executed example.
  5. Confirm the Example runs green in the per-package loop.
checker: health collector `example_coverage` census (specified as a gate: conform `seam-has-example`) + go test
raid_role: layer=cells; order=after:typed-builders; batch=seam
budget: active_rules=1; first_signal=go test ./<cell>/ (<60s)
```
