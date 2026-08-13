# CARD: scaffold-c-runnable-contracts — Runnable Contracts {#root}

<status stage="spec" state="done"/>

@fact:status-line **Discipline v0.2 · BETA** @status:impl/done

## Band 1 — Identity & Recognition {#band-one-identity}

@fact:CLASSIFICATION Classification: layer=E (verification); mechanism=scaffold C. @status:impl/done

@fact:INTENT Intent: Express pre/post-conditions and invariants as EXECUTING assertions or proofs attached to the unit and restated at use sites — so a paged reader gets ground truth without simulating the body. @status:impl/done

@fact:ALSO-KNOWN-AS Also Known As: design by contract; debug_assert; refinement; require/ensure; invariant check; Kani contract. @status:spec/done

@fact:APPLICABILITY-RECOGNITION Applicability / Recognition: Apply when — a function has a non-obvious precondition; a cross-cell invariant is relied upon far from where it is established (R3-009); a behavioral claim is currently only in prose. *Detector seed:* a comment asserting a property ("x is sorted", "non-empty", "already validated") with no adjacent runnable check → recognition fires (a prose claim is adversarial if it lies, R2C-004). @status:impl/done

## Band 2 — Justification & Tradeoffs {#band-two-justification}

@fact:MOTIVATION Motivation: A weak agent edits a function relying on "input already deduplicated" stated only in a doc-comment three modules away. It cannot do the whole-program inference to confirm. A `debug_assert!(is_unique(&input))` at the use site either holds or fires — the invariant is local ground truth. @status:spec/done

@fact:STRUCTURE-AND-PARTICIPANTS Structure & Participants: *Precondition/postcondition* (debug_assert or contract macro) · *Use-site witness* (assertion where the invariant is relied upon) · *Proof* (Kani requires/ensures, for safety-critical) · *Property test* (behavioral backing). @status:impl/done

@fact:COLLABORATIONS Collaborations: Defines "equivalent" for Class D oracles; pairs with Class B (types for protocol, contracts for value invariants); failures emit Class F diagnostics. @status:impl/done

@fact:GOALS-AND-NON-GOALS Goals / Non-Goals: *Goals:* make load-bearing invariants machine-checked at the point of reliance. *Non-Goals:* NOT proving everything (cost) — proofs for safety-critical, asserts elsewhere; NOT a substitute for the spec that JUSTIFIES the invariant. @status:impl/done

@fact:CONSEQUENCES Consequences: (+) invariants become local and checkable; (+) a paged reader trusts the assertion, not distant prose. (−) restatements can drift — pair with drift detection or keep them assertions (which fail, not mislead); (−) debug_assert is debug-only — use real checks on untrusted boundaries. @status:spec/done

@fact:ALTERNATIVES Alternatives: types (Class B) when the invariant is a protocol; a comment (rejected — lies silently); full proof when tractable and critical. @status:spec/done

@fact:RISKS-AND-ASSUMPTIONS Risks & Assumptions: assumes the invariant is expressible as a runnable predicate. *Sunset:* if types or vibe-tcg make the invariant statically guaranteed, the assert retires there. @status:spec/done

@fact:EVIDENCE-AND-TRANSFER-STRENGTH Evidence & Transfer-strength: DR1-019 (contracts give success criterion without body, theory), R3-009 (use-site restatement, theory), R2C-004 (prose lies harm, benchmark). Class: benchmark + theory. Tag: **[E-mid]**. @status:spec/done

## Band 3 — Operation {#band-three-operation}

```card-ops
trigger: WHEN a prose property claim lacks an adjacent runnable check, OR a cross-cell invariant is used far from its definition THEN apply
mode: inline
routine:
  1. State the invariant as a boolean predicate.
  2. Add debug_assert!/contract at the definition AND at each use site relying on it.
  3. For safety-critical invariants, add a Kani requires/ensures proof.
  4. Back behavioral claims with a property test.
  5. Replace the prose claim with the assertion (or label it verified, linking the check).
checker: conform T-sem `invariant-witnessed` (flags declared invariants with unwitnessed use sites) + cargo test
raid_role: layer=cells; order=after:naming-uniformity; batch=cell
budget: active_rules=2; first_signal=cargo test -p <cell> (<60s)
```
