# CARD: scaffold-g-doctests — Executable Examples / Doctests {#root}

<status stage="spec" state="done"/>

@fact:status-line **Discipline v0.2 · BETA** @status:impl/done

## Band 1 — Identity & Recognition {#band-one-identity}

@fact:CLASSIFICATION Classification: layer=C (meta) + G (empirics); mechanism=scaffold G. @status:impl/done

@fact:INTENT Intent: Ship one compiled, runnable example per public seam showing the ONE canonical way to use it — a few-shot usage signal that cannot drift into a lie because it must compile and pass. @status:impl/done

@fact:ALSO-KNOWN-AS Also Known As: doctest; usage example; example-driven docs; golden usage; runnable spec-by-example. @status:spec/done

@fact:APPLICABILITY-RECOGNITION Applicability / Recognition: Apply when — a public seam has no compiled example of canonical use; usage is documented only in prose; multiple usage idioms coexist with no canonical one. *Detector seed:* a `pub` seam item with no doctest demonstrating construction+use → recognition fires (the reference-library result, R2C-008; doctests are the executable half of "primitives + notes"). @status:impl/done

## Band 2 — Justification & Tradeoffs {#band-two-justification}

@fact:MOTIVATION Motivation: A weak agent imitates whatever usage it sees nearby (R3-006). If the nearest example is a prose snippet that has drifted, it imitates a lie. A doctest that lies fails CI, so the imitated signal is guaranteed truthful — and it shows the single canonical idiom, suppressing idiom-divergence. @status:spec/done

@fact:STRUCTURE-AND-PARTICIPANTS Structure & Participants: *Doctest* (compiled example in the item's docs) · *examples/ cell* (larger compiled scenario) · *canonical idiom* (the one blessed usage). @status:impl/done

@fact:COLLABORATIONS Collaborations: Encodes the canonical idiom Class B's types enforce; runs in the Class E loop; its truthfulness backs §8 prose discipline. @status:impl/done

@fact:GOALS-AND-NON-GOALS Goals / Non-Goals: *Goals:* every public seam carries ≥1 compiled doctest of canonical use. *Non-Goals:* NOT exhaustive examples (one canonical each); NOT a replacement for property tests (examples show usage, tests check behavior). @status:impl/done

@fact:CONSEQUENCES Consequences: (+) the imitated few-shot signal cannot lie; (+) one canonical idiom suppresses divergence. (−) doctests are code to maintain; (−) over-exampling bloats — one canonical per seam. @status:spec/done

@fact:ALTERNATIVES Alternatives: prose examples (drift silently); separate example files (fine, but doctests sit at the point of use). Prefer compiled, co-located. @status:spec/done

@fact:RISKS-AND-ASSUMPTIONS Risks & Assumptions: assumes the seam has a canonical usage worth blessing. *Sunset:* none material. @status:spec/done

@fact:EVIDENCE-AND-TRANSFER-STRENGTH Evidence & Transfer-strength: R2C-008 (executable reference material transformative, benchmark), R3-006 (codebase as few-shot prompt, theory), H4 (lying prose harms). Class: benchmark + theory. Tag: **[E-strong]**. @status:spec/done

## Band 3 — Operation {#band-three-operation}

```card-ops
trigger: WHEN a pub seam item lacks a compiled doctest of canonical construction+use THEN apply
mode: gate
routine:
  1. Write the single canonical usage as a doctest on the seam item.
  2. Ensure it constructs via the blessed path (Class B builder) and uses the seam.
  3. For larger scenarios, add an examples/ cell that compiles in CI.
  4. Confirm the doctest runs green in the per-cell loop.
checker: conform T-syn `seam-has-doctest` + `cargo test --doc`
raid_role: layer=cells; order=after:typed-builders; batch=seam
budget: active_rules=1; first_signal=cargo test --doc (<60s)
```
