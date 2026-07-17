# CARD: scaffold-i-codemods — Scaffolded Edit Operations / Codemods (Go)
**Discipline v0.2 · BETA · T2 · Go · [E-hyp] — validate before relying on it**

## Band 1 — Identity & Recognition
Classification: layer=H (weak-reader) + A (language-shape); mechanism=scaffold I.
Intent: Offer a capability-demanding multi-file change as ONE parameterized, checked operation — converting an edit a weak agent cannot safely coordinate into a parameter-filling task. Go's tooling sits mid-way between Rust's (immature) and TypeScript's (rich): `gofmt -r` ships pattern rewrites in the toolchain itself; `go/ast` + `go/format` make structural codemods a stdlib exercise; `golang.org/x/tools/go/analysis` offers the framework tier.
Also Known As: codemod; AST rewrite; `gofmt -r`; refactoring script; scripted migration; analysis-with-fix.
Applicability / Recognition: Apply when — a common change touches many files atomically (add a cell, register a variant, rename across a seam); the edit's size is itself the failure driver (R2C-006); the weakest swarm tier cannot coordinate it by hand. *Detector seed:* a recurring change-type that reliably requires touching >1 file in lockstep → recognition fires.

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent asked to "add a planner variant" must create the cell package, the conformance assertion, the directive tags, the registry arm, and the Example stub — five files in lockstep. `go-ai-native codemod add-cell <pkg> <cell> <seam> <variant> <spec-uri>` performs the change atomically and verifiably; the agent fills five parameters instead of coordinating five files. This mirrors how constrained decoding lifts weak models (DR1-015): collapse the hard task into a constrained, parameterized one.
Structure & Participants: *Codemod* (`gofmt -r` for pattern rewrites; a `go/ast`+`go/format` program or the shipped CLI verb for structural ones) · *Parameters* (the small named inputs) · *Atomic application* (all-or-nothing) · *Post-check* (`go build` + per-package `go test` green).
Collaborations: Implements bulk application of Classes A/B/G in raids; emits Class F diagnostics on failure; the Class D oracle wraps it when it changes behavior.
Goals / Non-Goals: *Goals:* convert capability-demanding multi-file edits into parameterized operations for the weak swarm. *Non-Goals:* NOT a general refactoring IDE; NOT for one-off changes; NOT semantic transforms without a Class-D wrap.
Consequences: (+) the weakest tier can perform edits otherwise beyond it; (+) atomicity kills desync and phantom diffs; (+) `gofmt -r` gives the simplest rewrites for free, and `go/format` output is canonical by construction (no formatting drift). (−) codemods are code to maintain and test; (−) **[E-hyp] risk:** parameterizing a codemod may itself exceed the weakest models — the very build/use boundary in question.
Alternatives: hand-editing (fails at scale for weak agents); a generator (Class A) when the artifact is derivable rather than transformed. Codemods are for TRANSFORMING existing code.
Risks & Assumptions: **assumes weak agents can correctly parameterize the operation** — UNVALIDATED; this is the prime pilot question. If false, restrict the weakest tier to fixed-parameter invocations only. Go's tooling maturity removes the build-side risk for simple rewrites (`gofmt -r` is toolchain-native); complex semantic codemods remain authored one-offs. *Sunset:* if language/tooling makes the change trivial, the codemod retires.
Evidence & Transfer-strength: first-principles from R3-013 (ownership graph bounds throughput) + R2C-006 (edit size drives failure) + DR1-015 (constraints lift weak models). Class: theory. Tag: **[E-hyp]**.

## Band 3 — Operation
```card-ops
trigger: WHEN a recurring change-type reliably requires >1 file edited in lockstep THEN apply
mode: raid            # bulk application; also offered as an on-demand command
routine:
  1. Identify the recurring multi-file change and its minimal parameters.
  2. Prefer `gofmt -r 'old -> new'` where the change is a pure expression rewrite; else implement a go/ast + go/format codemod (or use the shipped `go-ai-native codemod add-cell`).
  3. Add a post-check: result builds (`go build ./...`) and the touched packages' tests pass.
  4. Wrap behavior-changing codemods in a Class-D oracle.
  5. For the weakest tier, expose ONLY fixed-parameter invocations until the pilot validates free parameterization.
checker: the codemod's own post-check (go build + go test) ; conform `multi-file-change-has-codemod` (advisory, WISH until pilot-validated)
raid_role: layer=any; order=wraps-with:differential-oracle; batch=package
budget: active_rules=1; first_signal=codemod post-check (<60s/package)
```
