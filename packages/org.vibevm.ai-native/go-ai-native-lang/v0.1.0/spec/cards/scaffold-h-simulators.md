# CARD: scaffold-h-simulators — Local Simulators / Reference Models (Go)
**Discipline v0.2 · BETA · T2 · Go**

## Band 1 — Identity & Recognition
Classification: layer=E (verification) + H (weak-reader); mechanism=scaffold H.
Intent: Ship a small runnable model of a subsystem's behavior the reader can EXECUTE to understand or predict — offloading the execution-prediction that weak models fail at, without running the whole system. Go's test culture already lives here: small interfaces make hand-rolled in-memory fakes one-screen literals, and `httptest` is a stdlib network simulator.
Also Known As: reference implementation; in-memory fake; executable spec; oracle model; test double; `httptest` server; steppable model.
Applicability / Recognition: Apply when — a subsystem has non-obvious dynamics (a reconcile loop, a state machine, a retry/backoff protocol); understanding requires mentally simulating execution; an external dependency (HTTP, a store, a queue) must be reasoned about offline. *Detector seed:* a subsystem whose behavior is documented in prose-describing-execution, with no runnable model or fake → recognition fires (execution-prediction is weak models' weakest point — DR2-019, CRUXEval ~63% even for strong models).

## Band 2 — Justification & Tradeoffs
Motivation: A weak agent must modify the reconciler's convergence loop (diff → actions → apply → re-diff). It cannot mentally simulate whether a partial apply converges or oscillates. A steppable in-memory world — `sim.World` with `Step()` returning the applied actions and the next state — replaces mental simulation with execution: feed a desired/actual pair, watch convergence, print the trace. The EsoLang library shipped exactly this idea (a local simulator) and it carried the weak-agent gain.
Structure & Participants: *Reference model* (runnable, small, steppable — `Step()`/`State()` inspection surface) · *In-memory fake* (a literal implementation of the seam's narrow interface — Go's native double) · *`httptest` server* (the stdlib simulator for HTTP boundaries) · *Conformance test* (model vs production agree on representative inputs).
Collaborations: Provides the comparator for Class D oracles (the model IS the expected-behavior source); backs Class C contracts; pairs with Class G (the model's usage is Example-demonstrated). Capability injection (§2) is what makes fakes drop-in — a cell taking `seams.Store` accepts the ten-line map-backed fake with no mocking framework.
Goals / Non-Goals: *Goals:* make non-obvious dynamics executable, not just described. *Non-Goals:* NOT a second production implementation (a reference model, kept simple); NOT for trivially-obvious subsystems; NOT reflection-based mock generation (gomock-class module-graph interception is the §7 posture — literal fakes are cheaper and honest).
Consequences: (+) the reader runs instead of simulates; (+) doubles as a Class D comparator and the test fixture; (+) zero third-party cost — interfaces + httptest are stdlib culture. (−) a model is code to keep in sync — conformance-test it against production; (−) over-modeling wastes effort — only non-obvious dynamics.
Alternatives: prose describing behavior (weak readers can't execute prose); reading the production code directly (the thing too complex to simulate). The model is the offload.
Risks & Assumptions: assumes the subsystem's behavior is modelable simply; a model that drifts from production misleads — conformance-test it. *Sunset:* if the production code becomes simple enough to read directly, the model retires.
Evidence & Transfer-strength: R2C-008 (simulator in the transformative library, benchmark), DR2-019 (execution-prediction weakness, benchmark). Class: benchmark. Tag: **[E-strong]**.

## Band 3 — Operation
```card-ops
trigger: WHEN a subsystem with non-obvious dynamics has no runnable reference model or fake THEN apply
mode: gate
routine:
  1. Identify the dynamics a reader must predict (states, transitions, convergence).
  2. Write a small steppable reference model (Step()/State() inspection surface).
  3. Provide literal in-memory fakes for the seam's capabilities (map-backed store, fixed clock); use httptest for HTTP boundaries.
  4. Add a conformance test: model vs production agree on representative inputs.
  5. Demonstrate the model's usage with an Example (Class G).
checker: conform `nonobvious-subsystem-has-model` + model-vs-production conformance test (go test)
raid_role: layer=cells; order=after:contracts; batch=cell
budget: active_rules=1; first_signal=conformance test (<60s)
```
