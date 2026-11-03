# Common package choice integration: source observations and design input

Read-only source review, 2026-11-04. Main `c20b6bc8945ad4ff971db4ba8a2e9b1aa8645424`; prototype `/fast/git/v/vibevm-next` pinned `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. Read the named standing user rules, common configuration contract PROP-069, the accepted install consumer constraints in PROP-009, and relevant feature/subskill contracts in PROP-003. No builds, source/spec/map/state/Git edits or new workers. Main's existing dirty campaign log and untracked web-doc slot were left untouched. Prototype observations used pinned Git objects, not dirty files.

## Observed: the four reported effect outputs have no main consumers

An `rg` scan of main `crates/**/*.rs` finds `active_deps`, `dep_features`, `weak_dep_features`, and `active_subskills` only in `vibe-resolver/src/features.rs`: their definitions, production expansion/merge writes, and unit-test assertions. `Fetched` stores the whole FeatureExpansion, but no downstream reader accesses those four fields. There is no caller of FeatureExpansion::merge establishing cross-package unification. The comment in features.rs claiming that a higher solver layer calls it with unified features is not backed by the current source path.

The fifth field, `active_features`, does have readers: install plan's unmatched-feature warning; `record::locked_package_from_fetched` writes `LockedPackage.features`; `apply/lock::record_lock` writes qualified `meta.active_features`. These are recording/reporting consumers, not proof that dependency/subskill effects ran.

The ordinary pipeline is currently:

1. `InstallSource::solve(&[PackageRef]) -> ResolvedGraph` solves unconditional manifest dependency edges.
2. `plan/fetch.rs::fetch_node` fetches each selected node, then calls `expand_features(&cached.manifest.features, &req)`. Root requests are one global `FeatureRequest` filtered by `tailor_feature_request`; transitives receive FeatureRequest::default().
3. Later conditional expansion handles `[target."context(...)".dependencies]` separately, based on package context. It does not consume the four feature-effect fields.
4. Apply materializes the full package tree, and records active feature names. It does not persist original consumer feature/default intent.

### Important qualification: subskill inactivity is deliberate compatibility behavior

`record.rs` explicitly writes `subskills_active: Vec::new()`. `vibe-cli/tests/cli_registry_mgmt.rs::install_with_features_records_active_features_in_lockfile` and comments around the removed omnibus subskill cases state that the PROP-009 loading model retired install-time per-file subskill projection. All package bytes, including subskills, live in the verbatim dependency slot.

There are legacy consumers of **LockedPackage.subskills_active**: `vibe show subskills`, `show purls`, list output, and MCP QueryPackage/ReadSubskill/MaterialiseSubskill tools. They do not read FeatureExpansion.active_subskills; normal current installation does not populate their legacy rows. Restoring the four missing consumers must not silently resurrect copying chosen files into the project's authored spec tree.

## Observed: exact current seams

| Owner/file | Existing signature or data | Integration consequence |
|---|---|---|
| `vibe-core/src/configuration/model.rs` | `FeatureModel::new(Vec<FeatureNode>, Vec<ClosureGroup>) -> Result<FeatureModel, DefinitionError>` | Validated local closure model already exists; do not build a second package-only closure evaluator. |
| `vibe-core/src/configuration/resolve.rs` | `FeatureModel::resolve(&FeatureInput) -> Result<FeatureResolution, EvaluationError>` | FeatureBase::Defaults versus Selected, additions and disabled are explicit inputs. It currently provides closure groups, not the pending richer consumer-choice/public persistence layer. |
| `vibe-core/src/manifest/package/features.rs` | `FeaturesTable { features: BTreeMap<String, Vec<String>>, exclusive: BTreeMap<String, Vec<String>> }` | Legacy activation strings and exclusive sets need a faithful compatibility adapter to common data. |
| `vibe-core/src/manifest/package/wire.rs` | `InlinePackageDepWire`, deny_unknown_fields; version/path/git/tag/branch/rev/auth/link/access/friend/exclude | No current optional/features/default_features dependency fields. A solver cannot connect optional effects to authoritative declarations using syntax that main does not parse. |
| `vibe-resolver/src/features.rs` | `expand_features(&FeaturesTable, &FeatureRequest) -> Result<FeatureExpansion, FeatureError>` | Current output separates active dependency refs, strong/weak feature forwarding and subskill paths, but main lacks declaration binding and downstream execution. |
| `vibe-resolver/src/lib.rs` | `DepProvider::{resolve_version(&PackageRef), fetch_manifest(&Group, &str, &Version)}`; `VersionEnumerator::list_versions(...)` | Current metadata result is only Manifest. It loses the exact read's source/definition/content observation. |
| `vibe-resolver/src/resolvo_engine/provider.rs` | `get_candidates(NameId)`, `get_dependencies(SolvableId)` | Candidate is `(group,name,version)`. Candidate dependency collection is the place effects must influence SAT, before final version selection. |
| `vibe-core/.../application/requirement.rs` | `Manifest::resolution_requires() -> Vec<PackageRef>` | Adds ordinary package requirements **and installer application requirements**. Configured projection must preserve this incumbent behavior, not replace all requirements with a package-choice list. |
| `vibe-install/src/lib.rs` | `InstallSource::solve`, `manifest_of`, `solve_masked`, `resolve_and_fetch[_with_progress]` | Needs a small request/result-aware configured companion or extension. The current graph-only return cannot carry selected candidate evaluations/default bindings to fetch/apply. |
| `vibe-package-source/src/source.rs`, `cells.rs` | InstallResolver implements those methods; `metadata_manifest_cell(resource, pkg)`, `solve_masked_cell(flags, resource, roots, blocked)` | Actual local/multi/embedded provider selection remains here. All retry routes must share the same original intent and candidate observations. |
| `vibe-install/src/plan/roots.rs` | `RootRequirements::collect(workspace, selected, requests)`, `authored`, `held`, `package_count` | Already owns consumer identity and exact selected-node replacement. Extend its data flow for per-consumer choices rather than replacing it with a coordinate-keyed map. |
| `vibe-install/src/plan/fetch.rs` | `fetch_node`, `expand_conditional_deps(... roots, held_roots, ...)`, `sync_fetched_to_graph` | Consume the chosen evaluation here; do not expand defaults again after solve. Recomputed routes must discard effects for rejected/replaced candidates. |
| `vibe-install/src/record.rs` | `merge_manifest_requires`, `workspace_root_snapshot`, `locked_package_from_fetched` | Selected consumer authoring and generated result remain distinct. Unique root_dependencies compatibility rows cannot retain per-consumer choices. |
| `vibe-install/src/apply.rs`, `apply/lock.rs` | selected manifest/FileGuard, post-edit workspace checks, materialize/boot, lock and provenance guards | Reuse these owners for new authored answers and generated selection. No parallel global configuration store or generic journal is necessary. |

## Observed: prototype's useful behavior and its actual limits

Prototype `resolvo_engine/configuration.rs` exposes:

```text
CandidateConfiguration::from_metadata(&CandidateMetadata, ConfigurationInstance)
    -> Result<Option<CandidateConfiguration>, DepProviderError>
DefaultBinding::shown(edge: usize, request: &Request, candidate: &CandidateConfiguration)
ResolvoDepSolver<&TransactionCatalog>::solve_configuration(
    roots, request, definitions, defaults) -> Result<ConfiguredResolution, ConfigurationSolveError>
solve_configuration_routes(..., blocked_edges, retained_choices)
```

Its ConfiguredCatalog stores each candidate's projected Manifest and Activation independently. The candidate's full required/activated dependency set is rebuilt, not appended to the previously inspected version. Rejected candidates have named reasons. Selected outcomes return graph, original request, selected effects, manifests and provider choices. NeedsSelection/Rediscovery is reported for selected candidates rather than being mistaken for successful empty expansion. Unbound/stale default permission is not allowed to add solver implications.

Those invariants are valuable. However, the same entry explicitly refuses cross-package `FeatureEffect::TargetFeature` forwarding, non-registry dependency declarations, conditional targets, obsoletes, and multiple configuration instances of one package. A forwarding unit test elsewhere does not close this integration gap. It also creates an internal known partial expansion for unresolved choices; a new design must keep that internal-only and publish no executable result under PROP-069#CHOICE-CORE.

Prototype `authored_configuration.rs` constructs a JSON wire request with domain.definition/target/producer/role/identity, instance and configuration wrapper fields from each authored row. Those fields are not a minimal prerequisite for a single shared workspace package instance. They should not be imported merely to satisfy prototype signatures. Its useful part is reading original consumer authoring from **every** workspace node, not from resolved lock choices.

Prototype `fetch_node` takes the chosen `route_resolution().effects[coordinate@version]` instead of recomputing a fresh default FeatureRequest. This is the correct consumption boundary. Its extra catalog/runtime framework is not required to express that ownership rule.

Prototype `bootgen/effective_subskills.rs` demonstrates the right loading-model direction: selected exports are bound to package/content, eager content enters the effective boot lane, lazy-push enters a declaration/description view, lazy-pull stays outside boot and is read explicitly. The full original package slot remains intact. Its source-query/capture infrastructure need not be copied wholesale to implement that behavior through main's existing slot/boot owners.

For config-value effects, prototype install record serializes them and a machine choice handler exposes them. A source scan of prototype install/workspace/orchestrator/lifecycle found no actual runtime config-effect consumer beyond recording/inspection. This is another unfinished operation boundary, not an implementation to transfer blindly.

## Inference: where each effect must connect

### Dependency activation

Bind each `dep:` target to a declared dependency identity and source; no bare-name guess across groups. Main FeatureValue::parse splits forwarding on the first slash, so qualified `org.example/pkg/feature` needs an explicit alias/identifier rule rather than reusing that split unmodified. Typed declaration validation must include dormant branches. Legacy dependency grammar currently lacks optionality; introduce the smallest format extension in the existing manifest owner, with a single normalized declaration set, rather than a second resolver language.

Apply optional activation when producing **candidate dependencies** in resolvo's provider, before solving. `get_dependencies` must see the candidate's evaluated active edges together with its ordinary application requirements/capabilities/disjunctions. Acquiring an optional dependency later in install fetch cannot prove that the chosen package version was jointly satisfiable.

### Strong forwarding

An active parent candidate's `dep/feature` must both require the target dependency and contribute that feature request to the target's **actual shared instance**. Merge only edges of the selected parent candidates; rejected newer parent candidates must contribute nothing. This is not supported by calling the pure evaluator over a static workspace-root request list once.

The implementation decision belongs in the existing solver owner. Candidate-conditioned feature requirements can be represented in the existing SAT model, with the common evaluator validating local closure/consumer choices; alternatively a proven solver-owned propagation/backtracking strategy could preserve the same condition. A global mutable union inside fetch_manifest or a post-solve monotone append loop is insufficient: when a parent version is rejected, its forwarded features and optional dependencies must retract. No new standalone solver is needed, but the graph-only provider interface is not enough for the full feature semantics without this integration.

### Weak forwarding

`dep?/feature` contributes only when the referenced dependency participates in the applicable selected graph/declared edge meaning. It cannot create that dependency merely to satisfy the weak request. A pre-solve `graph.contains` test is unavailable; a union of all catalog candidates is wrong. Test both absent and independently introduced target plus backtracking removal. Root should pin whether “already in graph” follows the historical global graph wording or a stricter parent-edge condition before implementing: prototype's local `apply_weak_requests` checks active dependency IDs within an expansion, while PROP-003's prose says already in graph. The new contract should not silently substitute one for the other.

### Subskills

Keep materialization verbatim. Carry validated selected export/path identities plus original package content identity into existing `ResolvedDep`/boot lowering and durable selection data. At `bootgen::regenerate_boot_from[_with_spec_format]` and the node's effective lane computation, apply eager/lazy-push/lazy-pull semantics. Update read/list/MCP consumers to read that same selection and slot identity. Legacy lock subskill rows remain a compatibility reader; filling them without the new truthful delivery/path contract is not sufficient.

The existing `vibe-resolver::activation::evaluate(&ActivationRules, &ActivationContext, Option<&str>) -> ActivationOutcome` is a separate host-context predicate owner. It includes file/command/env probes and an explicitly unimplemented if_os branch. Do not move those ambient probes into the pure choice evaluator or infer manual feature choice from them.

### Config effects and parameters

Main FeatureValue has no config effect variant, and the pure FeatureResolution only carries selected features/reasons. A typed config effect needs declaration validation and a conflict-free evaluated value map, followed by an explicit **named domain consumer**. Existing app `PlanContext`/parameter/step owners can consume their own validated values; package extension/runtime config uses its existing host-owned fields. A generic map in a lockfile is not an executed config effect. Do not reinterpret user parameters as feature IDs or silently map arbitrary package config keys onto extension execution. Until an effect target is supported, return a typed unsupported outcome before mutation; public surface parity must describe that support boundary honestly.

## Existing identities that can bind shown defaults

**Observed main identities:** package identity is `(group,name,version,content_hash)`; `CachedPackage` carries that plus parsed Manifest, source_uri, fetched_from, registry_name, source_ref and optional resolved_commit. LockedPackage records content hash/source/commit after acquisition. `MultiResolution` carries source route/version/source URL/ref and flags **before fetch**, but not a content/definition digest. `ResolvedPackage.source_dir` is a path, not content identity. `source_url` is explicitly informational; mirror switching does not change package identity.

Main metadata methods return only Manifest. `GitBackend::fetch_file_at_ref(url, refname, path)` returns raw bytes, not a commit-bound observation; `head_commit(dest)` is available for an actual checkout. `GitPerPackageRegistry::fetch_manifest_at_ref` reads at a possibly mutable tag/branch and discards the raw byte witness after parsing. `MultiRegistryResolver::fetch_manifest` re-resolves a pinned version; same version does not mean same source bytes. Main therefore has **no already-complete candidate receipt at the solver metadata boundary**. Claiming one from coordinate+version would be false.

**Minimal inferred extension:** retain the definition bytes/fingerprint at the actual metadata read and return them with that candidate's existing coordinate and observed source provenance. Where a fetched CachedPackage already exists, reuse its content_hash and manifest; where an immutable Git commit or checked local tree hash is actually available, retain it. Where only a metadata read exists, label it definition observation and recheck against fetched candidate bytes before effects; do not fabricate a whole-tree authentication guarantee. Prototype `candidate_metadata.rs::{CandidateIdentity { source,content,definition }, CandidateMetadata::from_bytes/read_local/verify_checkout}` is a useful bounded reference for this seam, independently of its whole TransactionCatalog architecture.

A reviewed-default binding minimally needs the actual consumer/dependency identity, shown normalized definition/default/effects semantics (or a versioned deterministic digest), original omission/exact/additive policy and the candidate observation it was shown from. It is a stale-input receipt, not an execution grant. Re-evaluate explicit stable answers on an older candidate only if its definition supports them; a binding to one shown default must not consent to another candidate's different default. Full raw-manifest hashing is conservative and may cause rediscovery on comments; a semantic digest can reduce that only with an explicit recipe covering all definition/effect semantics. This choice does not require environment/lock/target/domain wrapper identities or a running host service.

## Multi-consumer and persistence constraints to keep

- Reuse the accepted RootRequirements owner and selected-node identity. The same package coordinate is one shared workspace installation; workspace member IDs are real consumers. Do not let an arbitrary request instance string split siblings into independent installations.
- Per-consumer version and choice intent stay paired. Explicit version replacement remains selected-node-only. A configuration answer updates only that consumer's authored configuration; omitted new input must not erase previous explicit/dormant intent.
- Held pins are optimization constraints, not new consumer answers/default consent. Candidate retries, visibility masking and conditional expansion must carry the full original per-consumer intent and all base version constraints. Rebuild the effective candidate/effect result for the selected versions; never merge rejected expansions.
- Visibility masks can remove a dependency **and its forwarding contribution**. Preserve the route's actual declaration origin so a blocked edge cannot keep activating target features. A coordinate-only edge mask cannot explain two distinct feature-induced declarations without an explicit mapping.
- Keep generated `root_dependencies` unique and conservative, as F02 does. Never read it back as rich choice intent. Persist original per-consumer answers in the selected authored manifest using the existing edit/write owner; persist selected normalized result/content binding in the existing lock owner with an intentional schema compatibility change. Current closed schema7 does not permit arbitrary new fields silently.
- Carry chosen evaluations/default witnesses in PlannedInstall/Fetched or an equally small plan-owned value to apply. Do not reread the old raw manifest and drop an interactive answer before recording it. The existing FileGuard/slot/lock rollback boundary should cover the new authored/result writes.
- Freshness must include configuration intent/definition applicability before returning Fresh. Current `freshness::check` has no feature/choice checks. Same version constraints with changed choices cannot reuse the old effective graph. Existing source-kind freshness exclusions remain valid.
- Maintenance is a real missing consumer: `vibe-cli/src/commands/update/inputs.rs` creates install flags with empty features/default false and its lock row helper carries old effective features/subskills verbatim. That cannot reconstruct original omitted/exact/additive/dormant intent. Update/reinstall/modify need the same common request reader and reevaluation, not merely copying effective sets.

## Minimal change sequence (recommendation, not acceptance)

1. Freeze package declaration/legacy lowering and weak-forwarding meaning. Add authoritative optional-dependency/effect references and consumer choice authoring within the existing manifest grammar; lower legacy feature/exclusive semantics into the common kernel without changing cycles/default/private-feature behavior. Pair each advertised effect with its actual domain target.
2. Retain a small observed-candidate definition binding in existing registry/provider reads and fetch verification. Characterize local, multi, embedded, redirect/override/source paths; unsupported modes remain typed. Do not import the prototype's unrelated host namespace stack.
3. Add a configured request/result path at the existing InstallSource/package-source/resolver boundary. Reuse common evaluation for each candidate and preserve all original consumer edges. Make candidate activation/forwarding conditional on actual selected parent edges using the incumbent solver. Carry selected evaluations to fetch instead of recalculating defaults.
4. Connect selected effects to install graph, subskill loading/reader consumers and the explicitly supported config target. Wire consumer authoring and lock result through existing apply/record. Bind freshness, update and reconfiguration before calling this slice a delivered user feature.
5. Expose the ordinary console and serialized describe/evaluate/plan/apply journey over those library owners. A local registry fixture and a normal CLI path are required; fixture-only evaluator or manually bootstrapped host is insufficient.

These are semantic sequencing boundaries, not an argument to defer required work or declare an intermediate registry-only route universal.

## High-value integrated tests

1. Real local-registry install: newer candidate's selected feature requires an impossible dependency; an older compatible candidate succeeds. Assert selected graph, fetched payload, recorded features and boot output contain only older candidate effects.
2. Same feature/default name, changed default meaning between versions: explicit stable answer may backtrack; reviewed default binding must return rediscovery rather than silently authorize older/newer semantics. Also change definition bytes at the same version between describe, solve and fetch; assert no authored/slot/lock effects.
3. Two workspace members request the same dependency with compatible version constraints but incompatible exact choices: named conflict and unchanged installed world. Additive/exact compatible case yields one installed package and retained original per-consumer requests. Repeat through held-pin fallback, visibility retry and conditional new-root retry using F02 fixtures.
4. Strong forwarding diamond: A and B select features on shared C; C's induced dependency is solved once. Reject A's newer candidate and prove its forwarded feature retracts while B's remains.
5. Weak forwarding: absent target stays absent; independently required target gets the feature; removing/rejecting the activating edge removes that effect. No weak request may force target installation.
6. Optional dependency target is validated even in a dormant branch; wrong alias/group, missing declaration, invalid target feature and unsupported source mode refuse clearly. Successful package application requirements still appear beside configured package edges.
7. Feature-selected subskill appears through the real boot/read surface for its delivery mode; inactive content remains byte-present in the full slot but absent from effective activation. Reconfigure away/back, preserve original package bytes and authored workspace spec files.
8. One supported typed config effect reaches its actual existing domain plan/runner; conflicting effect values publish no partial configuration. Parameter secrets are absent from description/progress/persisted result, independent of child-code behavior.
9. Ordinary attended answer and equivalent serialized answer produce the same plan and stored intent without a socket/namespace bootstrap. Second bare install is Fresh only when choice/definition semantics agree; changed choice forces a correct new graph. Update preserves dormant choices and asks about removed/new mandatory members.
10. Local registry candidate capture, immutable commit route, metadata-only unbound case, mirror-equivalent bytes and same-version source drift each exercise the actual source adapter, not only caller-authored digest strings. Report unavailable modes rather than counting a rejected placeholder as support.

The main/prototype source findings above are observations. The sequence, compact request/result extension and solver modeling options are design inferences for root review; no new API or completed behavior is claimed by this report.
