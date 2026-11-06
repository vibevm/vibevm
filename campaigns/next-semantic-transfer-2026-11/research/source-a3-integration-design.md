# A3 installed-reader / materializer bridge design

Read-only design against accepted main `626fcbd8de385be7e4ffb1d5f80eb5d17457fab2`, prototype `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. No product edits, Cargo, network, Git mutation or new agents. Companion JSON binds 56 files and the full exact original A3 object. Prior source-a23-review and source-foundation-decisions remain baseline. Optional XML inspector is a separate frozen candidate from migration_surfaces (`/tmp/nst-a3-xml-progress.md`); it is not accepted main and does not close the joined A3 task.

## Recommendation: one parser owner, data flowing down the existing DAG

Keep slot-record parsing/validation in **vibe-workspace::vibedeps::slot_record**. Add no workspace dependency to vibe-spec and no second TOML/record validator. The workspace prepares selected output bindings once from held metadata and the existing `read_slot_record_bytes`; the lower spec reader receives rows plus **opaque captured record and manifest bytes**, not a new record model it must deserialize. Every actual read compares current held metadata bytes with those preparation bytes before and after reading/checking its output. Exact byte equality preserves the already validated record meaning without reparsing it below its owner.

DAG remains `workspace → spec → core / wire / safefs / specdoc`. No trait callback into workspace, extra crate, M16 service or global source-world object is necessary. This is a finite per-operation package lookup table and read guard. It is not a permission receipt, original-source authentication or universal transaction.

### Proposed lower interface (new, not already implemented)

Owner: `crates/vibe-spec/src/resolver/declared.rs`, registrations in resolver.rs/lib.rs, integration into observation.rs and embed/read.rs.

```rust
pub struct InstalledSourceExpectation {
    pub kind: PackageKind,
    pub group: Group,
    pub name: String,
    pub version: String, // canonical host version, as in existing SelectedPackage
    pub source_hash: ContentHash,
    pub materialized_format: SpecFormat,
}
// Private fields after the checked constructor. This is host-prepared data,
// not a claim that spec parsed/authenticated the slot record.
pub struct PreparedInstalledSource { /* relative slot, expectation,
    PackagePolicyCheckpoint, opaque manifest/record bytes, output rows */ }
pub struct PreparedOutputRow { /* original source path, output path,
    selected interpretation, actual materialized media, output SHA-256,
    optional native address */ }

impl PreparedInstalledSource {
    pub fn from_host_checked(
        slot_relative: String,
        expectation: InstalledSourceExpectation,
        policy: PackagePolicyCheckpoint,
        record_relative: String,
        record_bytes: Arc<[u8]>,
        rows: Vec<PreparedOutputRow>,
    ) -> Result<Self, ResolveError>;
}
impl FileResolver {
    pub fn with_installed_source_bindings(
        self,
        workspace: Arc<Project>,
        bindings: Vec<PreparedInstalledSource>,
        installed_self: Option<(String, String)>,
    ) -> Result<Self, ResolveError>;
}
```

The host supplies the existing workspace `Project` capability; the new table retains one root capability, not one permanent FD per file/ancestor. Each lookup/read descends the relative slot through `Project::dir`/`Pinned`, temporarily holding the selected slot and file parent. Match the supplied workspace capability to this resolver's already-selected workspace root at construction; use incumbent root/policy observation checks rather than designing another root-identity algorithm. Retain/recheck original logical-root admission when its caller supported an explicit workspace alias. The slot is always a descendant, never a new caller-root grant.

`record_relative` comes from the workspace's sole `SLOT_RECORD_FILENAME` constant. Manifest bytes and their selected-root identity are already retained by PackagePolicyCheckpoint. The lower constructor checks portable paths, coordinate consistency, unique physical outputs and native addresses, native media compatibility and source/address relation; it **does not** duplicate record schema/shape/disposition parsing. These lower checks are necessary against publicly constructed lookup data and do not prove the host actually prepared it from a real package. No public type should be named an authentication/ownership token.

Keep legacy SelectedPackage and existing constructors compatible. Mark installed self explicitly in the new bound constructor: self-coordinate reads must descend the supplied workspace slot, including when the self address has no version. Keep existing self-version refusal rather than silently changing address syntax. Factor current known-derived-root handling so recognizable legacy with_roots self slots also avoid `admit(self_root)`; genuinely caller-supplied unrelated self-root aliases retain their current contract. Do not guess that any arbitrary path is a slot. Missing required installed-self binding refuses, never falls back to canonicalizing the slot.

### Workspace preparation (sole semantic record owner)

Owner: new `vibedeps/declared_read.rs`, alongside current `source_policy.rs`, with a narrow reexport in vibedeps.rs:

```rust
pub(crate) fn prepare_declared_slot_read(
    project: &Project,
    slot: &Pinned,
    slot_relative: &str,
    selected: &ResolvedDep,
    expected_materialized_format: Option<SpecFormat>,
) -> Result<Option<PreparedInstalledSource>, WorkspaceError>;
```

The workspace helper builds the mandatory lower InstalledSourceExpectation only after selecting declared policy: missing selected.source_hash or expected_materialized_format then refuses. A genuinely legacy package may retain None without inventing either witness. No new semver dependency is needed below workspace; use the incumbent SelectedPackage canonical version string.

1. Observe policy through `observe_package_policy_in`; use `PackageMeta::has_explicit_source_policy`, including export-only/empty. Never raw-TOML presence probing. Check independently selected kind/group/name/version.
2. Read `.vibe-slot.toml` through the same Project/Pinned, call `read_slot_record_bytes`, compare package source_hash and materialized input format with **expected**, and require recognized converter recipe where transformed. The current record validator only requires a nonempty recipe, not equality with the current supported converter.
3. Bind captured manifest bytes to the record's manifest row. For Mixed, original source is the output path and source/disposition must be absent by the existing validator. For transformed, use retained `source` and `disposition`, not filename guessing.
4. Select each original logical path with the existing core selector. Raw/CommonMark/XML-data/opaque/unclassified rows receive no native address. Export names never become spec:// addresses. Native media after conversion comes from accepted disposition/target conversion; copied rows keep declared media. Check actual output/source relationship against the admitted preparation rule, and refuse ambiguous native address claims.
5. Capture/check relevant output bytes with held source reads (hardlinks allowed), compare their recorded SHA-256, and recheck metadata/root. Retain only metadata/row witnesses needed for later reads, not a permanent FD for every output or an invented entire-source snapshot. Existing exact-file preparation remains the bounded source inventory; do not silently claim a directory census from an output list.
6. A proved unopted-in package can return None for legacy routing. Once a bound operation includes declared packages, retain the complete independently selected coordinate/version set so other dependencies do not revert to freshest-installed search. Unknown/missing declared evidence is an error, not None.

One genuinely missing narrow helper should be added to the incumbent policy owner, not reimplemented in the map: `PackagePolicyCheckpoint::recheck_in(&Project, &Pinned) -> Result<(), SourcePolicyError>`. It binds the supplied held slot's logical path/identity and captured manifest bytes to this checkpoint and reuses the existing logical-name/root drift checks. Current `recheck_unchanged()` only re-observes its own stored root; a lower reader must not accidentally check an unrelated old root while reading another slot. Use this helper at construction and before/after each read; it introduces no schema, new authority or second policy law.

No record wire change is required. The current record proves output digest plus a package-wide selected source hash. For converted rows it does **not** retain an independently witnessed original-file SHA. Do not manufacture that claim from output bytes or from a copied editable record.

### Actual read/overlay law

Extend SourceLocation with a declared read guard and materialized media. At locate and at actual read, obtain the selected descendant through the held root; reject policy/record changes. Actual read sequence is:

- recheck selected logical root/slot identity and captured manifest policy;
- held-read record bytes, require exact equality to the opaque captured record;
- choose the exact lexical overlay key if present, otherwise use incumbent held source read;
- validate the chosen buffer against the selected output digest and expected media/interpretation; bad overlay refuses without filesystem-content fallback;
- held-recheck record and manifest/root before returning that same buffer.

Use the existing SafeFS source reader, not state snapshots with a single-link restriction or prototype fixed source caps. Ordinary hardlink sources remain valid. This is a finite observation window; detached guard metadata does not exclude hostile inode reuse/ABA and later writes can still produce ordinary partial failure.

`FsSectionSource::read_location` currently projects by path via `project_bytes`, even when the file was located safely. Carry media to a **pure projection seam** used by both that reader and Simple boot inputs: selected Markdown stays Markdown; selected Vibe XML uses the existing vocabulary-aware XML parser and Markdown emitter. Raw resources refuse a native read. Projection-relative positions remain distinct from original byte positions. Keep native anchor resolution and F05 metadata-versus-location checks after projection. Custom frontends must not receive explicitly classified inputs until an actual compatible classification contract exists.

Pattern membership must call the same declared-aware locate path as point lookup, preserving package/version/revision pins and propagation of ambiguity/unsafe errors. An empty match is not an excuse to swallow a changed record. Main glob.rs already uses locate_source for selected-world membership; retain it rather than copying the weaker prototype map bypass.

## Identity, output naming and recipe

Three different keys must remain different:

1. original logical resource: package coordinate plus original declared relative path;
2. physical materialized output path;
3. native address: incumbent `canonical_doc_path(original_source)` for selected native inputs only.

Do not require canonical_doc_path(output) to equal canonical_doc_path(source). That prototype invariant breaks explicit media under arbitrary suffixes and silently couples addressing to output spelling. The Normal seed must be taken from the prepared binding's native address. `normal_seed(origin, physical_path)` remains only the legacy route. Likewise, `[boot_snippet].source` maps original source → recorded output before lookup; the existing literal-first/alternate-suffix function remains legacy-only. A raw same-stem neighbor cannot steal the snippet.

**Existing output naming:** a converted native file emits `relative.with_extension("xml"|"md")`; Mixed and raw copied resources keep their original paths. Retain this rule. For declared native input, use declared media rather than suffix to choose the parser; conversion/normalization is needed when target media/suffix differs or a native overlay applies. Do not invent disambiguating filenames to avoid a collision: native `foo.md → foo.xml` colliding with raw `foo.xml` is a real refusal. Ordinary `foo.md` + XML-data `foo.xml` coexist unchanged and mint no shared native address. Two selected native forms claiming one canonical original address refuse even in Mixed.

Main's emitter recipe is already `specdoc/4`. Recommendation: retain it when the existing pivot/emitter bytes and legacy branch are unchanged; A1 did not permit declared transformed slots, so there is no legitimate older declared-transform output needing automatic invalidation solely for this new admission. Require that exact supported recipe when reconstructing declared transformed meaning. A recipe bump becomes necessary only if implementation changes previously admitted output semantics; prove that with exact-output controls rather than importing a historical bump. No CopiedUnconverted enum/wire machinery is an A3 prerequisite. Invalid declared native conversion should refuse before effects; invalid XML-data remains raw and optional inspection failure is unrelated.

VibeDoc's Markdown projection is one-way in the incumbent pivot. Do not reinterpret that projection as native Doc IR. If the new bound reader cannot truthfully support a requested vocabulary/target pair, refuse that pair explicitly before the relevant effect; completing VibeSpec + raw roundtrips does not prove every later Doc consumer migrated. Directory-source completeness remains separately qualified: do not remove A1's census refusal until actual materializer-owned enumeration/preparation supplies its required validation. A fabricated universal census is not needed for the exact-file A3 journey.

## Concrete carrier closure: successful caller, not unused overloads

| Current owner | Required join |
|---|---|
| bootgen.rs / hybrid_emit::emit_package_units and emit_effective | Receive selected resolution + explicit materialized input format, prepare bindings after materialization and before semantic compile/publication; pass the same bound resolver to actual unit emission. Test this real function for Simple and Normal. |
| build_unit_table / snippet_source / node_dependency | Replace declared snippet suffix fallback with original→output mapping; preserve legacy fallback only for unopted-in packages. Unit own boot path and fragments carry recorded physical outputs while original native addresses remain separate. |
| boot_artifacts.rs::compile_static_artifact_with_plans / inputs.rs | Add internal bound sibling/optional bound argument, used by actual installer. Normal receives mapped seed and FsSectionSource; Simple reads the same bound buffer and projects by declared media. Public legacy wrappers stay unchanged and specifically refuse declared operation without required binding. |
| bootgen/native_managed + hybrid_emit/native_managed | Both have resolution and selected operation format now. Pass bindings through compile_static_owner_managed_using to inputs and its actual FsSectionSource; no fallback FileResolver::new in the declared arm. |
| boot_artifacts/native_managed/explicit.rs | Explicit backend currently creates both inputs and FileResolver::new independently. Add bound sibling and thread both carriers; a backend selection is not permission to bypass source classification. |
| analyze.rs + boot_artifacts/analyzed.rs | Add an actual internal/public bound overload such as analyze_node_lane_with_source_format(..., expected_materialized_format: SpecFormat), and equivalent bound-native/backend overloads. Use read_durable_resolution for independent coordinate/hash; pass independently selected input format from the command/install caller. Never infer input format from STATIC filename. |
| replay_prepare::{BootReplayCandidates,SealedBootReplay} + native_managed/replay.rs | Retain selected expectations/input format and prepared read bindings in the existing in-memory sealed candidate. Reuse them on prepare_boot_replay; missing hash/format refuses declared replay. Do not reconstruct expectation from output slot record. Existing generated overlay stays a separate provenance route. |
| materialised_read::read_durable_resolution/durable_row | Narrow held-read correction: read lock and each named slot manifest from one admitted Project and no-follow descendant chain, then use the existing parsers/checks. Current slot.is_dir + Manifest::read can follow a substituted slot before the later bridge gets a chance to refuse. Preserve embedded-source/configuration checks and lock order. |

**Independent format is a real missing parameter.** Lockfile/LockedPackage and ResolvedDep do not store SpecFormat. Analyze currently chooses Xml from STATIC.xml, otherwise Mixed; a Markdown lane can represent either Mixed or Markdown materialized input. Root approved an explicit parameter from the actual install/command selection owner, not a new lock field and not slot-record circularity. Keep `materialized_format` separate from the boot output rendering target. The positive caller must be real `emit_package_units`/native install (already has SpecFormat); bound analysis/backend overloads must be called by their selected-format owner before claiming those routes work. Old unbound analysis can remain legacy-compatible and specifically refuse declared input with missing representation binding. No full A3 acceptance if only unused bound methods exist.

The concrete public command callers were traced: `vibe-cli/src/commands/extensions_analyze.rs::run` invokes the unbound analyzer today; `commands/extensions.rs` invokes `compile_node_backend` after selecting the workspace/node. Extend exactly those call edges to bound overloads. Reuse the existing `commands/reinstall.rs::effective_rederive_format` / `commands/install::resolve_spec_format` selection owner at the command boundary, with the manifest selected by the incumbent materialization policy, rather than adding configuration discovery inside workspace/spec. This supplies current selected intent, not a reconstructed historical install receipt; mismatching installed representation refuses. A successful bound command fixture is required, while the old library wrappers keep legacy behavior and precise missing-binding refusal. These two command-body reads were added only to identify actual positive callers, not to broaden A3 into transport redesign.

### Generated artifacts are a separate incumbent lane

G3 excludes generated STATIC/INDEX from authored slot record and source declarations. `unit_substituted`, generated dependency carriers, compiler artifact recipe/fingerprint and replay's prepared overlay already distinguish generated output. Preserve that role explicitly: do not synthesize authored SourceDescriptors or exported ordinary spec:// addresses for STATIC. Simple substituted input must use the incumbent generated-artifact path with its known rendering target and selected owner; normal/internal generated reads must use the same narrowly scoped artifact binding/overlay, not raw source-map suffix fallback. Carry observed generated bytes/digest from actual publication/reuse validation where a later lane consumes them. If exact generated evidence is absent, refuse rather than treating a disk filename as proof. This is a narrow extension of existing artifact ownership, not a new source-world service. Include static-child/substitution/replay controls so the first successful authored read does not hide a broken composed unit.

## Preparation before effects and publication

The A1 earlier-hook regression must remain fixed. Before metadata/hooks/earlier-slot effects, reuse materializer-owned preparation to decide all known declared input availability, interpretation, conversion support, actual output collisions and native address collisions across each package. Do not perform an early weaker check and rediscover a predictable conversion/collision failure in a later package after the earlier hook ran. The final per-slot operation still rechecks captured manifest/source bytes/mode/identity; post-preflight changes may cause ordinary partial failure. No whole-install atomicity claim.

Current slot_diff plans reject differing unrecorded occupants and retain byte-matching unrecorded occupants, including their modes. Publication still uses replacing persist and rename after that preflight. Add a minimal placement disposition to the existing DiffPlan: **observed absent/new path** uses no-replace publication; a concurrently appearing destination refuses without removal. Staged hardlink publication must obey the same no-replace rule, never blindly remove the unexpected destination. Existing recognized-record updates retain their cooperative replacement law and existing source/placed-byte/mode checks. Do not claim atomic compare-and-swap of every old inode. Preserve permission-only reconciliation of recorded observed files and unchanged foreign same-byte inode/mode; no blanket new rule that changes legacy foreign ownership. Use incumbent portable path_identity_key for provable aliases; unsupported normalization guarantees are explicit, not a non-ASCII ban.

## Required discriminating tests / implementation order

1. Keep the pure XML inspector separate; use explicit optional inspection limits without capping raw retrieval. No dependency on full M16.
2. Implement host-prepared rows + opaque record guards and lower read/media seam first. Tests: changed record/manifest/output between preparation/read, missing selected hash/format, slot/self-slot symlink swap, workspace alias/root replacement, hardlinks accepted; unsafe lookup errors do not become empty pattern sets; anchors still resolve against the same held projection. Export-only remains no native addresses.
3. Implement selected conversion/raw preparation and no-replace publication using existing owners. Tests: raw same-stem/nonUTF8/invalidXML byte equality; mismatching native media/suffix with unchanged original native address; genuine output collision; native-address collision; new-path occupant race before Copy and Hardlink publication; foreign same-byte mode untouched; recorded mode-only update retained. Earlier-hook spy must remain zero when a later declared conversion/output plan is invalid.
4. Thread actual ordinary/native/backend/analyze/replay carriers. Positive actual emit_package_units Simple XML-under-md and Normal converted source; independent expected input format differs from lane target; bound analyzer success; unbound missing-format refusal; generated substituted unit + static children and replay overlay do not become authored sources. Missing independent replay selection refuses before publication. Legacy exact bytes/no-descriptor routes remain unchanged.
5. Root reviews complete install→derive→record→read/boot matrix, format compatibility and traceability. Pure inspector or an unused map constructor cannot promote A3.

Root has already approved data-only bridge direction, original-address ownership and explicit expected materialized format. Remaining packet-level choices are concrete: exact bound overload plumbing, and preserving/refusing unsupported vocabulary/census cases without silently broadening them. No shared record-owner extraction or new schema is presently necessary. Source-derived concerns above are not claimed executed failures; only the next bounded runtime discriminators can establish those failures.
