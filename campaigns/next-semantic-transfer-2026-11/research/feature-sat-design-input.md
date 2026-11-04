# Candidate-conditioned feature forwarding with existing resolvo 0.11.0

Read-only design input, 2026-11-04. Primary library source inspected at `/home/olegchir/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/resolvo-0.11.0/`. VibeVM's existing resolver adapter was read in main; main advanced from `c20b6bc8945ad4ff971db4ba8a2e9b1aa8645424` to `18dfad0921dc4e39a6ca22f8ded173227cebbf9a` during the root's work, but a Git comparison confirms no changes between those commits in the resolver wrapper paths inspected here. No source/state/Git edits, builds, new agents or test execution. The preceding integration report is unchanged.

## Recommendation

Use resolvo's existing conditional requirements inside the current provider. Reify independently requested package features as internal **feature names with versioned candidates**, then bind every selected feature candidate to the exact ordinary package candidate of the same version. Strong forwarding adds a requirement on the target feature; weak forwarding adds that requirement conditional on the chosen target's presence or on a chosen declaration-edge support condition, according to the meaning root selects.

This is an extension of the existing SAT adapter, not a second solver, a persistent catalog service, or a power set of feature combinations. Conditions alone suffice when all feature activation is already fixed for each package candidate. Full cross-package forwarding needs a solver-visible representation of feature activation because `Condition` can observe version-set requirements, not arbitrary external FeatureExpansion booleans.

## Observed API and semantics

Primary library files:

- `src/conditional_requirement.rs`: `ConditionalRequirement { condition: Option<ConditionId>, requirement: Requirement }`. `Condition` is `Requirement(VersionSetId)` or `Binary(LogicalOperator::{And,Or}, ConditionId, ConditionId)`. There is no public Not variant and no Condition::Union variant.
- `src/requirement.rs`: `Requirement::{Single(VersionSetId), Union(VersionSetUnionId)}`; `Requirement::with_condition(ConditionId) -> ConditionalRequirement`. A union is alternatives across version sets, often different package names. Unconditional conversions exist from both VersionSetId and VersionSetUnionId.
- `src/lib.rs`: `KnownDependencies { requirements: Vec<ConditionalRequirement>, constrains: Vec<VersionSetId> }`. Constrains restricts a package **if it is selected** and does not install it. It therefore cannot by itself activate a feature.
- `src/utils/pool.rs`: `Pool<VS,N>` accepts any name type satisfying Eq+Hash (`PackageName` has a blanket impl). `intern_package_name`, `intern_solvable`, `intern_version_set`, `intern_version_set_union`, `intern_condition`, and `resolve_condition` already exist. Conditions are interned/deduplicated; old interned data is immutable.
- `Interner::resolve_condition(ConditionId) -> Condition` supplies the expression tree. VersionSetId names one package/name, so observing a disjunction of differently named conditions needs an Or tree.

`src/solver/conditions.rs` gives the logical ownership explicitly:

```text
owner A_v requires target T if C
    means:  ¬A_v ∨ ¬C ∨ any_matching(T)
```

Thus a row returned by `get_dependencies(A_v)` is already gated by **that exact owner candidate**. If its feature effect is unconditional for that candidate, no additional `Condition::Requirement(A_v)` is necessary. If the effect depends on a feature selected independently, put it on that feature candidate or use an additional feature condition.

The current library lazily defers a conditional requirement until matching condition candidates are positively decided. It may enumerate candidates for the condition package even when the gated target is never loaded. A false weak gate therefore implies “does not select target,” **not** “performs no metadata lookup of the condition package.” Source failures still need their existing truthful treatment.

Conditions compile to DNF internally (`convert_conditions_to_dnf`). A large alternating And/Or expression can expand combinatorially. Feature forwarding usually needs a single presence condition or a short support conjunction; do not turn the authored positive guard DAG into an unnecessarily distributed giant expression.

Dependencies and constraints can be prefetched for rejected candidates (`HintDependenciesAvailable::All`); their clauses remain owner-conditioned. Provider callbacks must be deterministic over captured candidate data, not a mutable map of currently selected features. CDCL backtracking changes assignments, not callback contents or side-effect state.

## Smallest internal representation

Use a typed internal name, conceptually:

```text
Name::Package(group, name)
Name::Feature(group, name, local_feature_id)
```

The exact Rust type/name is for implementation review. Avoid concatenating synthetic names into parseable `group/name` strings: it risks collisions, accidental registry lookup and accidental public package output.

For every ordinary package version `B_v` that declares public/internal feature `g`, expose one candidate `F(B,g)_v` under `Name::Feature(B,g)`. Its candidate record can remain `semver::Version`; this reuses main's `SemverVersionSet`. Features absent from a version have no compatible feature candidate, or a named excluded candidate if needed for diagnostics. Feature candidates are generated from the same captured manifest as their owning package and never fetched as packages.

Every feature candidate hard-requires its matching ordinary package candidate:

```text
F(B,g)_v -> P(B) ∈ {v}
```

Use `SemverVersionSet::Explicit(singleton(v))` for exact internal candidate binding. A constructed semver `=v` requirement ignores build metadata; it is not the same exact-record identity. The solver's one-version-per-name rule then prevents selecting F(B,g)_1 and F(B,h)_2 together: both would require incompatible ordinary B candidates. Multiple different features at **one** B version coexist because they have different internal names.

No synthetic ordinary package is needed for “B is present”: `Condition::Requirement(P(B), Any)` already observes actual selected package presence. Do not encode presence by first requiring a dummy B marker that itself requires B; that would make weak forwarding strong.

Two consumers requesting the same F(B,g) share one feature candidate, while their original requests/provenance remain separate in the planner's input. This representation uses roughly one node per declared feature per candidate version, not one candidate per feature subset. Defaults/reviewed choices must be fixed by the common configuration contract; SAT cannot invent an answer to activate a feature atom.

## Exact lowering patterns

Notation: P(B,R) is the VersionSetId for ordinary package B satisfying R; F(B,g,R) is the VersionSetId for feature g candidates whose owner versions satisfy R. “Owner” below is the active forwarding feature candidate F(A,f)_a, or P(A)_a if common evaluation has already established that this effect is always active for that candidate.

### Strong forwarding through declaration d: A/f enables B/g

For declaration d targeting B with version range R:

```text
owner requires F(B,g,R)
F(B,g)_v requires P(B,{v})
```

The second requirement induces ordinary B presence. An additional owner→P(B,R) requirement is logically redundant but can be retained if the graph/provenance owner needs the explicit declared dependency edge; do not let either representation drop its declaration identity.

Internal local feature implication is the same form with an exact owner version:

```text
F(A,f)_a requires F(A,h,{a})
```

That keeps implication/effect definitions candidate-local. The lowering must correspond to the common validated feature/choice model, not replace it with an independently authored second meaning.

### Weak forwarding, global-presence interpretation

Conceptual actual API call shape:

```rust
let present = pool.intern_condition(Condition::Requirement(b_presence_set));
let row = Requirement::Single(b_feature_set).with_condition(present);
KnownDependencies { requirements: vec![row], constrains: vec![] }
```

The row lives in the owner's dependencies. `b_presence_set = P(B, Any)` checks selected B without selecting it. `b_feature_set = F(B,g, Any)` plus F→exact B forces g onto the **same** selected B version. No imperative “if graph.contains(B)” callback is needed, and no expansion is unioned into a global side table.

If the declared range is a hard condition even for a weak reference, it must appear either in the feature target set or in an explicit guarded ordinary B requirement. That choice has different user semantics from testing the range in the condition; see the decision table below.

### Weak forwarding, declaration-edge interpretation

Global package presence does not prove that this parent's declared dependency edge is active. If weak forwarding means “when declaration d is active,” gate on **support for d**, not merely P(B,Any).

For an unconditional required edge on A_a, d is active whenever A_a is selected. For a feature-activated optional edge, a condition can be the Or of its known activating feature candidates, tied to A_a. This avoids a separate marker where support is directly derivable.

If declaration activation involves an actual selected disjunction/provider edge that cannot be identified from package presence, an internal `Name::Edge(owner_candidate, declaration_id)` marker may be needed. Only the real activation/selection path requires that marker; the weak feature rule merely observes it. Its requirements bind the actual B version/source route. A weak request must never create its own edge marker. Do not create edge tokens universally when a static feature support condition suffices.

## Tiny graph oracles for implementation

These are logical expected outcomes for the proposed encoding; they were **not executed** in this task. Internal F names below must be excluded from the public installed package graph.

1. **Strong introduces dependency.** Root requires P(A,{1}) and F(A,send,{1}); send1 requires F(B,log,^1); B1 declares log; log1 requires P(B,{1}) and P(X,{1}). Expected SAT: real `{A1,B1,X1}`, features `{A/send1,B/log1}`. No B root was necessary.
2. **Rejected parent has no surviving effect.** A2/send requires F(B,log,^2), but no B2 exists; A1/send has no forwarding. Root requires P(A,Any), F(A,send,Any). Expected SAT selecting A1/send1, with no B/log or X introduced by rejected A2. If A2 is hard-pinned, expected UNSAT. This directly tests owner gating and output-effect retraction.
3. **Shared target cannot mix versions.** A→F(B,left,Any), C→F(B,right,Any); B1 declares only left, B2 only right. Roots A,C. Expected UNSAT: left1/right2 would require P(B,{1}) and P(B,{2}). Add B3 declaring both: expected SAT with only B3 and both feature atoms at3. No features-subset catalog is needed.
4. **Weak does not create target.** Root A/weak1; weak1 requires F(B,log,Any) if P(B,Any); B1 and log1 exist. Expected SAT with only real A. B must remain unselected, even though condition metadata may be enumerated.
5. **Weak fires when target is independently selected.** Add root P(B,{1}) to case4. Expected SAT with B/log1 and its X dependency. If B1 lacks log and B1 is pinned, expected UNSAT. If B is flexible and B2 declares log, selecting B2 is allowed under the global-presence/feature-required interpretation.
6. **Version coherence despite feature sort order.** Root P(B,{1}) and F(B,log,Any), with log1 and log2 candidates and descending version preference. Expected SAT choosing log1, never log2+B1. This catches treating feature versions as independent semantic versions.
7. **Weak gate changes during backtracking.** Parent gate allows B2 at first; B2 triggers a weak log dependency impossible under another root; an allowed B1 does not match a version-filtered gate. Expected SAT B1 with no selected log2/effect. Compare the final result to a fresh direct solve pinned to B1. This is a necessary product-specific test; the upstream backtracking test below does not prove this exact gate-becomes-false shape.
8. **Dormant edge versus independent B.** A has an inactive optional declaration d→B; A/weak refers to d; another root C requires B1. Global-presence meaning activates log; declaration-edge meaning does not. Both are representable; the expected result must come from the owner's chosen contract.
9. **Visibility retraction.** A's feature-supported d→B edge is masked; C independently keeps B1. Edge-bound weak forwarding disappears with d; global-presence weak forwarding may remain. Strong forwarding from a masked declaration must not keep B features alive through an unmasked synthetic feature edge. Masking must follow declaration provenance, not only a synthetic-name string.
10. **No spurious feature closure.** Feature f implies h and h implies f; neither is requested/defaulted/forwarded. Expected selected features empty. Then request f and expect both. Solver output must be checked against the common evaluator's supported closure rather than assuming any satisfying assignment is the canonical feature answer.

Retracting the SAT obligation is guaranteed by candidate-conditioned clauses. Do not overstate that as a proof that every previously considered payload/effect is absent from a final product result: `Solver::solve` returns all positively assigned solvables (`chosen_solvables`), and its API does not promise a globally minimal model. Project effects from the finally selected, justified feature/edge closure and verify that projection against common evaluation. Never publish effects accumulated during get_dependencies callbacks. An integrated fresh-solve comparison is the right regression for abandoned branches; if incidental unsupported selections occur, address their support/projection in the solver owner rather than silently shipping them as installed effects.

## Weak semantics: decision required before freezing tests

Suppose A's declaration d names B with range ^1; C independently selects B2; A's weak g is active.

| Meaning | Guard | Required feature set when guard holds | Result with pinned B2 |
|---|---|---|---|
| Global graph, feature follows actual B | P(B,Any) | F(B,g,Any) | g on B2 if supported; d's range does not govern this use |
| Global graph, declared range remains mandatory | P(B,Any) | F(B,g,^1) | UNSAT; presence activates an incompatible requirement |
| Global graph, only compatible presence triggers | P(B,^1) | F(B,g,^1) | No forwarding; B2 makes the guard false |
| Active declared edge only | support(d) | F(B,g,d-selected-version/range) | No forwarding if d is inactive, regardless of C's B |

PROP-003's current sentence says “already in the graph”; prototype local weak merging checks active dependency declaration IDs. Those are not equivalent. The common choice contract does not decide this by package kind. Root should choose or explicitly distinguish the normalized semantics; no implementation should infer a mode from incidental presence of an alias.

Source identity is another real boundary. Ordinary SAT package names currently distinguish `(group,name)` with one selected version, not two different sources serving the same version. A weak guard on B cannot verify that C's B came from the source intended by d. Reuse the existing package-source selection and captured candidate identity: validate that d's target binding is compatible with the selected candidate, or refuse conflicting source declarations. Only extend candidate identity if a real supported source-selection scenario demands it. A client-supplied domain/instance label cannot make two physical B instances appear to exist.

## Existing VibeVM adapter changes implied by this design

Current `provider.rs` already imports Condition/ConditionId and implements `resolve_condition` by delegating to Pool, but never calls intern_condition for feature behavior. Its `PkgName = String`, `name_parts` splits group/name, and `solvable_parts` assumes every solvable is a real package. Change that bounded internal name dispatch to distinguish Package versus Feature (and only if necessary Edge).

`get_candidates` must route feature names to feature-supporting versions from the same package candidate metadata; it must never ask a registry for a synthetic package. `filter_candidates` can keep the existing SemverVersionSet logic because feature candidate records share their owner versions. `get_dependencies` dispatches synthetic candidates to exact owner binding plus candidate-specific feature effects, while real packages retain ordinary dependencies, application installer requirements, capabilities, conflicts and disjunctions.

The final solve loop in `resolvo_engine/mod.rs` currently calls solvable_parts/manifest_of/build_resolved_graph for **every** returned solvable. It must separate internal feature state from real package graph construction, retaining a selected feature/effect result alongside the graph. Default receipts, user answers and source-definition observations remain request/result data, not synthetic packages in the lock. Synthetic package counts must not leak into public progress or materialization.

All original workspace consumer version constraints remain repeated root requirements, as accepted by F02. Requests for feature atoms are additional constraints, not a replacement for those roots. Held pin fallback, visibility masking, conditional retries and best-effort recommends must rebuild from the same immutable request/definition inputs, with no surviving mutable union from the rejected run. Shared common evaluation still owns cardinality/default/explicit-vs-dormant semantics; solver encoding should have discriminating correspondence tests against that evaluator.

## Relevant primary tests and limitations of their evidence

`tests/solver/main.rs` in this exact resolvo source contains:

- `test_conditional_requirements`: selected foo and bar activate baz.
- `test_condition_missing_requirement` and `test_lazy_conditional_skips_unreached_candidates`: absent gate does not require even a missing target; gated candidate loading remains deferred.
- `test_conditional_requirements_version_set`: choosing bar2 leaves a bar1-guarded dependency absent.
- `test_conditional_unsolvable_without_condition`: newer foo2 rejected; foo1 remains feasible.
- `test_conditional_and`, `test_conditional_and_mismatch`, `test_conditional_or`, `test_conditional_complex`: positive expression composition.
- `test_lazy_conditional_survives_backtracking`: keeps encoded conditional clauses correct across foo2→foo1 backtracking and compares clause count with a fresh run. Its actual fixture keeps `cond` rooted in both solutions, despite broader wording in the preceding comment; it does not prove condition removal after backtracking.
- `test_lazy_conditional_prefetches_forbidden_candidate`: prefetched false candidate clauses remain available if assignment backtracks.
- `test_lazy_conditional_externally_excluded_complement_does_not_fire`: excluding nonmatching cond1 does not count as positive selection of cond2. This guards a lazy/eager semantic difference relevant to weak-range guards.

`tests/solver/bundle_box/mod.rs` shows how conditions and requirements are interned and returned. Its `"...; if ..."` syntax is a **test-provider DSL**, not a new public VibeVM manifest syntax to adopt. `examples/sudoku.rs` demonstrates custom name/version-set types and non-installing constrains; it does not implement package feature forwarding.

No upstream or VibeVM tests were run here. These are primary-source observations plus a proposed encoding and explicit logical test expectations for root's implementation decision.
