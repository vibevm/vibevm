# Candidate-owned forwarding — pre-implementation ordering seam

Baseline qualification `f9ac3055b`; bounded implementation contract `NST-PACKAGE-FORWARD-FIRST-01.md` integrated as `906178aa6`. This is a source inspection report, not runtime implementation or permission to replace the solver.

## Existing operation order

1. `vibe-install/src/plan/roots.rs` collects actual authored workspace requires occurrences into fixed PackageConfigurationInputs. A forwarded package-owned edge is not another workspace occurrence.
2. `vibe-resolver/src/source_solve/capture.rs` freezes each permitted source domain. For every discovered candidate it calls metadata::prepare before constructing the resolver model. CandidateKey identifies coordinate/version/source binding; duplicate metadata changes refuse.
3. `source_solve/metadata.rs` calls ConfigurationSnapshot::retained_source. That snapshot evaluates the common definition against fixed workspace inputs and retains one projected dependency view per candidate identity and metadata digest. The target's selected feature may change its outgoing dependencies or data.
4. `source_solve/edges.rs` constructs original candidate-owned dependency witnesses from those already projected views; `model.rs` builds the finite resolvo model from them.
5. Only `source_solve/output.rs` decodes selected ActiveSourceEdges and their exact EdgeOwner::Candidate provenance. The selected source candidate and actual dependency edge are known here, after target views have already constrained viability.
6. The legacy fast path has the same decisive boundary: it calls metadata::prepare during fetch_manifest and returns unsupported configuration errors before selected-graph validation. Silently falling back to that path does not resolve the ordering problem.

## Missing decision

A target's effective incoming forwarded request depends on selection of a particular parent candidate/edge, while that candidate/edge selection can depend on the target's effective dependency view. The current cache identity does not include a candidate-conditioned incoming consumer set. Applying every discovered parent's answer would leak from unselected candidates; applying an answer after solve would invalidate the solved dependency view. A second solve with a fabricated workspace path would lose provenance and is expressly outside the contract.

The first strong-only, acyclic, unique local candidate slice may be made decidable by an architect-defined precondition that establishes exact mandatory parent/target authority before target evaluation, or by a finite input-variant relation inside the existing solver owner. Neither algorithm is already provided by the current fixed-input snapshot. Before writer activation, the architect must prescribe the exact eligibility proof, target-input evaluation key and independently validated durable source/target/edge provenance. Weak, recursive, ambiguous and unsupported routes must remain explicit refusals.

The builder reported this exact seam before changing product code or adding a solver pass. The existing accepted forwarding qualification still demonstrates selected refusal and dormant/direct-target positives; no new runtime test was needed for this read-only ordering report. No agent, provider, network, new inference, lock format or solver implementation occurred. UP adoption remains UNRUN; REMAINDER/AR/IDE unchanged.

## Architect's bounded algorithm decision

The architect subsequently supplied a first-slice rule: a compulsory directly required workspace root P has an authored exact version and uniquely admitted local source/content identity; its fixed original input alone determines the selected strong dependency edge. T must also have an exact declared version and unique admitted local source. No alternative candidates, optional root eligibility, weak/recursive forwarding, cycles or unaccounted incoming edges are admitted. A bounded owner may pre-seed that candidate-owned target request, use one existing solver run, key evaluation by candidate/metadata/incoming-set identity, and require final selected original edges to match the preseed before effects. Ranges are refused, never silently narrowed.

The builder proposed a separately typed forwarded_inputs record under conditional minimum epoch14, leaving authored consumers unchanged, and requested the exact durable field/identity decision before writer changes. Cold replay must validate the source/target/edge pairing; the existing workspace freshness comparison also needs to distinguish genuine authored consumers from separately validated package-owned requests. No implementation has started pending that format decision.
