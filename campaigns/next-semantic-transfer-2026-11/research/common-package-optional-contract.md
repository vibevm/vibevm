# Optional package dependency contract — decision memo

**Recommendation:** add an explicit boolean `optional` to the existing inline `[requires.packages]` declaration, defaulting to **false**. Preserve every old requires edge as mandatory. An effect that names an ordinary dependency must never make that declaration optional. A selected dependency effect or strong forwarding activates an explicitly optional declaration; weak forwarding only contributes to an already-active declaration of that same owning package instance.

This is a proposed canonical authoring/normalization contract for root judgment, not an implemented feature. No source edits, builds, runtime tests, Git mutations, new agents or network. The active A0 admission writer was not touched. The companion JSON binds current main files and immutable prototype blobs by SHA-256; prototype pin is `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. Existing repo common-package-next-boundary evidence is navigation, not a claim that its earlier A0 bypass remains unfixed today.

## What the prototype actually supplied

- `crates/vibe-core/src/manifest/package/features/effects.rs:1,65-73,127-140` declares a normalized algebra, explicitly **without authored optional wire grammar**. `DependencyDeclaration.optional` is a bool; `FeatureDeclarations::from_requires` receives a separate caller-supplied set of optional declaration IDs. Source/version/link/access/friend/exclude occur once on the declaration, never copied onto effects.
- The normal legacy adapter `crates/vibe-resolver/src/features.rs:209-222` calls `from_requires(..., empty_set)`. Its comment states that every old requires declaration is a required seed and ambient installed packages are not that owner's dependencies. Thus “prototype already accepted `optional=true` in ordinary requires” would be false.
- `features.rs:260-311` seeds nonoptional declarations, activates explicit dependency effects and strong-forward targets, and retains weak requests separately. `apply_weak_requests:130-138` only projects weak requests for IDs in that expansion's active declaration set. Merge invokes that projection again, providing order-independent local weak/strong composition.
- `features/m_05_b_1_cases.rs` contains actual source tests `weak_only_stays_dormant_and_strong_alone_activates`, `weak_forwarding_is_order_independent_and_survives_merge`, and `required_and_already_active_dependencies_enable_weak_requests`. These are algebra witnesses, not public install effects.
- The pinned configured solver still explicitly rejects every `TargetFeature` effect at `resolvo_engine/configuration.rs:395-403`, and nonregistry dependency declarations at `404-413`. Its configured base projection filters optional declarations (`66-84,457-491`) and selected active declarations induce requirements (`629-650`). Therefore local forwarding algebra cannot be counted as completed cross-package configured solving.

Old-task context: M-05-B.1 required optional dependencies/forwarding without a second choices grammar; acceptance says “Normalized algebra has no duplicated choices.requires dependency engine” and “Legacy syntax lowers through one typed representation.” M-05-C.2 requires candidate-local activation and preserving incoming consumer requests/default binding, with graph **and effects** counterexamples. M-05-C.3 requires the same immutable request/catalog across solve/manifest_of/masked/conditional paths and selected-edge provenance. All are in old accepted113, but source inventory classifies C.2/C.3 as **partial**; accepted status is not proof that the forwarding route works. D.1/D.2 still require separate original/resolved persistence and actual contribution activation with source bytes unchanged. No old task is closed by this memo.

## Current main owns the right source location, but not the activation route

`InlinePackageDepWire` (`manifest/package/wire.rs:168-196`) already holds source-independent link/visibility/configuration metadata and rejects unknown fields. `Requires` (`package/capabilities.rs:71-122`) keeps normalized source buckets plus keyed edge metadata. Conversion at `package/wire/requires.rs` preserves these fields across registry, git, path and `version.var`; the original configuration owner validates dangling metadata and duplicate normalized dependency identities.

Current `ConfigurationEffect::{Dependency,Forward}` (`manifest/configuration.rs:147-158`) already refers to a declared target rather than supplying another dependency source. `configuration/validate.rs:208+` resolves that target against the real requires declarations. This is the right authority boundary to retain.

However `Manifest::resolution_requires` (`package/application/requirement.rs:186`) includes every ordinary package requirement. The resolver candidate provider (`vibe-resolver/src/resolvo_engine/provider.rs:257-285`) consumes that unconditional list; main `FeatureExpansion` still does not establish nonlocal effect consumption. Adding a bool and parsing it cannot make inactive dependencies stop reaching these callers. No writer/admission must silently accept optional semantics while those consumers still treat all entries as unconditional.

## Proposed authoring contract

One existing declaration table, no `[choices.requires]` or duplicated `[optional-dependencies]` engine:

```toml
# Package org.example/driver owns these dependency declarations.
[requires.packages]
"org.example/base" = "^1" # mandatory, unchanged legacy meaning
"org.example/backend" = { version = "^2", optional = true, link = "dynamic" }

[[configuration.features]]
id = "enable-backend"
name = "Enable backend"
effects = [{ kind = "dependency", target = "org.example/backend" }]

[[configuration.features]]
id = "diagnostics"
name = "Backend diagnostics"
effects = [{ kind = "forward", target = "org.example/backend", feature = "trace", strength = "weak" }]

[[configuration.features]]
id = "fast-backend"
name = "Fast backend"
effects = [{ kind = "forward", target = "org.example/backend", feature = "fast", strength = "strong" }]
```

A real consuming workspace node supplies the driver's input through the existing edge:

```toml
[requires.packages."org.example/driver"]
version = "^1"
configuration = { policy = "require-selection", features = ["enable-backend", "diagnostics"] }
```

Precise rules:

1. Bare version strings and inline entries without `optional` are mandatory. Explicit `optional=false` means the same; retain its authored presence when round-tripping, as existing edge metadata does. `optional=true` is the only way this declaration becomes conditional. A dependency/forward effect targeting a mandatory declaration is valid and does not alter its mandatory status.
2. The flag belongs to the declaration, independent of registry/git/path/version.var source syntax. Resolve version variables without dropping it. Keep kind assertion, source routing/auth, link and visibility in their existing owners. No source URL/version/visibility payload belongs on the feature effect.
3. A strict boolean only: unknown field/type, null, dangling metadata and duplicate normalized declarations refuse. Existing manifests continue to read and keep their semantics; an old binary may reject a new inline key. “Backward-compatible” here means new readers preserve old source meaning, not that every old binary understands new optional semantics.
4. No implicit feature named after a dependency; no automatic alias or default activation. Optionality is not a user answer or permission to choose a branch. Missing mandatory group input remains a question; unresolved/conflicting evaluation exposes no executable effect set.
5. Local declaration/reference validation includes dormant definitions. Unknown target declaration IDs and malformed local references refuse even if their feature is inactive. Target feature existence/public visibility must be checked against the actual selected target candidate when the edge/forward request is consumed; no installed-newest lookup or client label may substitute another definition.
6. Existing app installer requirements, doc read/cache, remap consent and other kind-specific authorities do not become optional or executable through this field by implication. The ordinary package edge is this contract's owner; supported operation adapters must state their consumption boundary.

Suggested representation: an optional bool on `InlinePackageDepWire`, lifted into source-kind-independent Requires metadata keyed by the existing normalized `group/name` identity. Retaining a bool map entry for explicit false distinguishes authored presence from omission without putting another source record on the effect. The precise Rust name is routine. Do not widen every PackageRef in the system: version queries/application references do not acquire dependency activation authority.

## Normalized meaning and evaluation

A declaration identity is **(actual declaring package instance/candidate, stable local requires key)**. The local key is the existing canonical group/name, independent of declaration order and version constraint; kind remains a checked assertion. Preserve current main literal feature identifiers and explicit visibility, rather than copying the prototype's narrower token/private-underscore grammar.

One normalized declaration contains source, version constraint, optionality, existing metadata and original child configuration intent. The selected effect references that ID and carries cause/provenance, not a second source specification. Complete package effect identity must bind optionality and referenced declaration meaning as well as ConfigurationDecl's fingerprint: changing only `optional` cannot leave a stale prepared graph apparently valid merely because the local feature definition hash stayed equal.

For one resolved owner instance, let:

- `M` be its mandatory declarations;
- `D` be declarations named by selected dependency effects;
- `S` be declarations named by selected strong-forward effects.

The active declaration set is `A = M ∪ D ∪ S`, plus only independently proven activation of a declaration in this **same owner instance**. Target feature requests are selected strong requests plus selected weak requests whose declaration is in `A`. Recompute/merge to the same fixed point regardless of feature/consumer order. Weak requests never add to `A` and never make themselves true.

| Declaration | Selected relevant effect | Required edge? | Forwarded target feature? |
|---|---|---:|---:|
| Mandatory | none | yes | no |
| Mandatory | weak forward | yes | yes |
| Optional | none / weak only | no | no |
| Optional | dependency activation | yes | only any independently selected weak requests |
| Optional | strong forward | yes | strong feature plus any selected weak requests |

The existence of target D in the machine cache, installed set, or another unrelated root's dependency graph is **not** activation of A's optional declaration of D. Example: workspace B requires D, while package A has an inactive optional A→D edge and a weak request for D/trace. B's D remains installed, but A contributes neither that edge nor trace. If another consumer of the *same A instance* legitimately activates A→D, A's selected weak request then applies. This is owner-scoped merging, not global package-name membership.

When multiple actual consumers target one shared instance, retain each original input and merge only active contributions under the accepted exact/additive/omitted/dormant rules. Inactive edge input remains original dormant intent; it does not constrain an unrelated currently active instance through a matching name. Canonical public parameter values must agree; ephemeral secrets retain their separate existing boundary. Conflict reports name the real consumers/declarations.

## Actual workspace and candidate identity

`RootRequirements::collect` (`vibe-install/src/plan/roots.rs:71+`) already takes actual selected node and every discovered workspace node, preserving original constraints. Extend that carrier, not user-authored `domain`, `instance` or owner strings. The workspace root/lock owner identifies the actual shared installation world; actual node-relative declaration paths identify consumers. A selected parent package candidate supplies a transitive consumer edge with its own exact source/definition identity. A different workspace can configure its own instance independently even if both reuse a content cache.

All selected declaration effects must stay attached to the candidate introducing them. A rejected newer version contributes no optional activation, forwarding or default receipt to the selected older version. The same normalized input/basis must reach normal solve, held-pin retry, metadata projection, visibility-masked solve and conditional routes. Reuse the existing solver; unsupported solver/source combinations must refuse before mutation, not silently fall back to an unconditional graph.

## Decisions root must settle before positive activation

1. **Canonical source surface.** Approve the proposed `optional:boolean` on the existing inline requires entry. The prototype supplies the normalized semantics, not a published grammar to copy. No need to ask the user by default; this is a concrete architectural choice within the requested capability.
2. **Owner self-configuration versus a direct install request.** A workspace package's own `[configuration]` is its published interface, not automatically an answer for installing that node's dependencies (the accepted admission distinction). If its own requires contains optional edges, which actual request configures that owner while it is the workspace node? Likewise, explicitly naming a package for install must not silently clear an existing optional flag or manufacture persistent activation. Recommended initial boundary: configured consumed-package instances use the actual incoming requires intent; an operation lacking a real owning activation context returns a typed unsupported/needs-input outcome. Root must specify the real self-configuration/direct-root operation before claiming that route, rather than guessing default consent.
3. **Inactive foreign source validation.** Recommended separation: validate every local declaration and target spelling eagerly, but do not require an inactive optional dependency's remote payload/availability merely to install the active graph. Validate target-feature existence against the candidate once actually activated/consumed. The prototype's transaction catalog may pre-enumerate optional candidates and its configured forwarding is refused, so it is not proof of this complete policy. Root should confirm the desired discovery/preparation guarantee, especially for missing path/git/registry optional sources. This is not permission to skip local malformed/dangling checks.

Strong-forward activation and weak-only nonactivation have direct prototype semantic evidence and current main type comments; they do not need a new universal runtime design. Owner-local versus ambient-package activation should be made explicit in the canonical fact/test so later adapters cannot reinterpret it.

## Six decisive future test families

These are proposed gates, not tests run here. Reader tests prove the declaration foundation; actual CLI/solver/lock/boot outcomes prove delivery.

1. **Backward-compatible mandatory edge.** Actual install with existing string and inline requires, `optional=false`, and a disabled feature that mentions the dependency: dependency remains selected/materialized and retained on cold bare install. Reader round-trips registry/git/path/version.var, explicit false, link/visibility/configuration metadata; rejects wrong bool/dangling/duplicate IDs. Merely adding a dependency effect must not remove a legacy mandatory edge.
2. **Optional off/explicit/strong.** Real package A declares optional D. With no activation, D has no selected edge/slot/hook; selection of dependency effect activates it, and strong forwarding alone also activates it. The emitted lock explains the actual declaration and consumer cause. Remove activation and preserve D if an independent root still requires it; never prune full source bytes based on feature choice.
3. **Weak is owner-local and order-independent.** Another root already installs D; A's weak-only optional A→D must not turn on D/trace or create A's edge. Let trace itself induce an observable leaf dependency/boot contribution. Activate A→D explicitly, then trace/leaf must appear. Reverse declaration and consumer order and obtain the same graph/effects. This avoids a field-only weak_dep_features assertion.
4. **Shared consumer intent and dormant replay.** Two real workspace members consume the same A instance: additive compatible inputs union, active incompatible exact selections name both consumers and write nothing. Switch a guard off/on across fresh processes; original dormant explicit input remains and reactivates faithfully. A removed dormant option reports reconfiguration rather than filtering it away.
5. **Candidate rejection/default identity.** A newest candidate's selected optional edge makes its version unsatisfiable; an older compatible candidate is chosen only with the same stable explicit answer. Assert final graph and actual effects have no rejected candidate residue. A changed shown/default basis yields rediscovery/stale instead of selecting a different semantic branch. Exercise masked/conditional retry with the same normalized consumer set.
6. **Source/replay boundary and inactive availability.** An unselected optional source is unavailable while the required graph is available: under the root-approved policy it must not become a required fetch/effect. Activating it then gives a truthful pre-effect source/selection error. After a successful activation, cold replay/update preserves source/definition/optionality and original intent; same-version declaration drift cannot pass Fresh. Cover the actual supported registry/git/path/version.var routes or explicitly refused routes without claiming universal support from one fixture.

## Delivery boundary

A normalized optional declaration plus parser tests is a useful **schema/declaration prerequisite** only if it has a real enduring owner and unsupported consumers continue to refuse. It is not configured install completion. Positive delivery still requires candidate-bound evaluation, actual optional/forwarded requirements in the selected graph, original-versus-resolved lock semantics/freshness, real effects and ordinary user operations. No new solver, store, environment framework, data-only milestone or restored install-time subskill copying is proposed.
