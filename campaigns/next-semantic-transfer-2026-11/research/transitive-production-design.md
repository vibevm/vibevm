# Transitive configuration: production boundary, scope correction and alternatives

Read-only design review. No production or probe edits, Cargo/tests, dependency changes, network, Git mutations or new agents. v1/v2/v3 remain frozen research evidence; no result was rerun here. Prototype references below are immutable `7fe6376ccef7f321302a8f18610fcb0c9cc2686f` blobs read with `git show`, never dirty files. Current source references are main at the reviewed F16 boundary.

## Decision recommended now

**Do not start a general transitive-configuration solver foundation as the next transfer atom.** The pinned configured prototype explicitly refuses cross-package forwarding; the implemented local feature closure/expansion must not be relabeled as a working recursive consumer pipeline. The v1–v3 experiments exposed useful future design obligations, but research cannot manufacture campaign scope.

Preserve the following alternatives as future architecture debt. The next concrete useful vertical should be **package defaults/profile semantic basis in the existing fixed-consumer route**, grounded in actual prototype shown-default behavior, the already accepted fresh-default policy and main's working evaluator/state owners. Root has requested that as the next focused design task; this memo is not authorization to edit it. A narrow transitive specialization is not recommended merely to accumulate another supported exception.

## Actual prototype scope, not completion labels

| Pinned source/test | What it proves by source, and what it does not |
| --- | --- |
| `crates/vibe-resolver/src/features.rs:267–280,578–593` | A local feature queue recursively follows `FeatureEffect::Local`; the `cycles_terminate` test asserts local a↔b closure. This is recursion within ONE definition, not parent-selected child configuration. |
| Same file281–311,561–575 | Strong/weak effects populate `active_deps`, `dep_features` and `weak_dep_features`. The weak test checks expansion fields and lack of activation. It does not install/configure the foreign package. |
| `crates/vibe-resolver/src/features/configuration.rs:250–295` | Complete local consumer choices feed the effect expansion, preserving the choice-member authorization boundary. It remains a local algebra. |
| `crates/vibe-resolver/src/resolvo_engine/configuration.rs:396–403` | The actual configured solver rejects ANY TargetFeature in admitted definitions with an explicit candidate-conditioned-consumer limitation. This is a body-level refusal, not a missing test inference. |
| `resolvo_engine/m_05_c_2_cases.rs:237–267` | `compatible_older_candidate_keeps_only_selected_effects` exercises fixed-input candidate selection; its Strong TargetFeature control explicitly expects Unsupported. Task-linked test evidence contradicts universal forwarding completion. |
| `crates/vibe-install/src/plan/fetch.rs:33–44`; `plan.rs:550–579` | Ordinary planning fetches nodes independently. Transitively fetched nodes get FeatureRequest::default unless an already evaluated fixed route effect exists. These bodies do not propagate parent dep_features into a new target consumer input. |
| `crates/vibe-install/src/authored_configuration.rs:32–46,71–75`; `plan.rs:360–375` | Real workspace-authored inputs are captured; policy is hardcoded require_selection and default bindings start empty. This is useful ordinary input integration, not dynamic incoming-edge discovery. |
| `resolvo_engine/configuration.rs:149–178,511–547`; `machine_operation/configuration_choice.rs:331–373`; install `record.rs:351–374` | Actual shown consent is bound to the supplied candidate/definition/request, checked before using defaults, and represented in the existing request record. This is a demonstrated semantic feature whose equivalent lifecycle is worth finishing in main. It is not automatic highest-version consent. |

No reviewed implemented prototype route requires the GENERAL dynamic transitive/forwarding foundation discussed below. The reviewed route deliberately excludes it. This is a bounded call-path conclusion, not an assertion that every unrelated repository file was audited. Defaults, parameters and host/external surfaces must each be credited only to their actual source consumers; this memo does not assume package-parameter or host completion from DTOs or labels.

## What the finite probes established

v1 showed candidate+input-state solvables can share one real package NameId and use actual core merge, but unowned/reverse support can invent inputs/parents. v2 added one decreasing physical witness per nonroot; rooted hard cycles remain legal, rootless support cycles fail. It still accepted rooted C.on→A→Q.blue→C.on without a seed. v3 added separate causal witnesses: unseeded on is impossible while off remains feasible; external/required seeds permit the same cycle with all redundant original input preserved. Choice members still require actual choice authorization.

These are finite expressiveness results, not discovery/completeness/scalability results. States, alternative causes and rank bounds were hand-grounded. None establishes a package DAG or complete support provenance in main's exposed graph. The grounding-invariants memo documents omitted recommends/OR/capability reasons.

## Required end-state invariants if general forwarding is prioritized later

1. One actual version/configuration per real package instance; original consumers are exact actual workspace or selected parent-candidate/declaration identities. No authored label creates another installation.
2. Actual inputs equal fixed inputs PLUS exactly contributions of selected parents' active owner declarations. Inclusion alone is insufficient; extra fictitious consumers are forbidden.
3. Original sources stay immutable. Evaluated state depends on candidate, full input/forward context, retained-basis context and route constraints. Distinct contexts cannot share a mutable cached SolvableId meaning.
4. Physical selection requires a real ingress cause; dependency cycles remain legal. Conditional activation separately requires grounded causal justification. Root package presence is not a seed for every feature. Alternative genuine causes are OR alternatives: the model may choose an external/required cause while retaining a redundant cyclic contribution.
5. Weak forwarding requires THAT owner's active declaration. Strong forwarding activates that declaration. Ambient target installation is not owner activation.
6. Full common-core validation remains authoritative. Malformed definitions are errors; incompatible complete inputs can reject an assignment; questions/rediscovery are unresolved no-effects outcomes, not permission to choose another answer. Fresh automatic receipts bind only final selected candidates; retained/shown meaning cannot silently renew.
7. Selected graph, provenance, effects and durable record come from the same solving state. No later unguarded re-fetch, effect recomputation or trimming of unsupported nodes certifies success.

## Why a positive activation fixed point is insufficient

Current `choices/merge.rs:13–77` checks all answered exact projections for agreement and bounds additions; `choices/evaluate.rs:110–164` checks active groups/cardinality only after merging; `:301–335` derives positive guard activity from requested/default seeds. More participating consumers can change Resolved→Conflict (red/blue exact disagreement or upper-cardinality overflow), NeedsSelection→Resolved (a REAL additional answer), or enable a dormant group/answer that creates a new question/conflict. Closure fallbacks also cannot be treated as unconditional permanent source answers.

A support fact derived from a partial consumer set is therefore not automatically valid under the final complete set. Do not publish a provisional feature/default, retain its cause through added consumers and call the resulting cycle grounded. The eventual encoding must model complete participation/eligibility and exact/cardinality constraints, while using the current core as the semantic authority. Ordinary feature reachability cannot bypass choice-member authorization (`evaluate.rs:167–210`).

## Exact installed API obstruction

Main `configuration.rs:34–38,169–250` memoizes one evaluation per coordinate with fixed inputs. `resolvo_engine/provider.rs:239–244,258–288,369–373` interns a semver row and returns immutable dependencies. Installed resolvo0.11:

- `DependencyProvider::get_candidates(NameId)` and `get_dependencies(SolvableId)` receive no selected-assignment/retraction context (`src/lib.rs:127–150`).
- `solver/cache.rs:79–155` caches the complete candidate list per name; `:336–365` caches dependencies per solvable. Later appending an incoming context/variant is not a supported invalidation operation.
- Public `Problem` exposes requirements, version-set constraints and soft requirements (`solver/mod.rs:54–144`); public Solver::solve returns a selected set or conflict (`:475+`). There is no public incremental theory-conflict/clause hook or sound candidate-list growth epoch here.
- `Condition` exposes positive Requirement/And/Or, not a direct absence operation. This alone is NOT a general expressiveness impossibility: finite total true/false helper domains plus Union requirements can represent negative literals. The unsolved issue is obtaining a complete stable domain/constraint set lazily and grounding those helpers truthfully.

A VersionEnumerator wrapper that mutates inputs cannot fix this. `validate_selected` returning an error also cannot teach this solve to choose A1 instead of A2; it is a final admission guard, not a semantic conflict-learning callback.

## Compare the real alternatives

| Alternative | Concrete ownership and payoff | Cost / missing proof |
| --- | --- | --- |
| **Symbolic immutable variables in SAME resolvo** | Core owns ONE authoritative validated relation/lowering interface for participation, local feature/choice/guard/default meaning; resolver maps finite per-declaration/choice/feature variables and clauses to immutable solvables/version sets/unions. Actual package version stays one physical identity. Physical and causal justifications remain separate relations. | Current core exposes evaluation and structured results, not an exact symbolic relation. Reimplementing its merge logic ad hoc in resolver would create a second authority. A core-owned relation needs differential checks against the existing evaluator, including dormant/exact/omitted/questions. Variables avoid eager input-subset enumeration, but do NOT alone close unknown incoming-edge universes, alternative witness causes or candidate-list stability. Reverse support clauses must be complete. |
| **Source-bounded state enumeration** | Existing evaluator can evaluate complete explicitly bounded input contexts; immutable state rows fit today's resolver API. Source observations are sealed before exposing complete candidate/state lists. v1–v3 provide small witnesses. | In general contexts can grow combinatorially. Reading all potential incoming candidates can defeat lazy acquisition and dormant-source boundaries. Canonicalizing equal EFFECTS cannot erase different original/dormant/default provenance. A bound must be explicit and exhaustive; truncation is unsupported, never success. No production guarantee follows from the tiny probe. Not recommended as the default general algorithm. |
| **Actual resolvo API extension** | Keep core evaluating complete contexts and SAME CDCL solver, with an explicit assignment/theory-refinement protocol or a sound candidate-domain growth contract. Resolver owns semantic reasons; solver owns retraction/conflict learning. | This is a real solver change, not an available callback. Required contracts include a stable domain/epoch, correct cancellation of stale cached exhaustive clauses, guarded conflicts including participation/absence assumptions, and provenance of newly discovered candidate relations. Appending variants while keeping learned clauses based on old exhaustive lists can be unsound. A result-validation/re-solve loop is not a substitute. Requires separate API judgment and executable proof; no fork/dependency change proposed now. |

For a fully known finite universe, the symbolic route is the most plausible way to avoid an eager powerset while retaining general exact semantics. That is a research preference, not a proof of a production-lazy solution. No option currently proves full lazy completeness under the installed interface. A narrow specialization could expose some transitive paths, but would not resolve these obligations and is not the recommended roadmap merely because it is easier to implement.

### Stability/completeness conditions that an implementation must state

Before a NameId's candidate list is cached, that list must be complete for its declared domain. Before a SolvableId's dependencies are cached, its source/input-context meaning must be frozen. New metadata observations may add new independently named relations only if no prior closed-domain/support constraint assumed their absence. Otherwise a real solver-domain revision is required. One operation snapshot retains successes AND failures across held-pin/recommend/mask trials; discarded assignments never become retained consumer input. Inactive optional foreign payloads cannot be fetched merely to seal a global powerset. A design that cannot satisfy these simultaneously must name its missing API/discovery assumption, not call a mutable map “lazy.”

## Provenance is necessary, but not the solver solution

Current ordinary requirements come from `provider.rs:278+`; OR at293+, capability Union at325+, and selected output `mod.rs:178–234` loses the exact disjunct/provider reason when it uses resolution_requires. Recommends at `mod.rs:92–110` become accepted extra requests; they are nonroot and may outlive the originally recommending candidate. Capture the actual selected declaration/provider witness from the SAME solve if those routes enter the rich support universe. Do not infer it later merely from target presence, inventory or first matching provider.

A typed record would distinguish actual root request, ordinary authored edge, OR alternative, capability provider and accepted-recommend request. The recommendation case must retain its actual algorithmic admission provenance instead of inventing a still-selected parent. That record unlocks truthful physical support and durable replay for those existing routes; it does not establish causal grounding of conditional features, complete incoming configuration, or solve nonmonotone merge. Do not build a universal provenance/store layer now solely because the future proof mentions one. Extend the existing selected result/lock owner when a real consumer of those records is chosen.

## Concrete next vertical: actual package default decision lifecycle

The meaningful next atom uses the current fixed-input solver rather than new dynamic consumer variants. Target the actual shown-default / retained-meaning feature established by the pinned callers and the accepted MAIN PACKAGE-DEFAULT-BASIS extension:

- Core `manifest/configuration/{intent,semantic,package}.rs` and `configuration/choices`: derive provisional automatic reliance using existing normalization/evaluation, preserving omitted/empty/exact/dormant/profile distinctions. No global highest-version basis and no app dependency. Default observation is not yet a receipt.
- Resolver `configuration.rs`: keep one immutable fixed-consumer operation context; evaluate each actual candidate using fresh permission or retained/shown basis, and retain only the final selected candidate's result. This fits the current stable coordinate cache because consumers are fixed for this route.
- Existing install/lock/maintenance owners: record original input, typed reviewed meaning and automatic reasons; validate cold replay and update/rediscovery before effects. Review format evolution through the current lock owner; do not invent a second receipt store or casually change a released shape.
- Public vertical: first authored defaults request→newest-feasible actual candidate→durable meaning→fresh-process cold reuse→changed default/profile meaning produces rediscovery, while an explicit fresh replacement can approve it. Fully explicit/no-auto input and presentation-only changes remain their distinct controls. Parameter-bearing profiles remain a named unsupported boundary until their ACTUAL parameter consumer is integrated; do not claim general profile/parameter completion from feature-only tests.

This is proposed because it completes a real semantic feature present in the prototype/accepted contracts, not because it avoids difficult work. The next focused task must determine its exact supported scope/record compatibility before coding. It does not claim to close full common configuration. General transitive forwarding remains visible design debt for a deliberate later priority decision.

## Challenge cases retained for that future judgment

- A2/red versus A1/blue plus fixed Q/blue; final input set changes with parent version, with no stale cached Q meaning.
- Two real exact consumers conflict; additive overflows cardinality; a newly eligible dormant exact answer conflicts; ordinary feature seed cannot authorize a choice member.
- Omitted consumer is legitimately satisfied by another REAL consumer; missing mandatory input is not treated as SAT rejection to avoid questions.
- Weak owner edge inactive while another root supplies target; strong/mandatory/another genuine owner-local cause activates it.
- Multiple genuine causal alternatives, including required/external seed plus redundant cycle; rooted hard dependency cycle allowed, rootless physical cycle and unseeded conditional cycle forbidden.
- A candidate or provider discovered late would add an incoming consumer after Q's candidate list froze; completeness must be demonstrated, not assumed.
- OR/capability-only ingress and accepted recommend survive truthful provenance without fabricating a user root or final parent.

No implementation is started by this memo. Private proof remains frozen; root retains scope/architecture decisions and archival cleanup.

Coordination at handback: root closed the finite proof as research, current implementation is G1, and history_release_policy owns the independent defaults/profile next-design. The defaults paragraph above is a priority/ownership recommendation only; I am not starting or duplicating that design.
