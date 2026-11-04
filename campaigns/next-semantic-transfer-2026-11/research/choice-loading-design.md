# Choice-selected loading through existing boot/read owners

Read-only design input, 2026-11-04. Main observed at `effb460e4002dc6b3d31e91f9bfa312663bc81b4`; prototype pinned `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. Named user/configuration/loading scope only; no full boot, new worker, build or product/spec/map/Git/state edit. Existing dirty campaign log and untracked doc slot remain untouched. Recommendations below are not implemented behavior or acceptance.

## Conclusion

The smallest faithful transfer of the prototype's implemented choice-loading semantics is **selected subskill exports consumed by the current effective-boot and read owners**, while the whole package slot remains byte-present. Feature-conditioned boot fragments could be a useful separate feature, but main has no feature predicate today and that shortcut does not implement subskill identity, descriptions, three delivery modes or selected-export readers.

No second loading pipeline is needed: preserve `ResolvedDep`/per-node dependency closure → `DependencyBoot`/`EffectiveBoot` → existing compiler/INDEX publication, and give those owners the chosen export set from the same package evaluation used by install. A selected export is configuration/delivery state, not filesystem permission. An inactive export is absent from automatic loading/active discovery; it does not thereby prohibit a separately authorized explicit raw source read.

## Observed main contract and actual data

PROP-009 `TREE-VIBEDEPS` and `WRITES-RETIRED-WHY` require the package's whole shippable tree in its versioned dependency slot. One slot serves the shared workspace package version. Authored workspace files remain a separate tree. Package visibility controls membership in a node's effective dependency set; the linking axis controls how an effective member loads. `MATRYOSHKA-SCOPES` and `CATEGORY-ORDER` retain per-node reachability and dependency-before-dependent ordering.

`vibe-core/src/manifest/subskill.rs` already provides:

```text
SubskillManifest { subskill, activation, recommends, conflicts, content }
SubskillMeta { path, summary, description, delivery, describes }
DeliveryMode::{Eager, LazyPush, LazyPull}
SubskillContent { files_written: Vec<PathBuf> }
SubskillManifest::{read, validation_findings}, FILENAME = "vibe-subskill.toml"
```

The content paths are relative to the subskill directory. `path` is its declared logical path under `subskills/`. The old field name `files_written` does not require writing those files into the authored project tree. Several DeliveryMode comments still describe the retired pre-PROP-009 copying behavior; that wording is not current install behavior. `validation_findings()` checks absent descriptions and simple path shape, but does not itself enforce all path containment, declared-location equality, whitespace-only descriptions or complete content membership needed by a new reader.

Current install `record::locked_package_from_fetched` sets legacy `subskills_active=[]`. Existing CLI/MCP subskill readers consume those legacy lock rows, whose `files_written/cache_files` routing still reflects old project/store delivery. They are not wired to `FeatureExpansion.active_subskills`. This is corroborated by current CLI tests explicitly retiring the old per-file activation tests. Preserve compatibility readers where needed; do not write new selected state into a legacy shape with false location semantics.

### Main boot seams

- `install/model.rs::ResolvedDep` carries kind/group/name/version, fetched content_dir, optional source_hash, parsed Manifest, dependency edges, source provenance and materialization flags. It carries no chosen feature/export set today.
- `install/bootgen/conditions.rs::active_snippet(workspace_root, slot, Option<&BootSnippet>, &installed_identities) -> ActiveSnippet` yields one optional main contribution and a fragment vector.
- `BootSnippetFragment { source: PathBuf, when: Option<WhenCondition> }` inherits the parent snippet's link/category/format. `WhenCondition` supports only `Os` and `Installed`; there is no feature/choice condition.
- `resolve_condition` resolves installed-package conditions at generation time; OS conditions remain for INDEX. A configured feature selection is already known at generation, so it should not be relabeled as a session-time OS/dynamic predicate.
- `bootgen::node_dependency_boot(...) -> Vec<DependencyBoot>` computes a node's reachable package closure from the shared resolution. `hybrid_emit::build_unit_table(...)` independently consumes active snippets/fragments for hybrid compilation. A fragment-only change in one of these paths can disagree with the other.
- `boot::compute_effective_boot(NodeBootInputs) -> Result<EffectiveBoot,...>` is the pure ordered composition owner. `BootEntry` carries path, band, link, when, typed provenance, package format, substitution/elision state. The existing artifact compiler/emitter owns STATIC/INDEX bytes.
- `bootgen/materialised_read.rs::read_durable_resolution(workspace_root)` reads one lock, then its named slots with coordinate/materialization/embedded-source checks. It does not enumerate orphan slots. This is the right owner for a durable selection projection from that same lock epoch.

INDEX has an important naming distinction: a dynamically linked entry with **no residual `when`** serializes as `kind="static"` (direct read of a resolved file); a residual condition serializes as `kind="dynamic"`. Neither spelling alone identifies the eager versus lazy-push subskill mode. The mode needs its own typed loading intent/provenance, not an inferred interpretation of that INDEX kind.

## Observed prototype path: where selection actually comes from

`vibe-install/src/record.rs::configuration_record(request, provenance, fetched)` validates the selected child declarations, checks selected normalized effects against the admitted request/definition, and emits a `LockedConfiguration`. Its resolved package rows contain `package="group/name@version"`, `content=CachedPackage.content_hash`, and serialized effects including `subskills=FeatureExpansion.active_subskills`. Original consumer intent and selected results remain distinct within that record.

`effective_subskills.rs::selected_exports(record, package, content) -> Result<BTreeSet<String>,String>` decodes that record, finds exactly one package row, checks the content identity, and returns its subskill names. It does **not** derive selection by scanning directories, from legacy lock subskills_active, or from a bare active-features list.

Normal Ready apply constructs a preflight configuration record and passes it explicitly into boot work **before** the new lock is written. Durable regeneration obtains published selection from the lock. The important invariant is correct; the prototype's `durable_configuration(root)` is a separate lock read, so main should obtain resolution and selection together from its incumbent single-read durable owner rather than reproduce two independent epochs.

The prototype's shared selection/read functions are:

```text
capture_selected_subskills(root, selected) -> Result<Vec<CapturedSubskill>,String>
select_captured_subskills(files: &[CapturedSourceFile], selected) -> Result<Vec<CapturedSubskill>,String>
from_locked_package(root, lock, package) -> Result<Vec<CapturedSubskill>,String>
append_loading(root, resolution, dependencies, record, effective) -> Result<(),WorkspaceError>
bind_selected_slots(witnesses, record) -> Result<(),WorkspaceError>
```

`CapturedSubskill` contains the parsed declaration, its package-relative definition path and digest, and a package-relative content map with captured bytes. Reads use SafeFS. It checks declaration path equals `subskills/<declared path>/vibe-subskill.toml`, selected content is present, paths do not escape, lazy descriptions are nonblank, and duplicate content members refuse. Current main can reuse these invariants without importing `source_query::CapturedSourceFile`, common machine envelopes or an independent capture runtime; it already has SafeFS and held source-read machinery.

The helper named “validate complete child catalogue” only sees the supplied captured inventory. `capture_selected_subskills` supplies selected declarations/files, not a full tree census. Do not promote it into proof that all dormant exports were validated; definition validation must separately bind every declared effect/export reference required by the common configuration contract.

## Actual prototype delivery behavior

| Mode | Prototype boot behavior | Reader behavior | Whole-slot behavior |
|---|---|---|---|
| Eager | Appends selected content as `BootEntry`, Dependency band, Static link, Simple format, no residual condition | Selected content available through read_subskill | Full slot remains unchanged; only compiled boot contains selected content |
| LazyPush | Appends `vibe-subskill.toml` as a Dynamic-link entry with `SubskillOffer` provenance; INDEX includes description and read_subskill hint comments | QueryPackage exposes description/content paths; read_subskill returns body on explicit request | Full body remains in slot, absent from STATIC; no separate agent skill projection |
| LazyPull | Adds no boot entry | Selected export remains queryable/readable explicitly | Full bytes remain in slot, no automatic loading |
| Unselected | No generated selected contribution/offer | Active-export query omits it; prototype selected read_subskill refuses it | Bytes remain in slot |

Lazy-push here is an **offer plus explicit retrieval**, not an implemented autonomous task-matching engine that injects body bytes into every agent. Main must advertise exactly that behavior unless a real matching consumer is also built. The definition TOML path is an unconditional INDEX read; only its body payload is withheld. Both lazy modes need usable descriptions.

Prototype tests establish meaningful source-level scenarios: `record/m_05_d_1_cases.rs::m_05_d_2_install_loading_keeps_complete_slots_and_provenance` compares complete source/slot trees, checks eager-only STATIC text, lazy-push definition/description hint, no lazy-pull body in INDEX, durable regenerate/analyze parity and no project guide.md copy. `vibe-mcp/src/tools/m_05_d_2_cases.rs::captured_delivery_is_visible_readable_and_inactive_is_hidden` exercises selected query/read and writes=[] while independent Agent Skills remain unchanged. These tests were read, not executed here.

## Comparison with feature-conditioned existing fragments

A selected feature could gate an existing snippet fragment at generation time if a new normalized predicate or direct evaluated-selection input were added. That reuses `conditions::active_snippet`, `DependencyBoot.fragments`, hybrid units and the existing emitter. It is useful for “feature f includes this already-declared boot fragment.” It must feed both node composition and hybrid-unit construction and participate in fingerprints; no emitted artifact should retain a decision made from an earlier choice.

But it is not equivalent to exported subskills:

- A fragment carries one source path and optional when; it has no stable subskill export identity, multi-file declaration, description, PURL or delivery mode.
- Fragments inherit the parent's link policy. Prototype Eager explicitly selects Static, LazyPush offers declaration only, and LazyPull emits nothing. A single feature predicate cannot encode these distinctions.
- A fragment does not create the actual selected-export query/read path or durable export selection. Materializing all bytes and hiding one fragment does not complete that user journey.
- Adding a general `feature:` WhenCondition would be new authored syntax absent from the pinned prototype, not a mechanical transfer of its implemented semantics.

Recommendation: implement selected export loading as the first real vertical effect, using existing SubskillManifest/DeliveryMode and typed BootEntry composition. Add feature-gated fragments later only if a separate concrete package scenario needs them. Do not make both independent selection languages prerequisites.

## Minimal faithful vertical path (recommended)

1. **Definition/effect binding.** Resolve a normalized `subskill:<path>` effect against the owning package's declared export identity and actual child declaration. The existing parent feature activation string can be a legacy adapter; rich definitions may expose an explicit export set. Validate references, path/definition correspondence and supported content interpretation. Do not infer activation from mere existence of a child directory or from `[activation]` host probes.
2. **One selected value.** Carry the successfully evaluated export set with package coordinate/version and the same source content/definition identity used by plan. A small typed companion passed with ResolvedDep/boot inputs is enough; it need not alter every public ResolvedDep literal or embed the prototype's full JSON-in-JSON lock record. The common install result owner still has to persist a versioned generated selection alongside original consumer authoring for durable regeneration. A temporary map alone cannot finish delivery.
3. **Pre-lock versus durable input.** Ready apply passes its reviewed in-memory selection through normal boot preparation. Fresh/check/analyze/regenerate reads the published selection with the lock in the existing durable-world reader. Do not use the old lock while applying the new choice and do not independently reread it to collect selection.
4. **Existing composition/emission.** Lower selected export content/offers to the existing EffectiveBoot path for reachable packages of each node. Preserve dependency topology, band ordering, user-override placement, hybrid substitution/hoisting rules and compiler input provenance. Eager/subskill identities must reach fingerprints and any retained native/replay inputs before a reused plan is considered current. No separate files are copied into authored specs.
5. **Same read owner.** Query/list selected exports and read their declared payload from the selected slot through shared validated package/source observations. New selected delivery should read the full installed slot, not infer an old project/store location from mode. Preserve the current safe FileResolver source lookup for spec documents; for arbitrary subskill-relative resources reuse SafeFS's admitted root/held read rather than resolve-to-path followed by unchecked fs::read. Do not force arbitrary resource paths through spec-URI semantics or import the entire prototype source-query framework.
6. **Public vertical proof.** An ordinary local package choice through the real install owner changes the effective boot/active-export read result, survives lock reload, and can be changed back without changing original payload bytes. Console and serialized-answer routes ultimately feed the same result. The existing installation rollback and source revalidation owners remain responsible for their existing effect boundaries.

### Improvements required rather than literal prototype copying

The prototype appends all selected subskill entries after primary boot composition, then sorts by band only. Main's dependency-before-dependent contract needs proof for subskills too: appending A's/B's selected content after all ordinary dependency content can violate the intended per-package topological contribution order. Insert/compose them in the existing ordered dependency contribution owner or explicitly establish the resulting order; do not call a band-only sort complete ordering.

The prototype stores content in a BTreeMap, so emitted multi-file order is lexicographic. Main SubskillContent declares a Vec. Choose and test the content ordering law deliberately, preferably retaining authored order where meaningful, rather than inheriting container iteration accidentally.

`PackageFormat::Simple` is not a universal claim of raw-byte preservation in compiled output. The compiler performs generated framing/qualification and its established input behavior. Unsupported binary/invalid text or source interpretation must refuse the **automatic loading projection** truthfully; original bytes stay accessible through a raw resource path. Raw resource bytes are not normalized merely because the export is selected.

The prototype sets eager Static and offer Dynamic without consulting a package's ordinary declared link precedence. Whether subskill delivery is an explicit separate mode overriding that link or a consumer-overridable suggestion must be frozen as part of the target contract. The observed implementation is the former; main's generic fragments implement the latter parent policy. Tests must name this distinction instead of quietly treating them as equivalent.

## Selection is not a new authorization layer

Active-export discovery can list only selected exports; an API explicitly defined as “read an active subskill” can return not-active for an inactive selection. That does not make inactive source bytes secret or inaccessible. If the caller already has access to the installed package/source and asks explicitly for raw `subskills/inactive/guide.md`, preserve that read capability and path safety. Reading does not turn the export on, add it to boot, change lock intent, or execute its contents.

Likewise, not selecting a package due to dependency visibility is distinct from not selecting one of an installed package's optional exports. The selection helper must not widen package visibility or source roots, and must not narrow legitimate raw-source permissions merely to make active-view tests green.

No `MaterialiseSubskill` implementation should restart old project-tree copying for newly configured exports. The bytes already exist in the whole slot. A compatibility surface may truthfully report no new files written for that route or direct callers to explicit reading; old legacy records remain governed by their own supported reader contract.

## High-value tests for this vertical

1. Local package with one chosen eager export and another inactive export: actual install produces selected eager text once in STATIC; inactive body absent from STATIC/INDEX; both definitions/bodies remain byte-identical in the full slot; no authored project content copy.
2. Three chosen exports, one per mode: eager body in boot; lazy-push only description/definition offer and correct retrieval hint; lazy-pull no boot row. Explicit read returns exact declared body for all three, no process/network/choice mutation.
3. Inactive export: absent from active discovery/automatic loading; an existing safe explicit raw read still returns its bytes and leaves selection/boot unchanged. Do not replace this with a blanket forbidden-read assertion.
4. Install before lock update uses new selected set, not old lock; durable regenerate and analyze after reload yield the same logical entries and compiled content. Switch away/back preserves dormant authored intent and original content.
5. Multi-member workspace: package unreachable from leaf C contributes no selected export there; root/A/B scopes retain their real dependency closure. Shared package choice/consumer merge follows the accepted configuration result, not a caller-invented independent instance.
6. Package B depends on A, both have ordinary snippets and selected exports: ordering respects dependency-before-dependent and user overrides remain last. Exercise static, dynamic, hybrid, repeated shared dependency and transformed spec-format paths without stale fingerprints or duplicate selected text.
7. Missing selected declaration, changed content binding, path/declared-name mismatch, duplicate members, traversal and symlink escape refuse before publication; existing lock/boot state follows its established rollback contract. Dormant reference validation is not falsely inferred from reading selected files only.
8. Eager invalid/unsupported body produces a precise loading error while explicit raw retrieval remains possible. Content order for multi-file exports is stable and contract-defined.
9. Agent Skills declared separately stay projected exactly as before; subskill activation does not manufacture an Agent Skill, grant process authority or invoke a script. New MaterialiseSubskill compatibility path reports no authored-tree writes.
10. Reconfigure only the selected consumer's choice; sibling constraints/answers still bind the shared package. Changed selected definition/default requires re-evaluation, never silently reads old computed export names against new package bytes.

Observations are grounded in the named source paths; the proposed compact selection carriage and integration order are recommendations. The actual new declaration/persistence format and link-vs-delivery precedence remain root's architectural decisions. No complete user feature is claimed from this read-only report.
