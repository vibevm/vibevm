# CARD: scaffold-h-simulators — Local Simulators / Reference Models
**Discipline v0.2 · BETA**

## Band 1 — Identity & Recognition
Classification: layer=E (verification) + H (weak-reader); mechanism=scaffold H.
Intent: Ship a small runnable model of a subsystem's behavior the reader can EXECUTE to understand or predict — offloading the execution-prediction that weak models fail at, without running the whole system.
Also Known As: reference implementation; in-memory fake; executable spec; oracle model; test double / simulator.
Applicability / Recognition: Apply when — a subsystem has non-obvious dynamics (a state machine, a protocol, a fixpoint); understanding requires mentally simulating execution; an external dependency must be reasoned about offline. *Detector seed:* a subsystem whose behavior is documented in prose-describing-execution, with no runnable model or fake → recognition fires (execution-prediction is weak models' weakest point — CRUXEval ~63% even for strong models).

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent must modify the resolver's conditional-dependency fixpoint. It cannot mentally simulate the solve→probe→add→re-solve convergence. A runnable reference model it can step through (feed inputs, watch the fixpoint converge) replaces mental simulation with execution — the EsoLang library shipped exactly this (a local Befunge simulator) and it carried the weak-agent gain.
Structure & Participants: *Reference model* (runnable, small) · *In-memory fake* (of external deps) · *Stepping interface* (inspect intermediate state).
Collaborations: Provides the comparator for Class D oracles; backs Class C contracts (the model defines expected behavior); pairs with Class G (the model's usage is doctested).
Goals / Non-Goals: *Goals:* make non-obvious dynamics executable, not just described. *Non-Goals:* NOT a second production implementation (a reference model, kept simple); NOT for trivially-obvious subsystems.
Consequences: (+) the reader runs instead of simulates; (+) doubles as a Class D comparator. (−) a model is code to keep in sync — drift detection or a conformance test against production; (−) over-modeling wastes effort — only non-obvious dynamics.
Alternatives: prose describing behavior (weak readers can't execute prose); reading the production code directly (the thing too complex to simulate). The model is the offload.
Risks & Assumptions: assumes the subsystem's behavior is modelable simply; a model that drifts from production misleads — conformance-test it. *Sunset:* if the production code becomes simple enough to read directly, the model retires.
Evidence & Transfer-strength: R2C-008 (simulator in the transformative library, benchmark), DR2-019 (execution-prediction weakness, benchmark). Class: benchmark. Tag: **[E-strong]**.

## Band 3 — Operation
```card-ops
trigger: WHEN a subsystem with non-obvious dynamics has no runnable reference model or fake THEN apply
mode: gate
routine:
  1. Identify the dynamics a reader must predict (states, transitions, fixpoint).
  2. Write a small runnable reference model with a stepping/inspection interface.
  3. Provide in-memory fakes for external dependencies of the subsystem.
  4. Add a conformance test: model vs production agree on representative inputs.
  5. Doctest the model's usage (Class G).
checker: conform T-sem `nonobvious-subsystem-has-model` + model-vs-production conformance test
raid_role: layer=cells; order=after:contracts; batch=cell
budget: active_rules=1; first_signal=conformance test (<60s)
```
