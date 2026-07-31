# CARD: scaffold-e-fast-loop — Per-Cell Fast Verification Loop (TypeScript) {#root}

<status stage="spec" state="done"/>

##status-line **Discipline v0.2 · BETA · T2 · TypeScript** @impl/done

## Band 1 — Identity & Recognition {#band-one-identity}

##CLASSIFICATION Classification: layer=E (verification) + H (weak-reader); mechanism=scaffold E. @impl/done

##INTENT Intent: Guarantee every cell is independently type-checkable and testable in seconds (`tsc --noEmit -p <cell>` + `vitest run <cell>`), so an agent's edit→check→read-error→edit loop gets a first signal fast enough to steer — the substrate that makes every other scaffold's pass/fail usable. @impl/done

##ALSO-KNOWN-AS Also Known As: tight feedback loop; incremental check; project-reference isolation; per-project typecheck; watch mode; agent-computer interface. @spec/done

##APPLICABILITY-RECOGNITION Applicability / Recognition: Apply when — a cell cannot be checked without type-checking/building the whole repo; the only verification is a multi-minute CI run; an agent must wait minutes for a signal. *Detector seed:* `tsc --noEmit -p <cell>` or `vitest run <cell>` fails to run in isolation, OR the cell has no project reference / vitest project → recognition fires (verification locality beats capability, R3-007). @impl/done

## Band 2 — Justification & Tradeoffs {#band-two-justification}

##MOTIVATION Motivation: A weak agent in a bounded loop gets one signal per multi-minute CI run — useless. With `tsc --noEmit -p <cell>` + `vitest run <cell>` returning in seconds (sub-second under the TS 7 / "Corsa" native compiler, ~10× faster checking), the same agent iterates ten times in the budget that previously bought zero feedback. The interpreter-budget result confirms: more local runs help agents that can use feedback; the loop is the amplifier. @spec/done

##STRUCTURE-AND-PARTICIPANTS Structure & Participants: *Cell isolation* (a project reference / a `vitest` project) · *Fast checker* (`tsc --noEmit -p` + `vitest`) · *Structured error* (Class F) · *Budget guard* (first signal < ~60s). @impl/done

##COLLABORATIONS Collaborations: Runs Classes C/D/G checks and the type-level tests (guide §12); consumes Class B's compile checks; emits Class F diagnostics. The raid executor runs this per batch. @impl/done

##GOALS-AND-NON-GOALS Goals / Non-Goals: *Goals:* sub-minute first signal per cell; make the loop the standard agent workflow. *Non-Goals:* NOT replacing full CI (still runs at merge); does NOT create capability — scaffolds amplify, they do not create. @impl/done

##CONSEQUENCES Consequences: (+) every other scaffold becomes loop-usable; (+) iteration count rises within budget; the native Corsa compiler sharpens this further. (−) requires cells genuinely isolable (drives §3 cell design); barrel-file sprawl breaks project-reference isolation; (−) fast checks may miss what full CI catches — gate-mode checks backstop. @spec/done

##ALTERNATIVES Alternatives: whole-repo `tsc`/CI only (too slow for an agent loop); manual testing (not mechanical). Neither is a substitute. @spec/done

##RISKS-AND-ASSUMPTIONS Risks & Assumptions: assumes cells are isolable; god-modules and giant barrels break this (anti-pattern). *Sunset:* none — the loop is foundational. @spec/done

##EVIDENCE-AND-TRANSFER-STRENGTH Evidence & Transfer-strength: R3-007 (verification locality, theory), R2C interpreter-budget result (more runs amplify capable agents, benchmark), BLD-010 (30–60s practical threshold). Class: benchmark + theory. Tag: **[E-strong]**. @spec/done

## Band 3 — Operation {#band-three-operation}

```card-ops
trigger: WHEN a cell lacks a sub-minute isolated check, OR verification requires a full-repo typecheck/build THEN apply
mode: gate            # a structural precondition, verified at merge
routine:
  1. Make the cell an independently checkable unit (a project reference + a vitest project).
  2. Provide `tsc --noEmit -p <cell>` scope for it.
  3. Ensure `vitest run <cell>` runs in isolation, < ~60s.
  4. Wire the cell's Class C/D/G + type-level checks into that command.
  5. Confirm first-signal latency is within budget; if not, narrow the project or reduce test case counts.
checker: harness assertion `cell-fast-loop-present` (cell type-checks+tests in isolation under budget)
raid_role: layer=infrastructure; order=after:cell-boundaries; batch=cell
budget: active_rules=1; first_signal=<60s by definition
```
