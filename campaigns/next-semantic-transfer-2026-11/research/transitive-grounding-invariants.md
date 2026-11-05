# Main resolver grounding invariants — bounded source check

**Decisive result:** the reviewed main resolver boundary has **no shared DAG invariant**, and `ResolvedGraph.dependencies` plus `is_root` is **not a complete record of why every selected package was selected**. A grounded-support rank witness therefore needs an additional proof/contract; it cannot cite roots-first output order or the existing acyclic property-test generator as an established package-graph invariant.

Read-only source findings. No Cargo, tests, probes, source/Git mutation, network or agents. `/tmp/nst-transitive-proof-report.md` was read as existing evidence: its reverse-support case passes selected-input equality but fails original-edge reachability. This audit does not rerun that result or reopen accepted F16.

## Cycles: specification versus actual code

- `PROP-017-resolvo-resolver.xml#SHARED-OUTPUT-BUILDER` (line136) promises roots-first order, exact-version pinning and obsolete dropping. `#DETERMINISM` (line330) says roots-first then group/name order. Neither is a topological order or DAG requirement. No package-cycle prohibition was found in that named specification.
- PROP-069's acyclic **guard** relationships concern local configuration/choice activation. Its positive feature-cycle behavior is another local algebra. Neither states that the selected physical package dependency graph must be acyclic.
- `crates/vibe-resolver/src/lib.rs:156-174` represents the output as a public vector of nodes and roots as the `is_root` filter. There is no graph-validating constructor.
- `build_resolved_graph`, `lib.rs:480-530`, removes obsolete selected entries, emits roots first and other nodes sorted, exact-pins references when their target is selected, and deduplicates dependency spellings. It does not perform cycle detection, topological sorting or root-reachability validation.
- `naive.rs:151-167` accepts an already-selected package if the incoming version constraint matches. `:240-257` enqueues dependencies and commits the chosen entry before processing the next queue item. Thus a compatible A→B→A queue terminates by revisiting chosen A rather than rejecting the cycle; both original direct dependency lists are retained. This is a concrete source counterexample to an invariant shared by solver cells, not an executed cycle test.
- Resolvo's adapter passes requirements into the solver (`resolvo_engine/provider.rs:277+`) and returns chosen packages through that same builder (`mod.rs:177-235`); it adds no acyclicity check. I did not inspect/probe cached resolvo internals and do not claim a newly measured cyclic resolvo outcome.

Focused-test limit: `tests/solver_properties.rs:6-9,36-43,201+` explicitly generates only acyclic worlds by allowing package i to depend on larger indexes. This is a **strategy restriction**, not a production invariant. The scoped search found a `features::cycles_terminate` test for feature implication, not a package-cycle acceptance/refusal test. Existing graph property/differential results cannot establish DAG behavior outside that generated domain.

## Which support survives the output projection?

| Cause of selected package | Solver admission | Output dependency edge/root support |
|---|---|---|
| User root | Main requirements plus `root_order` | `is_root=true`, preserved root order unless obsolete dropping removes the node. |
| Ordinary package requirement (including the current manifest's ordinary application-requirement projection) | Candidate dependency requirement | `manifest.resolution_requires()` is copied into `Chosen.direct_deps`; builder exposes/pins it. |
| Accepted `[recommends]` package | Extra hard requirement in a later greedy re-solve | **Not a root**, and no recommendation edge is added to the recommending node's `dependencies`. |
| `[[requires_any]]` selected alternative | Resolvo Union; naive separately queues an alternative | Selected provider edge is **not added** to the final `direct_deps` list unless also present as an ordinary requirement. |
| Required capability's chosen provider | Resolvo Union over catalogued matching provider versions; selected-set verification | Capability-provider relation is **not added** to `dependencies` or marked a user root unless independently selected as such. |

Exact paths:

- `resolvo_engine/mod.rs:92-110` accumulates accepted recommends in `extra`, after collecting them from a selected graph. `:125-155` supplies roots+extra to the hard solve, while only actual roots populate `root_order`. `:189,210,224` determines `is_root` from that root list and sets `direct_deps = manifest.resolution_requires()`.
- `tests/recommends.rs:135-146`, `recommend_is_pulled_in_when_it_fits`, has root A with **no ordinary dependencies**, recommends B, and asserts B exists with `is_root=false`. Together with the actual output builder, this is a source-backed witness that a legitimate selected non-root is not reachable by traversing the exposed ordinary dependency edges from A. It was not run here. “Weak” in this specification means package recommendation, not the common feature `Forward { strength: Weak }` contract.
- `collect_recommends`, `mod.rs:248-269`, reads the current selected manifests. Previously accepted extras stay in the `extra` vector across subsequent solves. No final recommending-owner provenance/revalidation is emitted. An extra is a solver-level accepted request; it is not automatically proof of a final authored parent edge.
- `provider.rs:293-320` encodes OR alternatives as Union requirements. `naive.rs:313-344` queues the chosen alternative separately; that helper does not append it to the owner's stored direct_deps. `resolvo_engine/tests.rs:248-268` tests selection of available alternative Y, but not a retained A→Y provenance edge.
- `provider.rs:323-354` encodes capability providers as a Union over exact provider versions. `capabilities.rs:43-84` discovers candidates by traversing ordinary/OR closure across **all available versions**, with a seen set; that is discovery reachability, not selected-candidate root support. `:90+` verifies that selected packages collectively provide required capabilities, not selected provider-edge provenance.
- `resolvo_engine/tests.rs:342-362`, `resolvo_satisfies_capability_via_required_package`, also has an ordinary A→B requirement. Its selected B assertion therefore does not test the missing capability-only output edge. A provider known from an unselected version/OR branch can be solver-supported through a capability without that capability relation appearing in the physical graph projection; this latter statement is source inference, not a new runtime counterexample.

Specification/implementation distinction: PROP-017’s capability encoding table (`ROW-CAPABILITY-RESOLVO-ENCODING`, line201) describes virtual capability NameIds. The current provider body instead builds Union requirements over real package/version candidates. Neither description establishes a retained physical provider edge; the current bodies above decide what the output actually contains.

There is also no universal “all exposed edges point at a selected node” proof in the builder: obsolete dropping removes nodes before pinning, and the pinning loop leaves a reference alone when its target is absent. `resolvo_engine/tests.rs:320-334` checks the obsolete package disappears, not that all incoming references or descendants are repaired. This is a limit of the output representation, not a request to change obsolete semantics in this task.

## The selected validation hook does not supply the missing invariant

`VersionEnumerator::validate_selected` (`lib.rs:235-240`) defaults to Ok. The F16 wrapper (`configuration.rs:135-167`) verifies selected candidate evaluation/record consistency and delegates to its underlying source. It does not certify a physical DAG or reconstruct recommendation/OR/capability support. This is consistent with F16's accepted bounded scope; no stronger transitive grounding claim is inferred from it.

## Consequence for the proof decision

1. A rank required to increase along **every** physical dependency edge would impose an additional restriction on compatible cycles; it cannot be justified as reusing an existing DAG guarantee.
2. Roots-first vector positions are deterministic presentation positions, not support ranks. A queue's finite visited set is termination evidence, not acyclicity evidence.
3. A witness grounded only in `ResolvedGraph.dependencies/is_root` would reject the existing legitimate recommends fixture and omit real OR/capability selection causes. It must state which support relation and actual roots it proves; discovery closure or reverse support requirements cannot silently serve as that authority.
4. The probe's failed original-edge reachability condition is therefore a meaningful **additional** invariant for its claimed effect/input-support universe, not a property already mechanically guaranteed by main's general graph builder. Establishing that invariant requires new bounded proof/evidence while respecting existing cycle and auxiliary-selection behavior; this report proposes no production encoding or solver replacement.

Strict distinction: these conclusions concern the inspected resolver/output boundary. Other domain owners may apply their own loading/execution constraints; no downstream universal graph guarantee was assumed or audited.
