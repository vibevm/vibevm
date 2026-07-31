# CARD: scaffold-e-fast-loop — Per-Cell Fast Verification Loop {#root}

<status stage="spec" state="done"/>

##status-line **Discipline v0.2 · BETA** @impl/done

## Band 1 — Identity & Recognition {#band-one-identity}

##CLASSIFICATION Classification: layer=E (verification) + H (weak-reader); mechanism=scaffold E. @impl/done

##INTENT Intent: Guarantee every cell is independently compilable and testable in seconds, so an agent's edit→check→read-error→edit loop gets a first signal fast enough to steer — the substrate that makes every other scaffold's pass/fail usable. @impl/done

##ALSO-KNOWN-AS Also Known As: tight feedback loop; incremental check; per-package test; smoke loop; agent-computer interface. @spec/done

##APPLICABILITY-RECOGNITION Applicability / Recognition: Apply when — a cell cannot be checked without building the whole repo; the only verification is a multi-minute CI run; an agent must wait minutes for a signal. *Detector seed:* `cargo test -p <cell>` fails to run in isolation, OR cell has no fast local check → recognition fires (verification locality beats capability, R3-007). @impl/done

## Band 2 — Justification & Tradeoffs {#band-two-justification}

##MOTIVATION Motivation: A weak agent in a bounded loop gets one signal per 30-min CI run — useless. With `rust-ai-native fast-loop --cell <crate>` + `cargo test -p <cell>` returning in seconds, the same agent iterates ten times in the budget that previously bought zero feedback. The interpreter-budget result confirms: more local runs help agents that can use feedback (Opus, Sonnet); the loop is the amplifier. @spec/done

##STRUCTURE-AND-PARTICIPANTS Structure & Participants: *Cell isolation* (independently buildable package) · *Fast checker* (conform tiers + per-cell tests) · *Structured error* (Class F) · *Budget guard* (first signal < ~60s). @impl/done

##COLLABORATIONS Collaborations: Runs Classes C/D/G checks; consumes Class B's compiler checks; emits Class F diagnostics. The raid executor runs this per batch. @impl/done

##GOALS-AND-NON-GOALS Goals / Non-Goals: *Goals:* sub-minute first signal per cell; make the loop the standard agent workflow. *Non-Goals:* NOT replacing full CI (still runs at merge); does NOT create capability — Haiku didn't improve with more runs (it amplifies, not creates). @impl/done

##CONSEQUENCES Consequences: (+) every other scaffold becomes loop-usable; (+) iteration count rises within budget. (−) requires cells to be genuinely isolable (drives §1 cell design); (−) fast checks may miss what full CI catches — gate-mode checks backstop. @spec/done

##ALTERNATIVES Alternatives: whole-repo CI only (too slow for an agent loop); manual testing (not mechanical). Neither is a substitute. @spec/done

##RISKS-AND-ASSUMPTIONS Risks & Assumptions: assumes cells are isolable; god-files break this (anti-pattern). *Sunset:* none — the loop is foundational. @spec/done

##EVIDENCE-AND-TRANSFER-STRENGTH Evidence & Transfer-strength: R3-007 (verification locality, theory), R2C interpreter-budget result (more runs amplify capable agents, benchmark), BLD-010 (30–60s practical threshold). Class: benchmark + theory. Tag: **[E-strong]**. @spec/done

## Band 3 — Operation {#band-three-operation}

```card-ops
trigger: WHEN a cell lacks a sub-minute isolated check, OR verification requires a full-repo build THEN apply
mode: gate            # a structural precondition, verified at merge
routine:
  1. Make the cell an independently buildable unit (own package or test target).
  2. Provide `rust-ai-native conform check --scope <crate>` + `rust-ai-native fast-loop --cell <crate>` for it.
  3. Ensure `cargo test -p <cell>` runs in isolation, < ~60s.
  4. Wire the cell's Class C/D/G checks into that command.
  5. Confirm first-signal latency is within budget; if not, reduce test case counts.
checker: harness assertion `cell-fast-loop-present` (cell builds+tests in isolation under budget)
raid_role: layer=infrastructure; order=after:cell-boundaries; batch=cell
budget: active_rules=1; first_signal=<60s by definition
```
