# Rich consumer-choice input after the feature kernel

Read-only design input for root. Existing main code was read at `84e44442c8ba836ed9d87448965a5a792a756779`; prototype at `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. Current working `PROP-069-common-configuration.xml` was read with SHA-256 `fa85769ef6e08b2229f623d8e3e9e64f6b17000016fa9ddd519de58c6c757cb0`, plus the named 90-user standing rules and the campaign's `research/choice-semantics-comparison.md`. The feature-kernel worker's checkout was not read or modified. Its reported API shape below is provisional integration context, not independently accepted code. No builds, executions, network calls, DB operations or product edits.

The next reusable unit is **pure evaluation of consumer choice intent for one already identified actual configuration instance**, composed with the existing local feature-closure kernel. It needs definition validation, guard ordering, answer/default processing, cross-consumer merge and a resolved-result boundary. It does not need a socket host, registry access, a second package identity, a grant, an execution namespace, a scheduler or a new store.

## Proposed semantic types and API boundary

Names below are proposed; reuse the landed kernel's actual names when root accepts it. Do not create a second Cardinality or feature-ID universe.

| Type | Required data and purpose |
|---|---|
| `ChoiceGroup` | Stable local group ID; ordered member feature IDs; shared `Cardinality`; positive group guards; `defaults: Option<Set<FeatureId>>`. `None` and declared empty defaults differ. Presentation order must survive; membership comparisons use sets. |
| `ChoiceGuard` | `Group(GroupId)` or `Member(FeatureId)` over the validated choice graph. No expressions, host paths, environment reads, or package effect execution. Validate every reference and detect cycles, including inactive branches. |
| `ChoiceModel` / validated form | Choice groups plus member guards, joined to the existing validated feature model. Validation precomputes the guard DAG. Closure groups remain in `FeatureModel`; consumer-choice groups are explicit separate normalized meaning. Do not reinterpret a group from package kind. |
| `Selection` | `Omitted` or `Explicit(Set<FeatureId>)`, including the empty set. Missing map entry normalizes to omitted, never to explicit empty. |
| `SelectionIntent` | `Additive` or `Exact`, independent of `Selection`. This independence is necessary even when members are omitted; see the verified default example below. |
| `RequestedSelection` | Selection, merge intent and request origin. Origin is an existing consumer/declaration reference or diagnostic key, not authorization. |
| `ConsumerChoiceIntent` | Existing consumer/declaration key; per-consumer `RequireSelection` / `UseDeclaredDefaults`; group answer map; retained default receipt, when applicable. Domain owner already grouped the actual consumers. No domain/target/lock identity reconstruction belongs here. |
| `DefaultReceipt` | Consumer/declaration binding, identity of the semantic definition/defaults shown, and the consent scope/merge intent retained with the request. Use a canonical semantic snapshot or its reproducible fingerprint; do not treat it as a secret or capability. Keep the previous receipt when a default disappears so the disappearance is detectable. |
| `ChoiceEvaluation` | Original requests separately from active/dormant group answers, eligible control data, default usage, questions, conflicts and named stale/removed-answer diagnostics. Only a resolved branch contains a value usable for domain effect projection. |
| `ResolvedConfiguration` | Effective selected features and kernel implication reasons, with evaluated choice evidence. Created by successful shared evaluation only. Domain planners consume this result; unresolved diagnostics cannot masquerade as an effect expansion. |

A minimal callable boundary is conceptually:

```text
validate_choices(feature_model, choice_definition) -> ValidatedChoices | DefinitionErrors

evaluate_configuration(
    feature_model,
    validated_choices,
    ordinary_feature_input,
    consumers_of_this_instance,
    optional_expected_definition_for_split_interaction
) -> ChoiceEvaluation
```

`ordinary_feature_input` uses the kernel's base/additions/disabled representation, prepared by the existing app/package adapters. It is not another checkbox algorithm. Original complete domain requests remain with their owner; the pure evaluator need not copy unrelated package/version/operation fields into itself. A resolved outcome invokes the one closure kernel. A blocked outcome carries questions/conflicts/reconfiguration diagnostics and no `ResolvedConfiguration`.

Parameter evaluation can join this result through the existing typed parameter owner; do not manufacture feature IDs for string/integer/path values. This packet does not require parameter schema changes to implement choice merge. Profiles lower to ordinary feature input and/or explicit group inputs before evaluation, with profile origin retained by the consumer. They do not inject effects directly.

The current worker reports `FeatureModel` nodes plus `ClosureGroup { id, members, Cardinality, fallback }`, and `FeatureInput { base: Defaults|Selected, additions, disabled }`; positive implication closure precedes simultaneous fallback rounds. The rich layer should use that contract, not depend on its private graph or replicate its closure rules.

## Evaluation rules, in order

1. Validate the whole declaration and request against the current candidate definition. Unknown or removed explicit/dormant member/group produces a named invalid-answer/reconfiguration diagnostic. Do not silently filter it as legacy app update may do. Duplicate IDs are invalid; being a member of two *closure constraints* is not a duplicate feature declaration.
2. Check an optional split-interaction definition identity before using answers for a changed displayed definition. Separately check default receipts for consumers actually following defaults, including retained previously-defaulted groups that have disappeared. Unused stale metadata must not authorize current defaults. Missing/currently stale consent produces rediscovery, with no default-derived feature activation.
3. Walk the positive guard DAG. An active group contributes requirements; an inactive group is dormant and neither asks a mandatory question nor activates effects. Members become eligible from their positive guards. An answer does not activate its own guard.
4. For each consumer and active group, retain the original answer unchanged. Explicit members, including `[]`, never consult defaults. Omitted + require-selection supplies no members. Omitted + use-defaults supplies the reviewed defaults if available; the answer's merge strength remains independent. No declared defaults supplies no invented answer.
5. Merge the active eligible projections for each group: additive sets union; exact sets agree; every additive set must be a subset of any exact set. Name both contributing consumers/declarations on conflict. Evaluate cardinality on the effective selected members. Mandatory underflow asks for selection; overflow/conflicting exacts refuse. Empty exact for a zero-or-more group forbids another consumer's nonempty additive contribution; empty additive does not.
6. Feed successfully chosen members and the already normalized ordinary feature input into the shared closure kernel. Reject any ordinary implication/fallback that silently activates an unchosen consumer-choice member. Likewise refuse closure which makes an accepted explicit choice inconsistent with closure constraints. Closure groups may constrain effective features; choice groups authorize their selected members. Do not repair this mismatch by deleting the consumer answer or silently picking another group member.
7. Return original/dormant intent independently of effective features. Only the resolved result can reach the existing package effect or app step planner. Keep enough diagnostic choice data for a console/GUI to answer the next question even though there is no executable partial effect result.

**Guard-scope clarification needed in PROP-069:** the prototype applies exact/additive conflict checks to active eligible projections, while retaining full original dormant answers. Example: guarded member `x` is inactive; A requests exact `{x}`, B exact `{}`. Their current eligible projections are both empty; once the guard activates they conflict. This is a coherent meaning for dormant intent and is the recommended behavior. `CONSUMER-MERGE` currently says “exact selections must agree” without explicitly saying when dormant members participate. Root should state the chosen rule before freezing tests; rejecting all dormant disagreement immediately would differ from the prototype and make hidden branches constrain today's configuration.

## Verified omitted-plus-exact behavior: defaults are not always additive

Prototype `vibe-wire/src/behaviour/consumer_configuration.rs::m_05_e_4_cases::explicit_empty_and_omitted_answers_remain_distinct` constructs an Exact selection, changes `members` from `Some([])` to `None`, and asserts `admit_request` accepts it as Omitted. Admission preserves `intent` independently. In `vibe-resolver/src/features/configuration.rs::resolve_consumer_configuration`, omitted use-defaults materializes defaults and retains the explicit selection entry's intent; an entirely absent group entry defaults to Additive.

Meaningful example for a zero-or-more plugin group whose reviewed defaults are `{a}`:

- Consumer A says “include the reviewed defaults”, B adds `{b}`: additive default `{a}` + additive `{b}` resolves `{a,b}`.
- Consumer A says “use exactly the reviewed default set”, B adds `{b}`: exact default `{a}` conflicts with B.
- Consumer A explicitly says exact `[]`: no defaults apply, and B adding `{b}` conflicts.

These are different real requests. Preserve selection state and merge strength as independent fields. Do not require a placeholder omitted entry to acquire a hidden privilege: normalize the absent-entry shorthand explicitly to Omitted+Additive, and let a real Exact omitted entry express the stronger default-set requirement. `UseDeclaredDefaults` only chooses where omitted values come from; it does not determine merge strength. This conclusion is supported by admission/test source and evaluator source, not a newly run test.

## Receipt and candidate ownership

The pure core can compare a caller-supplied definition fingerprint or normalized semantic snapshot. The actual app/package owner supplies the definition bytes it is using and checks source/version/preconditions; a client cannot make a new physical configuration instance by sending a label. Consumer keys inside evaluation are for preserved attribution/conflict reporting only.

Two checks must remain separate:

- A split describe/evaluate/apply operation checks the particular shown definition it is applying. Changed semantics require rediscovery rather than applying an answer to an unseen definition.
- Solver candidate trials may apply stable explicit answers to another compatible candidate. They must not carry defaults authorized for candidate v2 into v1. Explicit answers do not intrinsically pin an obsolete version; default receipts do bind the shown semantics.

Prototype `DefaultBinding` stores the entire request plus candidate identity and full shown definition and requires all of them equal (`resolvo_engine/configuration.rs:149,511`). That is evidence for the stale-input invariant, not a mandate to copy all environment/source wrapper types into the common core. Smallest meaningful retained basis is the owning consumer's choice/default context plus the exact semantic definition/default basis; unrelated consumer ordering and display labels must not change its meaning. Root should define canonical encoding before generating a wire contract. No receipt should include a secret parameter value or a public reusable digest of one.

## Actual adapters and reusable effect owners

| Consumer / target | Existing functions and exact next connection |
|---|---|
| Common semantic module | Proposed `crates/vibe-core/src/configuration/choices.rs` and focused `choices/tests.rs`, exported from the configuration module. Reuse landed kernel identifiers/Cardinality/closure; add the validated guard/intent layer here. Names are suggestions, not existing files at the inspected commit. |
| Main app compatibility | `crates/vibe-app/src/steps/features.rs::resolve_features`, `steps/plan.rs::choose_features`, `operations/modify.rs::keep_recorded`. Keep command/profile/prompt/previous precedence here. Existing app exclusive-group lowering is a closure constraint with its valid default, not a RequireSelection choice group. Existing Installation records are effective-state compatibility input, never fabricated richer original intent. |
| App values/effects | `steps/params.rs::resolve_parameters`, `ParameterDecl::canonical`, `steps/plan.rs::holds`, existing steps runner. Keep path existence in host facts and step host conditions out of the guard DAG. Add rich original-intent retention through the app installation owner when that domain exposes rich choices, preserving old document schemas via adapters/versioning. |
| Existing package declaration | `crates/vibe-core/src/manifest/package/features.rs::FeaturesTable`. Preserve ordinary feature strings and at-most-one `[features.exclusive]` compatibility. Rich definitions need an explicit normalized lowering; no mutation of package definitions to store user answers. |
| Package feature algebra | `crates/vibe-resolver/src/features.rs::{FeatureValue::parse, expand_features, FeatureExpansion::merge}`. Lower local Feature→Feature edges into the common closure model, then project existing dependency/subskill values from the resolved selected set. Do not run a second independent closure afterward that activates unchosen rich members. Add checked typed config-effect projection to this owner rather than introducing a general effect runtime. |
| Actual package solve | `vibe-package-source/src/source.rs::{solve,solve_masked}`, `cells.rs`, `vibe-resolver/src/resolvo_engine/provider.rs::get_dependencies`. Choices that affect dependencies must be evaluated per candidate while obtaining its dependency set, before committing a graph. Both normal and visibility/masked retries must carry the same original consumer intent. Evaluating only after fetching the solved graph is too late. |
| Install planning/result | `vibe-install/src/plan.rs` currently calls `source.solve` before `plan/fetch.rs::fetch_node` expands features. Carry selected candidate evaluation into the prepared install and validate it before the existing apply transaction. `record.rs` and `apply/lock.rs` currently persist active feature sets; extend the real authored consumer/request and lock-result owners for rich intent, not a parallel global database. The existing `PlannedInstall` already holds the selected manifest/workspace for apply. |
| Console / external tools | Existing app `describe`, `plan`, callbacks and CLI prompt helpers are real working consumers. Normal package install must expose NeedsSelection from its actual source owner to those controls, then stage accepted authored answers through the existing operation. No manual `ConfigurationChoiceHost` bootstrap; missing authority cannot be replaced by trusting arbitrary raw discovery JSON. |
| Other kinds | Flow/Feat/Stack/Tool/Mcp/Lang can describe the same semantics through their real package/workspace consumer. App uses its installation home. Doc remains read-only/cache/reader scoped; Remap remains user-home/project remap policy scoped and explicit-consent controlled (`PackageKind`, `commands/cache/add.rs`, `commands/remap/packages.rs`). A resolved configuration cannot by itself authorize installing Doc or accepting a Remap. All nine kinds share definition/evaluation meaning, while supported effect operation remains domain-owned. |

**Actual main integration gap:** at the inspected commit, repository-wide readers of `active_deps`, `dep_features`, `weak_dep_features` and `active_subskills` occur only inside `vibe-resolver/src/features.rs` and its tests. `vibe-install::plan/fetch` computes FeatureExpansion after solving; `record.rs:327` records `active_features` and sets `subskills_active` empty. Thus reuse the existing parser, feature algebra, resolver, install and subskill owners, but do not claim forwarding/subskill activation is already wired end-to-end. The prototype is also incomplete here: its B1 pure forwarding works, while C2 explicitly refuses TargetFeature in configured solving. A common option system must connect the real consumers, not inherit either gap.

## Compatibility edges and concrete specification risks

1. **Three existing group meanings, not two accidental aliases.** Main package exclusive is at-most-one without fallback (`PROP-003#EXT-EXCLUSIVE`, FeaturesTable), legacy app active exclusive is exactly-one with its unique fallback, rich consumer choice has explicit policy and four cardinalities. All fit the normalized distinction in PROP-069. Mapping package groups to app exactly-one or making their direct flags replace defaults would contradict current package behavior. Package adapter must put the explicit/default union into the kernel's Selected base; reserve direct-addition replacement for the app's specified semantics.
2. **Overlapping closure constraints.** A feature `b` in `g1={a,b}` and `g2={b,c}` represents two at-most-one constraints. Selecting `b` satisfies both; selecting `a,c` also satisfies both. The main parser and normal evaluator allow this, and the current spec defines each list as an at-most-one set; no structural membership ban was found. Do not use one global `feature→group` slot or import prototype's “multiply owned member” prohibition into closure validation. This is not a claim that a complete authored check was run: `validate_features_table` probes all-features and can report an exclusive conflict even for ordinary default-valid groups. That existing probe does not establish a structural overlap ban.
3. **Rich member ownership needs an explicit decision if overlap is exposed there.** Prototype assumes a member belongs to one choice group and uses an unqualified Member guard. PROP-069 does not state this uniqueness rule. Do not infer it from the code. Closure overlap must work; for rich overlapping choice groups, either define a group-qualified guard/member interpretation with a real consumer example or state a deliberate restriction. Do not silently reject legacy closure overlap to simplify rich choices.
4. **Positive implication cycles differ from guard/parent cycles.** Main feature implication cycles terminate by closure (`features.rs::cycles_terminate`). PROP-069 only demands parent/guard-cycle rejection. Do not generalize that into rejection of established idempotent feature-requires cycles.
5. **Unknown or disabled branch answers stay diagnostic.** Rich removed dormant members require reconfiguration, unlike legacy app's filtering of old effective sets. Both behaviors are already distinguished by PROP-069's compatibility clause. No contradiction needs a new identity protocol.

## Discrimination tests for the next slice and its consumers

These are proposed checks, not execution results:

- All four cardinalities; active mandatory empty => question, inactive mandatory empty => dormant/no question; declared `None` default versus `Some([])` default.
- Omitted+RequireSelection, absent+UseDefaults(Additive), explicit Omitted+Exact+UseDefaults, explicit `[]` under both strengths. The three-consumer permutation order must not change merge result or reported origins.
- Exact `{a,b}` plus additive `{a}` resolves; exact `{a}` plus additive `{b}` conflicts; differing exact sets conflict; exact empty versus additive nonempty conflicts.
- Guarded answer persists through switch-away/switch-back; inactive exact disagreements become conflicts only when eligibility activates if root adopts the rule above. Unknown dormant answers and cyclic/unknown guards refuse before effects.
- Ordinary implication attempts to activate an unanswered choice member: refuse; app closure-group implication/default behavior continues unchanged.
- Reviewed default changes/removes a member or disappears entirely: rediscovery, including when today no current default is present. Explicit compatible choice survives a changed default. An unrelated UI label/order change must not masquerade as a changed semantic default.
- v2 effects rejected by constraints do not leak into v1; v2 default receipt cannot silently authorize v1; original consumers survive candidate retry and visibility retry.
- Legacy package default `a` plus explicit `b` in exclusive `{a,b}` conflicts; legacy app explicit `b` replaces default `a`. Overlapping closure groups `g1={a,b}`, `g2={b,c}` evaluate per constraint.
- Pure feature effect tests already present remain: strong forwarding activates dependency; weak forwarding alone does not; activating the dependency later enables its weak feature; merge order independent; typed config conflict returns no partial expansion. Then a real package solve/install test must prove each advertised effect changes the proper downstream result.
- Real app describe/plan/install/update/modify behavior remains compatible; ordinary console and serialized package answers persist intent, and a second fresh-process install/reconfigure reproduces it. Doc/Remap tests prove no effect authority is obtained solely from a resolved answer.

Prototype discriminators to translate semantically: `features/configuration/m_05_b_2_cases.rs::{explicit_defaults_and_empty_policy, shared_additive_union_and_actual_evaluator, exact_conflicts_name_both_edges_and_additive_must_fit, dormant_intent_survives_until_positive_guard_is_active, dormant_member_exact_intent_and_group_guards}`; `resolvo_engine/m_05_c_2_cases.rs::{compatible_older_candidate_keeps_only_selected_effects, changed_semantic_default_requires_rediscovery, removed_optional_default_requires_rediscovery}`; `vibe-orchestrator/src/install/m_05_e_2_cases.rs::{update_keeps_explicit_choice_when_default_changes, removed_option_and_new_required_group_return_all_needs_without_writes, update_retains_explicit_empty_and_dormant_intent}`. Reuse their counterexamples, not their unnecessary host/environment wrapper surface.
