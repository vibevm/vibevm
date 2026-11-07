# Source pins: feasibility against the installed solver

Read-only check, 2026-11-07. No code, dependency edits, build, tests or network. This is source evidence and a design limit, not a runtime verdict or a blocker for the whole campaign. Exact source SHA256 and evidence locations are in the companion JSON. Earlier policy/solver plans remain drafts.

## Result

The existing resolvo integration can represent **a source-bearing candidate and an explicit positive pin constraint** without another solver. It does **not** currently offer the public absence/negation or conditional-constraint API assumed by the earlier “active pin witness” proposal. I have not proved a complete encoding that simultaneously preserves default first-registry semantics, admits valid transitive pin solutions, and avoids fabricated pin witnesses. Do not implement or expose that proposal as settled architecture.

The version is confirmed as resolvo 0.11.0 in `Cargo.lock:3258`; `crates/vibe-resolver/Cargo.toml:17` selects the workspace dependency. Dependency source observations are from the explicitly authorized local cargo registry directory, not a claim of pristine upstream source.

## Actual APIs and their limits

Paths abbreviated below: **R** = `/home/olegchir/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/resolvo-0.11.0/src`; **V** = `/fast/git/vibevm/crates/vibe-resolver/src`.

| Observed declaration | Exact source | Consequence |
|---|---|---|
| Solvable has generic record V and package NameId | R/utils/pool.rs:13, :18, :33, :154 | A record `{version, canonical source binding}` is supported; keep one package NameId. No new resolver is needed for this representation. |
| Filter sees candidate IDs + VersionSetId + inverse | R/lib.rs:130 | A version-set predicate can match both semver and registry identity. It has no chosen-graph argument. |
| Candidates lists all candidates, favored/locked are candidate IDs | R/lib.rs:149, :174 | favored is a preference, not an active-pin permission. A static locked value cannot follow conditional incoming pins. |
| Conditions are Requirement(VersionSetId) or Binary(And/Or) | R/conditional_requirement.rs:21, :34 | No Not, absence predicate or arbitrary callback into current chosen graph. |
| ConditionalRequirement attaches a positive condition to a requirement | R/conditional_requirement.rs:7; R/requirement.rs:24 | This expresses “if selected matching candidates, require …”, not directly “if no selected pin carrier, enforce default domain.” |
| KnownDependencies.requirements is conditional; constrains is Vec<VersionSetId> | R/lib.rs:254, :261, :274 | No public conditional-constrains field. Ordinary constraints apply with their owning solvable and can constrain an already selected target without pulling it in. |
| Problem offers requirements, constraints and soft requirements | R/solver/mod.rs:103, :115, :137 | No public general clause injection/no-good callback was found on this inspected Solver/Problem surface. |
| Clause enum and Literal are crate-private | R/solver/clause.rs:50, :654 | Internal SAT machinery is not an API the Vibe adapter can call. |
| Condition encoding uses complement and internal selected auxiliary | R/solver/conditions.rs:25, :35, :46 | Internal complement handling does not expose absence as a public Condition; internal one-way selected auxiliaries are not a user-facing exact pin-witness API. |

Qualification: a positive conditional requirement targeting an empty candidate set can express a guarded **refusal**. Therefore “no conditional-constrains field” is not a proof that all conditional constraints are mathematically inexpressible. The missing piece here is selected-edge **absence/default applicability and supported pin authority**, not a generic inability to encode contradictions.

## Current Vibe seam

* `V/resolvo_engine/version_set.rs:16` defines Any/Req/None/Explicit over semver only; `:46` sets associated record type to semver::Version.
* `V/resolvo_engine/provider.rs:36` holds the pool, `:38` caches candidates by NameId; `:214` returns that cached set; `:241` interns semver-only solvables; `:247` sorts descending semver; `:264` fetches metadata by group/name/version; `:280` reads resolution_requires and `:288` interns only semver constraints. This is where source-bearing candidate identity and pin-bearing edge predicates would go.
* `V/lib.rs:224`, `:227`, `:283` expose resolve_version, fetch_manifest(group,name,version) and list_versions(group,name); they cannot name an exact source candidate. `:238` selected validation explicitly does not change selection. `V/resolvo_engine/mod.rs:233` invokes it after building the resolved graph. This hook alone cannot backtrack to a valid alternative source/parent.
* `V/resolvo_engine/capabilities.rs:44` prescans versions/manifests, skipping failed observations at `:58`/`:62`; it is a discovery index, not an authoritative set of selected edges. Using this prescan to accumulate pins would admit pins from discarded candidate versions.
* `V/conditional.rs:33` has Not for **activation-context** predicates, but that is a different owner/API; it cannot be passed to resolvo Condition. Its evaluation is over ActivationContext (`:93`), not the engine's decision trail.
* `V/naive.rs:153` reconciles an already chosen version and fails on conflict; `V/sat.rs:185` starts bounded naive attempts, and `:207`/`:238` handle version conflicts/exhausted bounds. The sat cell's decreasing-version bound is not a source/pin backtracker or a ready-made immutable pin-assumption engine.
* `crates/vibe-registry/src/multi_registry_resolver/walk.rs:71` stops at the first nonempty version list (`:77`, `:95`). Full-registry candidate union would change that behaviour unless the new relation retains default eligibility correctly.

## Counterexamples that reject shortcuts

1. **Default domain as a hard condition on every unpinned edge is wrong.** A requires X^1 unpinned; selected B requires X^1 from R; default registry has 1.2 and R has 1.5. R/1.5 is valid. An unpinned edge contributes its semver condition; active pin controls source/version domain.
2. **Enumerate every registry and merely sort default first is wrong.** Root X>=1.4, default registry offers only 1.2, R offers 1.5, no active pin. Previous resolution fails; a broadened fallback candidate succeeds. Sorting is not eligibility.
3. **Accumulate encountered pins globally is wrong.** B2 pins X to R but conflicts elsewhere; solver backtracks to B1 with no such pin. R must cease constraining/admitting X. Discovery or metadata cache is not selected-edge authority.
4. **X outside default requires a synthetic pin token is wrong without support proof.** The solver can choose the token to satisfy X; nothing proves an active authored edge created it. Making the token require B turns a policy witness into an installation dependency and can pull in B merely to justify X. With B→X, the pair can self-support. No such reverse dependency was authored.
5. **Require an actual parent candidate instead of a token is still wrong.** In an A→(B or C) world, C may satisfy the authored requirement, while X's synthetic witness additionally pulls in B just for B's pin. The solver solves the artificially expanded graph, not the requested one. Ordinary union semantics do not forbid both alternatives.
6. **Post-solve pin refusal is incomplete.** It can reject B2 while B1 would make the same request valid; without feeding a correct scoped conflict back to the existing engine it never explores that alternative. Starting a fresh solve with a sticky pin set does not fix it.
7. **Adding only a Not enum arm is not a proved fix.** Even if absence becomes expressible, supportedness and graph-bound pin authority still need proof; an encoding must not let an unrelated/unsupported carrier be selected solely to disable default policy. A dependency change must not be proposed as a one-line solution.

## Simplest concrete representation that is actually supported

Use one existing package NameId and a source-bearing candidate record; represent a pin as a candidate filter containing registry identity alongside the semver predicate. Fetch/capture its manifest by candidate identity and retain it through ResolvedNode/acquisition, never look up metadata again by version alone. An **immutable pin supplied by a known active root** can constrain the candidate domain before solve; equal pins intersect, incompatible pins produce disjoint sets. Positive transitive edges can likewise require their pinned target candidates. These are representable building blocks, **not a complete implementation of default-domain release under arbitrary selected transitive pins**.

No public pin grammar should ship on the strength of only that root-pin subset. No proposal here changes the owner requirement to root-only pins.

## Minimal next proof steps

1. Keep the independent exclusion/preference facets progressing with actual consumers: contact admission before managed probes/fetch, and fixed-version equivalent-source ordering. They do not require solving the arbitrary transitive pin-witness issue. Preference-only must retain current version enumeration. Do not expose pin grammar yet.
2. For pins, extend the **existing provider/candidate seam**, not a new solver. Before code, write a finite table of candidates, authored requirements and expected selected solutions for the seven counterexamples above. Include incompatible diamond pins with a valid parent-version alternative, optional/conditional edges and a valid pin that makes an otherwise default-unsatisfiable graph satisfiable.
3. Check one narrowly specified extension to the current resolvo adapter/library contract: candidate eligibility conditioned by **selected authored incoming edges with solver-owned backtracking lifetime**. The required semantics are exact support and default applicability; whether the implementation is an extended conditional-constraint mechanism or a solver-owned source-constraint relation is unresolved. Inspecting private clause code establishes where the current API stops, not authorization to fork/modify resolvo.
4. If evaluating immutable-assumption reruns instead, require a complete scoped conflict and termination argument. Existing public API lacks arbitrary learned no-good injection; do not claim the current sat cell supplies it. A fresh fixed-point solve loop alone fails the no-initial-solution and discarded-pin cases. Reject any proposal that becomes a second search framework merely to avoid extending the owning seam.
5. Only after an encoding proof, run bounded synthetic solver tests and then registry-integrated cases under a separately authorized implementation packet. This audit executed none. Public grammar waits for the correct whole-closure consumer.

At this checkpoint there is no proved complete winning encoding. The useful result is the precise API boundary and the eliminated wrong designs. Root can continue source facts, exclusion, preference and other campaign work independently.
