# CARD: scaffold-f-structured-diagnostics — Structured, Requirement-Citing Diagnostics (Go)
**Discipline v0.2 · BETA · T2 · Go**

## Band 1 — Identity & Recognition
Classification: layer=E (verification) + C (meta); mechanism=scaffold F.
Intent: Engineer checker/error output as agent input — stable, structured, citing the violated requirement and the fix surface — because error text is the highest-leverage prompt in the loop.
Also Known As: actionable diagnostics; SARIF output; fix-it hints; structured errors; REQ-citing error values.
Applicability / Recognition: Apply when — a seam error or custom check emits free text; an error states what failed but not which REQ or where to fix; tool output is unstable across runs. *Detector seed:* a seam error type whose `Error()` lacks a `spec://` REQ URI, or a custom check message without the fix-surface hint → recognition fires (tool output is the agent's percept, R3-011). Note: `go vet`/staticcheck output is already coded and stable — wrap with REQ context where a Discipline rule cites them; do not reimplement.

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent sees `plan failed: conflict`. It guesses, burns iterations. With `plan: ErrConflict: violates REQ spec://go-demo/PROP-001-reconciler#req-plan-total; fix surface: resolve desired/actual disagreement in Planner.Plan or extend the divergence list`, the agent acts directly. The strong author's "what to do when this fails" is materialized in the message.
Structure & Participants: *Error value* (the seam's closed-set struct: `Code + Spec + Err`, guide §5) · *REQ citation* (`spec://` URI in `Error()`) · *Fix-surface hint* (where/what) · *Stable format* (the fixed grammar `violates REQ <uri>: <why>; fix surface: <where>`; SARIF for conform findings).
Collaborations: Carries failures from Classes C/D/E and the §7 ban census; feeds the agent loop's next prompt; in raids, structured diagnostics let the orchestrator triage misfires. The `Unwrap` chain (`%w`) keeps causes machine-walkable — chain hygiene is part of this card's surface.
Goals / Non-Goals: *Goals:* every seam error and custom check is agent-actionable. *Non-Goals:* NOT rewriting the toolchain's own diagnostics (vet/staticcheck are already good — wrap them); does NOT replace the contract that defines correctness.
Consequences: (+) iterations-to-green drop, more for weaker models; (+) diagnostics double as a navigable requirement map (the error IS a spec pointer). (−) message authoring cost; (−) verbosity vs token budget — keep the grammar compact (one line of why + one of where).
Alternatives: free-text errors (wasted conditioning); silent failure (worst). Neither acceptable for an agent loop.
Risks & Assumptions: assumes a stable REQ namespace exists (it does — specmap, guide §8). *Sunset:* none material.
Evidence & Transfer-strength: R3-011 (tool output is highest-leverage prompt, theory), R2C-004 (agent conditions on tool text, benchmark). Class: benchmark + theory. Tag: **[E-mid]**.

## Band 3 — Operation
```card-ops
trigger: WHEN a seam error's Error() or a custom check message lacks a spec:// REQ URI + fix-surface hint THEN apply
mode: inline
routine:
  1. Put the violated REQ's spec:// URI in the error value (the `Spec` field) and render it in Error().
  2. Add a one-line fix surface at the boundary rendering: where to change and what.
  3. Wrap causes with %w so the chain stays machine-walkable.
  4. Emit custom-check findings in the fixed grammar (SARIF for conform).
  5. Keep it compact (one line of why + one of where).
checker: conform `seam-error-cites-req` (seam error types must carry Spec + the grammar) — shipped, go-ai-native-conform
raid_role: layer=tooling; order=after:none; batch=package
budget: active_rules=1; first_signal=conform check (<60s)
```
