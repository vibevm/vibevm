# Common authored configuration through the ordinary manifest

Read-only design input, 2026-11-04. Main source inspected at `5af2eadc8`; its parent session was concurrently integrating the separate secret-diagnostics candidate. Those dirty files were read only where relevant and not modified. Prototype profile/parser observations below use Git objects at `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. No builds, product/spec/state/Git edits or new agents. Names and grammar below are a proposal for root's decision, not currently admitted syntax.

## Observed boundary

Main has a useful single admission route: `Manifest::read` and `Manifest::parse_str` deserialize `ManifestWire`, then call `Manifest::validate`. `ManifestWire` actually lives in `manifest/package/visibility.rs`, with explicit conversion in both directions and `deny_unknown_fields`. Registry local, Git lookup/fetch, override and offline paths use those ordinary entrypoints. The current `FeaturesTable` is a manual decoder for arbitrary feature-name keys plus the reserved `exclusive` table; inserting metadata keys there would collide with legal feature IDs.

Prototype `profile/constrained.rs` instead detects `features.profile = "constrained-r1"`, deserializes a specialized generic ManifestWire and constructs a CheckedFeatureManifest beside a legacy projection. `Manifest::parse_package_source` returns only its projected Manifest; CandidateMetadata separately reparses the same source to retain checked_features. The constrained profile admits typed local/dependency/forwarding effects and four-cardinality group guards, but no common presentation labels, parameters, named user profiles or member guards. Its companion finite migration grammar covers only a subset of ordinary package fields/kinds. Copying this split into main would preserve the gap between ordinary readers and configuration-aware readers.

## Recommended authored locations

Add one optional `configuration: ConfigurationDecl` field to ordinary `Manifest`/ManifestWire, written as `[configuration]`, legal only with `[package]`. Add the **same declaration type** as optional `configuration` in `InstallerOptionDecl`, written under the selected `[[installer_options]]` row. There is no profile marker, format epoch or second TOML decoder choosing another universe of package fields.

These locations identify two real scopes:

- Top-level configuration is the source package's configuration. In a workspace it is evaluated for the shared selected package instance from the actual dependency graph and its real consumer declarations. It controls package feature/effect data and the supported loading/dependency consumers.
- Installer-option configuration belongs to an application installation owner and its actually selected installer option. It controls that option's step features/parameters. Two installer IDs do not create two dependency instances; they are alternative definitions selected by the existing app planner. An unselected option's answers cannot leak into the selected option.

An app may carry both scopes because acquiring its package in a workspace and installing the application into a home are distinct existing operations. Neither is implicitly merged into the other. IDs such as `gui` are local to the selected definition. The external API should use a closed selector `Package` or `Installer { id }`, resolving that ID against the actual package/option; it must not accept client-created domain/target/instance/owner strings. Namespace clarity is necessary data, not a new ownership protocol.

## One declaration type, explicit legacy lowering

Proposed `ConfigurationDecl` consists of ordered vectors `features`, `groups`, `parameters`, `profiles` and optional declared typed config values. Every entity has a stable local ID and suitable public name/description; order is presentation order, IDs/sets compare literally. Shared IDs use the accepted nonblank/no-control rule; the old app declaration's portable-token rule remains its legacy adapter restriction.

`ConfigurationFeatureDecl`: `id`, `name`, optional `description`, `parent`, `required`, `default`, `requires`, member `guards`, and a closed typed `effects` list. Ordinary local implications lower directly into FeatureNode. Effects use explicit records for declared dependency activation/forwarding, selected subskill export and config-value data, rather than making package coordinates compete with the local-feature string grammar. A dependency operand names the actual authored requires key (qualified coordinate/kind spelling normalized by that owner); an export operand binds a real SubskillManifest. Unbound foreign references are not admitted merely because the pure FeatureModel accepts an ID.

`ConfigurationGroupDecl` is a closed tagged sum with `mode = "choice"` or `"closure"`. Both have ID/name/description/members/cardinality. Choice groups have `defaults` and positive `guards`; member guards are declared on features owned by a choice group. Closure groups instead have `fallback`, with activity derived from parents as in the accepted kernel. They do not acquire a choice-policy receipt. Cardinality spelling is `zero-or-more`, `one-or-more`, `zero-or-one`, `exactly-one`. Do not overload one defaults field to mean consented consumer defaults and unconditional closure fallback.

Parameters reuse the existing authored `ParameterDecl` vocabulary (string/bool/integer/choice/path, typed default, required, min/max, choices, must_exist, secret), moving validation/canonicalization behind the common owner rather than creating another parser. Profiles have ID/name/description and supplied ordinary features, group answers and public parameter presets. Profile contents and explicit request overrides are normalized within one consumer before merging consumers: an explicit group answer replaces that consumer's profile group answer, including explicit empty; two independent consumers still obey exact/additive merge. Retain the chosen profile ID and overrides, plus the reviewed definition basis needed to explain profile/default drift. A profile never authorizes effects directly.

Avoid two simultaneous graph definitions. For a given scope, choose the common declaration **or** the existing legacy declaration surface:

- If top-level configuration is absent, lower the complete current FeaturesTable exactly as accepted by the package adapter. If common configuration is present, nonempty legacy `[features]` in that same package scope is a field-addressed conflict. All unrelated package fields remain admitted and roundtrip normally.
- If an option's configuration is absent, lower its existing features/profiles/parameters. If present, nonempty legacy option features/profiles/parameters in the same option are a field-addressed conflict. Steps, platform/arch, result, launchers, requirements, license and other existing option fields remain valid. Step feature/parameter references validate against the selected normalized definition. Rich groups must not accidentally become legacy exactly-one groups.
- This preserves every existing valid manifest without rewriting it, while giving new authors one place for configuration. If incremental mixed declarations are later required, define an explicit presentation-only reference mechanism; do not silently union two independent graphs or choose one by priority now.

## Two proposed TOML examples

The first is a package definition using normal package fields and the common model. The `verbose` effect is inspectable/persisted data, not an arbitrary config-file write. All three named features are declared, including the guarded one.

```toml
[package]
group = "org.example"
name = "workflow"
version = "1.0.0"
kind = "flow"

[configuration.config_values]
verbose = true

[[configuration.features]]
id = "mup"
name = "Multi-user planning"
[[configuration.features]]
id = "wal"
name = "Write-ahead log"
[[configuration.features]]
id = "audit"
name = "Detailed audit"
guards = [{ member = "wal" }]
effects = [{ config_value = "verbose" }]

[[configuration.groups]]
id = "planning"
name = "Planning method"
mode = "choice"
cardinality = "exactly-one"
members = ["mup", "wal"]
defaults = ["mup"]
[[configuration.groups]]
id = "addons"
name = "Optional additions"
mode = "choice"
cardinality = "zero-or-more"
members = ["audit"]

[[configuration.parameters]]
id = "history-limit"
name = "History limit"
type = "integer"
default = 20
min = 1
max = 100

[[configuration.profiles]]
id = "audited"
name = "Audited workflow"
selections.planning = { intent = "exact", members = ["wal"] }
selections.addons = { intent = "additive", members = ["audit"] }
parameters.history-limit = 40
```

The second is original intent on a real ordinary dependency edge. It needs no caller-supplied consumer ID: the workspace node plus this declaration supplies provenance. Switching planning to `mup` leaves the audit answer dormant; the authored row is retained. `members = []` would be explicit empty, while omitting `members` inside a selection means Omitted with its stated strength.

```toml
[project]
name = "demo"
version = "0.1.0"

[requires.packages."flow:org.example/workflow"]
version = "^1.0"
link = "dynamic"

[requires.packages."flow:org.example/workflow".configuration]
policy = "require-selection"
parameters.history-limit = 30

[requires.packages."flow:org.example/workflow".configuration.selections]
planning = { intent = "exact", members = ["mup"] }
addons = { intent = "additive", members = ["audit"] }
```

For an installer, precisely the same definition subtree is `installer_options.configuration` beneath its chosen array row, e.g. `[[installer_options.configuration.parameters]]`; its answers arrive through the app operation and belong to that installation's record, not automatically to `[requires.packages].configuration`. Installing an app from a project does not erase this distinction. Cross-scope bindings require an explicit future contract; matching IDs are insufficient.

## Actual-edge original intent and persistence

Add optional `configuration` to `InlinePackageDepWire`, retaining the bare version-string form unchanged. Registry/path/git/version.var entries use the same field and preserve it through RequiresWire conversions and workspace variable finalization. Internally, a `Requires::configurations` map keyed by the existing normalized declaration identity is a bounded extension analogous to links/accesses; it must be carried for every source bucket, not only `packages`. Duplicate spellings that normalize to the same edge should diagnose before one overwrites another. RootRequirements retains the supplying workspace-node identity, version/source requirement and original answer together; a coordinate-only merged map is not sufficient for multi-consumer provenance.

Proposed `ConfigurationIntentDecl`: optional ordinary `features`, explicit default-features policy, optional profile reference, group default `policy`, group `selections`, public typed `parameters`, and retained reviewed-default/profile receipts when actually produced by an interactive owner. Selection entry has independent `intent = additive|exact` and optional members. Absent entry normalizes to Omitted+Additive. Missing configuration uses the legacy route; presence of a rich configuration requires an explicit policy rather than silently interpreting assume-yes as consent. `use-declared-defaults` requests those defaults, but the existing plan/discovery owner must bind any accepted use to the shown semantic definition; a policy string is not a fabricated receipt.

Dormant answers stay in this authored request. Effective selected features, config values and exports belong in the generated result. Receipt data records the reviewed semantics, not user-supplied authority; emit/update it through the same selected-manifest edit transaction, and revalidate against the actual target definition. Unknown/removed dormant answers return reconfiguration diagnostics, never normalization by deletion. Secret parameter values are ephemeral operation inputs and are not auto-written into this map, a profile, lock or receipt. Whether a parameter is secret comes from the selected target definition, so enforce that again before persistence; syntactic TOML admission alone cannot know a remote parameter's classification.

Multiple consumers' parameter values need an explicit shared-instance rule too: equal canonical values agree; differing supplied values conflict with contributor names. Do not reuse last-CLI-value-wins across consumers. Defaults/profile origins remain distinguishable. A request for a path is resolved by the actual planning host; it is not a portable shared filesystem fact.

Optional dependency activation belongs on the same real dependency declaration (`optional` in its inline entry), not a separate package-wide list keyed by guessed names. Strong/weak forwarding refers back to that entry and retains its version/source/visibility. This is a proposed companion field needed for complete effects, not syntax already supported by the candidate. The already chosen weak rule is active-declaration support, not incidental target presence elsewhere in the graph.

## Exact owners and normalized lowering

| Existing owner | Proposed bounded change |
|---|---|
| `vibe-core/src/manifest/document.rs::Manifest::{read,parse_str,write}` | Preserve these entrypoints; add `configuration: Option<ConfigurationDecl>` to the ordinary document, never a `parse_package_source` alternate reader. |
| `manifest/package/visibility.rs::ManifestWire` and both conversions | Carry the new field losslessly, retain all existing sections; unknown fields remain errors. |
| New `manifest/configuration.rs` | Authored serde structures and field-addressed validation; closed guards/effect forms, typed parameters/profiles. No I/O/effect execution. |
| `manifest/document/validation.rs`, option `validate_wizard`/step validators | Validate both whole definitions and explicit legacy/common exclusivity; use normalized feature/parameter names for app step references. Add data-only doc/remap restrictions explicitly. |
| `configuration::{FeatureModel,ChoiceModel}` | Continue pure local evaluation. A proposed `ConfigurationDecl::lower` produces their inputs and presentation metadata; it does not execute foreign effects. Parameter model joins alongside these, not inside feature IDs. |
| `vibe-resolver/src/features/compiled.rs` | Existing legacy lowering remains the old-input adapter. A rich projection consumes the same resolved features and typed effect declarations; no typed→string→typed roundtrip and no duplicate closure. |
| `manifest/package/{capabilities.rs,wire.rs,wire/requires.rs,deps.rs}` | Admit/serialize source-independent edge intent and optionality, preserving them through all source conversions and version.var resolution. |
| `vibe-install::plan::RootRequirements`, source/solver seams | Bind each actual consumer edge to the candidate's normalized definition, preserve intent on every retry, and return selected evaluations to prepared install. |
| `vibe-install/src/record.rs::merge_manifest_requires`, apply/lock owner | Edit only the selected consumer's original intent; persist selected result and semantic binding through the existing transaction. Freshness/update must read original intent, not infer it from LockedPackage.features. |
| `vibe-app::steps::plan/params`, operations install/update/modify, index | Lower selected option definition, retain app scope/origin, drive existing prompts/runner. Rich intent persisted in the actual installation owner; legacy effective records remain explicitly legacy. |
| `vibe-app::describe`, CLI app wizard/common configuration adapter, wire generators | Build one presentation/evaluation DTO from the same normalized declaration; old app views keep their contract, richer common views get a new versioned generated document. |

Normal registry reads must retain the rich field, so fetching/reparsing a source cannot silently fall back to only a legacy effect projection. Candidate metadata can attach its actual definition/source observation to this one Manifest; it need not own a second CheckedFeatureManifest decoder. Hash/canonical semantic encoding is a separate well-defined boundary including effects/defaults/guards, not arbitrary serialization of a display DTO.

## All nine kinds and operation boundaries

| Kind | Shared definition/evaluation | Existing consumer/allowed effect operation |
|---|---|---|
| flow | Top-level common or legacy lowering | Workspace package solver, full slot, effective boot/read contributions. |
| feat | Same | Workspace package capability/content owner; selected effects through real solve/loading. |
| stack | Same | Workspace aggregation and dependency selection; preserve ordinary constraints. |
| tool | Same | Project slot/bin owner; selection does not itself run a binary. |
| mcp | Same | Package/registered MCP owner; existing exact-pin and mcp-kind rules still apply. Selection does not launch a server implicitly. |
| lang | Same | Workspace guidance/loading owner; no fabricated compiler/runtime action. |
| app | Top-level package and separately selected installer scope | Package acquisition plus existing app install/update/modify/home state. Domain operation chooses scope; no automatic merge. |
| doc | Top-level data-only configuration, or empty definition | `cache add`/reader and request-local evaluated data. Still no install, boot, binary, hook or executable effect. No ordinary requires edge is invented to bypass doc admission. |
| remap | Top-level data-only configuration, or empty definition | Existing remap offer/acceptance owner; evaluation does not accept policy, choose a provider or activate rows automatically. No normal package dependency bucket. |

Remap currently rejects `[features]` and most unrelated tables in `document/validation/remap.rs`; admitting common data requires an intentional narrow allowance, not removing that deny-list. A nonempty configuration may expose read/inspect values on doc/remap without granting executable effects. Any actual configurable remap-row selection must bind the *resulting full offer* to its existing consent ledger before application; that is additional named domain work, not an implication of a radio selection. Until such a mapping exists, describe/evaluate is supported, applying a novel effect is a typed unsupported result. All-kind parity means the same semantics and honest operation support, not nine fictitious installers.

`[requires.remaps]` is currently a separate string-only bucket. Do not silently move it to packages or overload package configuration with policy consent. If durable remap data intent is needed, extend that bucket with its own backwards-reading inline variant and existing consent ownership. Doc reader preferences can stay request-local until its existing reader owner gains a named persistence surface. The ordinary requires-edge example applies to actual ordinary dependency instances only.

## Backward reading and closed schema boundaries

Authored compatibility is additive in the new reader: missing new fields lower exactly as today, old manifests keep their representation, and only explicitly authored common scopes select the new declaration. Do not automatically rewrite the whole document or discard comments, source fields, defaults or omitted distinctions. The old binary has deny_unknown_fields and **will reject** new configuration fields; it is not forward-compatible. Set the established `compatibility.min_vibe_version` for newly authored syntax and explain that older readers need upgrading. A manifest parser epoch does not solve that and is unnecessary for this addition.

Current `vibe.lock` is a closed schema with `CURRENT_SCHEMA_VERSION = 7`, and read rejects every other version. Durable rich requested/resolved/default evidence therefore needs an explicitly specified next lock schema, not unknown fields inserted into schema 7 or JSON hidden in an unrelated string. Recommended migration: a new reader can import 7 as LegacyEffective with no invented original choice/default provenance; normal replanning uses authored intent. Write the new schema only via the existing apply transaction. A legacy exact-replay promise cannot be upgraded by guessing intent. Accepting both versions requires an intentional change to the current strict-reader policy and tests, not just a constant bump.

Application index uses generated `application/e2/index` and runtime protocol `vibe-user-applications/3`; app describe/plan are also closed generated contracts. Keep existing describe/plan projections stable. Introduce a separately versioned common configuration document and generated codecs for rich views, while the installation index needs its own deliberate next protocol and compatibility reader for existing effective-only records. Do not use an untracked sidecar as a second source of original configuration truth. Run/work/resume formats must evolve if later continuity requires persisted input-presence metadata; this authored-shape proposal does not pretend current steps.json already stores it.

## Discriminating delivery tests needed

The two examples were parsed with Python's `tomllib` as a document sanity check; both are syntactically valid TOML. This does not mean the current VibeVM parser admits the proposed fields. No Rust build or product execution was performed.

Ordinary parse/read/serialize must retain rich data alongside hooks, skills, documentation, visibility, conditional dependencies and app options; every registry source path returns the same normalized definition. Old feature named `profile` remains an ordinary legacy feature; old defaults/private/cycle/space-ID cases stay valid. Mixed legacy/common graphs refuse with exact fields. Source alternatives and version.var retain edge intent. Explicit empty, omitted/exact/default strength and dormant answers survive comments-preserving selected-node writes and fresh-process reload. Package and installer scopes with the same local IDs do not bleed into one another. Two workspace consumers merge correctly with names on conflict. Old records are read as legacy effective evidence without manufactured consent; new machine/index/lock formats pass exact generated-code and wire gates. Doc/remap evaluate without crossing their existing effect/consent boundaries. These are proposed acceptance cases, not tests executed in this read-only task.
