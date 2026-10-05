# A0.1 research review: dual-language proof and task-solving model

Reviewed on 2026-10-05 against the owner priority to model task solving
independently of existing Zap behavior. The bounded research artifact is
`/fast/git/v/research/2026-10-05-zap-lean-study-receipt.json` (SHA-256
`9bfcbc2703fb74e61a8fc0056484a299b547dbe5a6879f6f910f8c328b2e1495`).
Its three companion documents cover the source-mapped Zap decision surface,
the proposed dynamic software-engineering world and Lean theorem programme,
and the exact VibeVM same-method Rust + Lean capability audit. All 253 local
document links resolved in the recorded review.

**Accepted research decision, not accepted implementation:** Zap's current
code and behavior are diagnostic evidence and migration input, not the
correctness oracle. Model a partially observed, changing engineering world:
authorized intent, actual versus known facts, discovered obligations and
unknown relevant closure, code/dependency versions, tools/agents/resources,
evidence, external effects and authority. The task graph is a derived
frontier. Prove safety and honest completion for all modeled traces; prove
progress/solution only under explicit discoverability, viable strategy,
resources, observation, fairness and stability assumptions. A hidden
required dependency makes an empty visible frontier insufficient to claim
Solved. An unsupported control cannot wait forever for an event its adapter
cannot emit. Formal properties must be checked against source and live event
traces; a long inactive interval alone does not prove an algorithmic stall.

VibeVM's neutral graph already permits Rust and Lean relations to one spec
method. The missing product capability is composed checker invocation and
one authoritative, versioned proof-result join: current per-language maps
can overwrite each other, host projection loses pin freshness, and an edge
is not a theorem result or proof of the Rust implementation. A0.2 must ship
the minimal AI-Native Lean family; A0.3 must exercise both checkers and a
fresh exact-subject witness through the same consumer. Keep the proof grade
explicit: a checked Lean model is not a refinement theorem for Rust.

The research-only Lean 4.34.1 prototypes compiled 22 named theorems and 12
examples across two small models, with kernel replay exit 0. Eight bounded
source-slice check groups passed. These results do **not** verify current
Zap, prove arbitrary task completion, exercise installed mixed-language
checkers, or explain the 34h32m Lens event gap. The latter remains an
unexplained historical data gap; a separately correlated unsupported-pause
state mismatch is established and tracked in Zap/PodBay backlogs.

The implementation order is A0.2 Lean family, A0.3 same-method composition,
then the exact A0 gate. The later Zap programme begins with an independent
world/action/knowledge contract and non-vacuity/progress proofs, followed by
refinement, controlled migration and a continuous improvement loop. Running
Pods retain their original effect identities during compatible model-version
rollout. No other pending NEXT atom resumes before A0-GATE acceptance.
