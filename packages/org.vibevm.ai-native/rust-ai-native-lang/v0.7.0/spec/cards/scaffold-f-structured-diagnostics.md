# CARD: scaffold-f-structured-diagnostics — Structured, Requirement-Citing Diagnostics {#root}

<status stage="spec" state="done"/>

##status-line **Discipline v0.2 · BETA** @impl/done

## Band 1 — Identity & Recognition {#band-one-identity}

##CLASSIFICATION Classification: layer=E (verification) + C (meta); mechanism=scaffold F. @impl/done

##INTENT Intent: Engineer compiler/linter/checker output as agent input — stable, structured, citing the violated requirement and the fix surface — because error text is the highest-leverage prompt in the loop. @impl/done

##ALSO-KNOWN-AS Also Known As: actionable diagnostics; SARIF output; fix-it hints; structured errors; machine-readable lint. @spec/done

##APPLICABILITY-RECOGNITION Applicability / Recognition: Apply when — a custom check emits free-text; an error states what failed but not which REQ or where to fix; tool output is unstable across runs (line shifts, ordering). *Detector seed:* a `thiserror`/lint/conform message without a `spec://` REQ URI and a fix-surface hint → recognition fires (tool output is the agent's percept, R3-011). @impl/done

## Band 2 — Justification & Tradeoffs {#band-two-justification}

##MOTIVATION Motivation: A weak agent sees `Error: invalid configuration`. It guesses, burns iterations. With `violates REQ spec://config/r3: tls requires a cert path; fix surface: ConnectionBuilder.cert()`, the agent acts directly. The strong author's "what to do when this fails" is materialized in the message. @spec/done

##STRUCTURE-AND-PARTICIPANTS Structure & Participants: *Diagnostic* (structured record) · *REQ citation* (`spec://` URI) · *Fix-surface hint* (where/what) · *Stable format* (SARIF / fixed grammar). @impl/done

##COLLABORATIONS Collaborations: Carries failures from Classes C/D/E; feeds the agent loop's next prompt; in raids, structured diagnostics let the orchestrator triage misfires. @impl/done

##GOALS-AND-NON-GOALS Goals / Non-Goals: *Goals:* every custom check is agent-actionable. *Non-Goals:* NOT rewriting rustc's own diagnostics (already good); does NOT replace the contract that defines correctness. @impl/done

##CONSEQUENCES Consequences: (+) iterations-to-green drop, more for weaker models; (+) diagnostics double as a navigable requirement map. (−) message authoring cost; (−) verbosity vs token budget — keep a compact grammar. @spec/done

##ALTERNATIVES Alternatives: free-text errors (wasted conditioning); silent failure (worst). Neither acceptable for an agent loop. @spec/done

##RISKS-AND-ASSUMPTIONS Risks & Assumptions: assumes a stable REQ namespace exists (it does — specmap). *Sunset:* none material. @spec/done

##EVIDENCE-AND-TRANSFER-STRENGTH Evidence & Transfer-strength: R3-011 (tool output is highest-leverage prompt, theory), R2C-004 (agent conditions on tool text, benchmark). Class: benchmark + theory. Tag: **[E-mid]**. @spec/done

## Band 3 — Operation {#band-three-operation}

```card-ops
trigger: WHEN a custom check/error message lacks a spec:// REQ URI + fix-surface hint THEN apply
mode: inline
routine:
  1. Add the violated REQ's spec:// URI to the message.
  2. Add a one-line fix surface: where to change and what.
  3. Emit in a stable structured form (SARIF for conform; fixed grammar for thiserror).
  4. Keep it compact (one line of why + one of where).
checker: conform T-lex `diagnostic-cites-req` (custom messages must match the grammar `violates REQ <uri>: <why>; fix surface: <where>`)
raid_role: layer=tooling; order=after:none; batch=crate
budget: active_rules=1; first_signal=lint pass (<60s)
```
