# Candidate-conditioned consumers: bounded next-boundary review

Read-only source assessment, no tests/builds/source changes/Git mutations/network. Current fixed-consumer implementation remains useful and is not blocked by this next-stage design. Core packet path resolves to `crates/vibe-core/src/manifest/configuration/package.rs`. Previously named90-user/PROP069 standing applies; current package-positive design was read. Prototype evidence remains the already reviewed immutable7fe6376 pin, which explicitly refuses target forwarding; no dirty prototype was consulted.

## Source-proven boundary

- `vibe-resolver/src/configuration.rs:9–26,83–117` keys one original/evaluation/view by `(group,name,version)` and evaluates against immutable workspace inputs. That key is sufficient only while incoming consumers are fixed. Lines74–78 deliberately refuse a selected package edge into a configured target; lines92–103 refuse missing/unsupported activation contexts. These are honest initial limits.
- `vibe-core/src/manifest/configuration/package.rs:79–108` normalizes all consumer inputs, unions ordinary feature seeds and invokes the shared choice evaluator once. Lines113–138 activate dependencies from mandatory declarations and selected effects, while forwarding/active transitive configuration deliberately refuse. `project` at161–175 is a derived selected view and removes raw configuration metadata; it is not the source for recovering incoming intent later.
- Existing `VibevmResolvoProvider` (`resolvo_engine/provider.rs:36–42,239–244,258–288,369–373`) interns semver-only solvables and caches the manifest/dependencies by SolvableId. It is the concrete place where parent-candidate requirements enter SAT. Output at `resolvo_engine/mod.rs:178–234` maps each selected solvable through that same cached view into one package graph; extension must preserve this association, not re-evaluate against a later mutable map.
- Installed resolvo0.11 `DependencyProvider::get_dependencies` receives only SolvableId (`src/lib.rs:127–150`), not the current selected assignment. `solver/cache.rs:336–365` memoizes each solvable's dependencies once. Mutating incoming consumer maps cannot invalidate those clauses on backtracking.
- Installed `KnownDependencies` has conditional requirements and optional presence constraints (`lib.rs:254–277`). `Condition` provides positive Requirement plus And/Or (`conditional_requirement.rs:7–39`), with no public Not/absence condition here. This is useful machinery, not proof that exact consumer-support equivalence is already expressible by one conditional edge.

## Minimal seam to extend next

Keep source observations keyed by coordinate and the existing operation snapshot. Separate that immutable source identity from **configuration state identity**: a solved package state needs the candidate plus its canonical actual incoming declarations/contributions. Extend the existing resolvo bridge/interner/version-set handling so distinct immutable evaluated states cannot share a SolvableId. Selection must still choose at most one actual version/configuration state for the real `(group,name)` instance. Ordinary version constraints filter the physical version; they must not create independent per-consumer installations.

The core adapter remains the single merge/effect authority. It needs a real forwarded-contribution input alongside original authored child inputs, with owner-candidate/declaration/feature provenance; do not fabricate an authored profile/default policy merely to pass a forwarded feature. Record the contribution separately, validate target feature existence/public visibility against the actual selected target candidate, and merge all real incoming inputs in one evaluation. Add typed package-consumer identity when this actual consumer exists; do not retrofit unused future types into the current slice.

Logical invariant, for selected target candidate Q:

`inputs(Q) = fixed workspace inputs(Q) + exactly the inputs/contributions of selected parents whose own declaration to Q is active`.

This must be equality, not just “contains every demanded input.” Extra unsupported inputs can otherwise manufacture answers and activate dependencies. Each selected parent-state/declaration forces a compatible Q-state and version constraint; Q-state must neither omit a selected consumer nor claim a consumer from an unselected parent. Conflicting exact inputs reject that joint assignment, rather than installing two Q variants. Candidate-local default observations remain provisional; only the final selected state binds fresh consent under the newly accepted default policy. Retained/shown basis never renews through search.

Strong forwarding makes that declaring parent's dependency active and contributes its feature. Weak forwarding contributes only if that SAME parent's declaration is already active from mandatory/dependency/strong causes. Ambient Q presence from another root is irrelevant. Preserve the activation reason separately from target presence so weak forwarding cannot bootstrap its own edge.

## What is still an encoding hypothesis

Immutable configuration-state solvables plus guarded requirements are a plausible seam, not an implemented production algorithm. Full enumeration of all input subsets is potentially exponential and is not a recommendation. On-demand variants also cannot be appended invisibly after resolvo has cached a name's candidates/dependencies. The implementation must prove a finite, stable discovery/state interning strategy before claiming completeness.

Support/absence is a real missing proof: parent→target compatibility alone lets a target state claim phantom extra consumers. Requiring a supposed consumer's parent backwards can spuriously install that parent, creating authority that the real dependency graph never supplied. Positive Condition and constrains must not be advertised as an already proven solution to that problem. If the installed public bridge cannot express the needed support exclusion, identify the narrow additional condition/constraint capability explicitly; do not hide a second solver or a global mutable fixed-point loop behind it.

## Counterexample to solve / collect inputs / re-solve

Root A admits versions2 and1. A2's real child declaration asks Q for exact red; A1 asks Q for exact blue. Another fixed workspace consumer requires that same Q instance exact blue. An ordinary unconfigured solve can choose A2. A later merged evaluation then conflicts, although A1+Q/blue is valid. Treating that conflict as a global error misses the solution; retaining A2/red in a mutable input map while retrying falsely rejects A1. Replacing the map does not repair dependency clauses already cached for the same Q SolvableId. Conversely independently satisfying red and blue under different synthetic Q names violates the single real instance. Correct conditioning retracts A2's input when A2 is rejected and evaluates only the selected joint state.

## Small later proof before broad activation (not run here)

Use the SAME resolvo engine with a finite local fixture and explicit no-auto input first:

1. A2/red versus A1/blue and fixed Q/blue above: selects A1, exactly one Q, no A2 input/effect/default residue in final record; reverse candidate discovery order does not alter meaning.
2. Selected A has inactive optional Q and selected weak forwarding; independent root B requires Q. A's weak contribution remains absent. Activating A's own declaration makes it appear; strong forwarding activates it without borrowing B's presence.
3. A target with a missing mandatory answer must remain NeedsSelection, not be satisfied by a state claiming an unselected parent as consumer. This is the support/absence discriminator, not merely a happy-path merge test.
4. Two actual selected consumers with incompatible exact inputs must fail unless a legitimate parent/version alternative resolves the conflict; no duplicate physical Q state. Selected graph/record use the same state cache; rejected recommends/masked attempts cannot leave inputs behind.

This bounded proof can hand-enumerate a tiny finite state model to test the logical seam, but that is not evidence of a production-scale state-discovery strategy. Defaults, source-mode coverage, durable replay and general forwarding remain separately required once the seam is demonstrated. No current writer scope is expanded by this memo.

## Source fingerprints

Current sources are under active primary implementation; these hashes delimit the reviewed checkpoint, not final acceptance.

- `/fast/git/vibevm/crates/vibe-resolver/src/configuration.rs`: `aeae867617793d618cd9f0638bd6b34dfd40b8f78b58bbc0c26d529cf8657fed`
- `/fast/git/vibevm/crates/vibe-resolver/src/resolvo_engine/provider.rs`: `e11e125766710cd5277a3ffbe3a3bbad206100890d62167b61443b06a9c0d874`
- `/fast/git/vibevm/crates/vibe-resolver/src/resolvo_engine/mod.rs`: `9bb5a41e4fb2a4f88f31c930a84f9e2eb7b282ecdd3faaccb151172fb3e92fea`
- `/fast/git/vibevm/crates/vibe-core/src/manifest/configuration/package.rs`: `3612829529632d7937bc35a1b7b05a731203d9bf6c86c34b6b612301dbfbc8c1`
- `/home/olegchir/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/resolvo-0.11.0/src/lib.rs`: `30946db989adddbd25f3c474f57167c664b6f699f235046969b772de8b6f5ec7`
- `/home/olegchir/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/resolvo-0.11.0/src/conditional_requirement.rs`: `56251448173bc4a675a9ba48ab4e45c1a88ace4e4d3c261a249b172d3a39828d`
- `/home/olegchir/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/resolvo-0.11.0/src/solver/cache.rs`: `06f29fd4f867f88c02bf05e26c6bc1c41438f6fe4a6b1251301d167d676e72e0`
