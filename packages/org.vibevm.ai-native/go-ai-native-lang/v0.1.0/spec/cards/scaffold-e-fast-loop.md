# CARD: scaffold-e-fast-loop — Per-Cell Fast Verification Loop (Go)
**Discipline v0.2 · BETA · T2 · Go**

## Band 1 — Identity & Recognition
Classification: layer=E (verification) + H (weak-reader); mechanism=scaffold E.
Intent: Guarantee every cell is independently buildable and testable in seconds (`go test ./internal/cells/<name>/ -race`), so an agent's edit→check→read-error→edit loop gets a first signal fast enough to steer — the substrate that makes every other scaffold's pass/fail usable. Go hands this scaffold the strongest substrate of the three stacks: per-package testing needs no project references (TS) and no shared cold `target/` (Rust); the compiler was engineered for build speed as a headline feature.
Also Known As: tight feedback loop; per-package test; incremental check; smoke loop; agent-computer interface.
Applicability / Recognition: Apply when — a cell cannot be checked without building/testing the whole module set; the only verification is a multi-minute CI run; an agent must wait minutes for a signal. *Detector seed:* `go test ./<cell>/` fails to run in isolation (cross-package test coupling, missing fakes forcing integration setup), OR the cell has no tests at all → recognition fires (verification locality beats capability, R3-007).

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent in a bounded loop gets one signal per multi-minute CI run — useless. With `go test ./internal/cells/batchplanner/ -race` returning in ~1–5 s, the same agent iterates dozens of times in the budget that previously bought zero feedback. The interpreter-budget result confirms: more local runs help agents that can use feedback; the loop is the amplifier.
Structure & Participants: *Cell isolation* (a package with injected capabilities — no ambient setup) · *Fast checker* (`go vet` + per-package `go test -race`) · *Structured error* (Class F messages + Go's file:line panics) · *Budget guard* (first signal < ~60s; in practice far under).
Collaborations: Runs Classes C/D/G checks (invariants, fuzz seeds, Examples all live in the same per-package run); consumes Class B's compile checks; emits Class F diagnostics. The raid executor runs this per batch. Capability injection (§2) is what MAKES the isolation real — a cell touching ambient state needs integration scaffolding and falls out of the loop.
Goals / Non-Goals: *Goals:* sub-minute first signal per cell; make the loop the standard agent workflow. *Non-Goals:* NOT replacing full CI (module-wide `go test ./...` still runs at merge); does NOT create capability — scaffolds amplify, they do not create.
Consequences: (+) every other scaffold becomes loop-usable; (+) `-race` rides along at per-package cost, so the concurrency discipline (§5) is checked in the same loop. (−) requires cells genuinely isolable (drives §2 cell design); a cell with no tests has NO loop — that is a finding, not a neutral state.
Alternatives: whole-module CI only (too slow for an agent loop); manual testing (not mechanical). Neither is a substitute.
Risks & Assumptions: assumes cells are isolable; ambient coupling breaks this (anti-pattern). *Sunset:* none — the loop is foundational.
Evidence & Transfer-strength: R3-007 (verification locality, theory), R2C interpreter-budget result (more runs amplify capable agents, benchmark), BLD-010 (30–60s practical threshold). Class: benchmark + theory. Tag: **[E-strong]**.

## Band 3 — Operation
```card-ops
trigger: WHEN a cell lacks a sub-minute isolated check, OR has no tests, OR verification requires a whole-module run THEN apply
mode: gate            # a structural precondition, verified at merge
routine:
  1. Make the cell a self-contained package: capabilities injected, no ambient setup (§2).
  2. Ensure `go test ./<cell>/ -race` runs in isolation, < ~60s (in practice seconds).
  3. Wire the cell's Class C/D/G artifacts (invariants, fuzz seeds, Examples) into that same run.
  4. Give the loop a one-line entry: `go-ai-native fast-loop --cell <package>` (asserts the budget).
  5. Confirm first-signal latency is within budget; if not, cut corpus/case counts.
checker: `go-ai-native fast-loop` (cell tests in isolation under budget; a test-less cell fails)
raid_role: layer=infrastructure; order=after:cell-boundaries; batch=cell
budget: active_rules=1; first_signal=<60s by definition
```
