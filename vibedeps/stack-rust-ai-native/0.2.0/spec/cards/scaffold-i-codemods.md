# CARD: scaffold-i-codemods — Scaffolded Edit Operations / Codemods
**Discipline v0.2 · BETA · [E-hyp] — validate before relying on it**

## Band 1 — Identity & Recognition
Classification: layer=H (weak-reader) + A (language-shape); mechanism=scaffold I.
Intent: Offer a capability-demanding multi-file change as ONE parameterized, checked operation — converting an edit a weak agent cannot safely coordinate into a parameter-filling task.
Also Known As: codemod; AST rewrite; refactoring script; scripted migration; semantic patch.
Applicability / Recognition: Apply when — a common change touches many files atomically (add a cell, register a variant, rename across a trait surface); the edit's size is itself the failure driver (Rust failures correlate with edit size/file count, R2C-006); the weakest swarm tier cannot coordinate it by hand. *Detector seed:* a recurring change-type that reliably requires touching >1 file in lockstep → recognition fires.

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent asked to "rename this seam across its 7 call-sites + the registry + the error enum" desynchronizes them. `vibe codemod rename-seam --from X --to Y` performs the change atomically and verifiably; the agent fills two parameters instead of coordinating seven edits. This mirrors how constrained decoding lifts weak models (DR1-015): collapse the hard task into a constrained, parameterized one.
Structure & Participants: *Codemod* (`syn`-based AST rewrite or cargo-integrated operation) · *Parameters* (the small named inputs) · *Atomic application* (all-or-nothing) · *Post-check* (compiles + tests green).
Collaborations: Implements bulk application of Classes A/B/G in raids; emits Class F diagnostics on failure; the Class D oracle wraps it when it changes behavior.
Goals / Non-Goals: *Goals:* convert capability-demanding multi-file edits into parameterized operations for the weak swarm. *Non-Goals:* NOT a general refactoring IDE; NOT for one-off changes; NOT a production compiler.
Consequences: (+) the weakest tier can perform edits otherwise beyond it; (+) atomicity kills desync and phantom diffs. (−) codemods are code to maintain and test; (−) **[E-hyp] risk:** parameterizing a codemod may itself exceed the weakest models — the very build/use boundary in question.
Alternatives: hand-editing (fails at scale for weak agents); a generator (Class A) when the artifact is derivable rather than transformed. Codemods are for TRANSFORMING existing code.
Risks & Assumptions: **assumes weak agents can correctly parameterize the operation** — UNVALIDATED; this is the prime pilot (R4) question. If false, restrict the weakest tier to fixed-parameter invocations only. *Sunset:* if language/tooling makes the change trivial, the codemod retires.
Evidence & Transfer-strength: first-principles from R3-013 (ownership graph bounds throughput) + R2C-006 (edit size drives Rust failure) + DL1-015 (constraints lift weak models). NOT in the follow-up. Class: theory. Tag: **[E-hyp]**.

## Band 3 — Operation
```card-ops
trigger: WHEN a recurring change-type reliably requires >1 file edited in lockstep THEN apply
mode: raid            # bulk application; also offered as an on-demand command
routine:
  1. Identify the recurring multi-file change and its minimal parameters.
  2. Implement a syn-based / cargo-integrated codemod performing it atomically.
  3. Add a post-check: result compiles and per-cell tests pass.
  4. Wrap behavior-changing codemods in a Class-D oracle.
  5. For the weakest tier, expose ONLY fixed-parameter invocations (no free parameterization) until R4 validates parameterization.
checker: the codemod's own post-check (compile + cargo test) ; conform `multi-file-change-has-codemod` (advisory, WISH until pilot-validated)
raid_role: layer=any; order=wraps-with:differential-oracle; batch=crate
budget: active_rules=1; first_signal=codemod post-check (<60s/crate)
```
