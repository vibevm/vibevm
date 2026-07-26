# CARD: scaffold-f-structured-diagnostics — Structured, Requirement-Citing Diagnostics (TypeScript) {#root}

<status stage="spec" state="done"/>

##status-line **Discipline v0.2 · BETA · T2 · TypeScript** @impl/done

## Band 1 — Identity & Recognition {#band-one-identity}

##CLASSIFICATION Classification: layer=E (verification) + C (meta); mechanism=scaffold F. @impl/done

##INTENT Intent: Engineer linter/checker output as agent input — stable, structured, citing the violated requirement and the fix surface — because error text is the highest-leverage prompt in the loop. @impl/done

##ALSO-KNOWN-AS Also Known As: actionable diagnostics; SARIF output; fix-it hints; structured errors; machine-readable lint; ESLint `messageId`. @spec/done

##APPLICABILITY-RECOGNITION Applicability / Recognition: Apply when — a custom `@typescript-eslint` rule (or a thrown domain error) emits free text; an error states what failed but not which REQ or where to fix; tool output is unstable across runs. *Detector seed:* a custom eslint/check message or a domain error without a `spec://` REQ URI and a fix-surface hint → recognition fires (tool output is the agent's percept, R3-011). Note: `tsc`'s own diagnostics (TS2322 etc.) are already coded — wrap them with REQ context, do not replace them. @impl/done

## Band 2 — Justification & Tradeoffs {#band-two-justification}

##MOTIVATION Motivation: A weak agent sees `Type 'string' is not assignable to type 'never'` from a `satisfies never` exhaustiveness check — true but opaque. With the custom rule emitting `violates REQ spec://errors/r6: error union not exhaustively handled; fix surface: add a case for variant 'Timeout' before assertNever`, the agent acts directly. The strong author's "what to do when this fails" is materialized in the message. @spec/done

##STRUCTURE-AND-PARTICIPANTS Structure & Participants: *Diagnostic* (an eslint report with a stable `messageId`) · *REQ citation* (`spec://` URI) · *Fix-surface hint* (where/what) · *Stable format* (SARIF / fixed grammar). @impl/done

##COLLABORATIONS Collaborations: Carries failures from Classes C/D/E and the §8 bans; feeds the agent loop's next prompt; in raids, structured diagnostics let the orchestrator triage misfires. @impl/done

##GOALS-AND-NON-GOALS Goals / Non-Goals: *Goals:* every custom check is agent-actionable. *Non-Goals:* NOT rewriting `tsc`'s own diagnostics (already good — wrap them); does NOT replace the contract that defines correctness. @impl/done

##CONSEQUENCES Consequences: (+) iterations-to-green drop, more for weaker models; (+) diagnostics double as a navigable requirement map. (−) message authoring cost; (−) verbosity vs token budget — keep a compact grammar. @spec/done

##ALTERNATIVES Alternatives: free-text errors (wasted conditioning); silent failure (worst). Neither acceptable for an agent loop. @spec/done

##RISKS-AND-ASSUMPTIONS Risks & Assumptions: assumes a stable REQ namespace exists (it does — specmap, guide §9). *Sunset:* none material. @spec/done

##EVIDENCE-AND-TRANSFER-STRENGTH Evidence & Transfer-strength: R3-011 (tool output is highest-leverage prompt, theory), R2C-004 (agent conditions on tool text, benchmark). Class: benchmark + theory. Tag: **[E-mid]**. @spec/done

## Band 3 — Operation {#band-three-operation}

```card-ops
trigger: WHEN a custom eslint/check message or a domain error lacks a spec:// REQ URI + fix-surface hint THEN apply
mode: inline
routine:
  1. Add the violated REQ's spec:// URI to the message.
  2. Add a one-line fix surface: where to change and what.
  3. Emit in a stable structured form (eslint `messageId` + SARIF; fixed grammar for domain errors).
  4. Keep it compact (one line of why + one of where).
checker: @typescript-eslint `diagnostic-cites-req` (custom messages must match the grammar `violates REQ <uri>: <why>; fix surface: <where>`)
raid_role: layer=tooling; order=after:none; batch=package
budget: active_rules=1; first_signal=lint pass (<60s)
```
