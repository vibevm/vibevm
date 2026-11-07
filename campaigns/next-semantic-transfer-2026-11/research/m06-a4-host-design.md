# A4 host sources and cited synchronization design

Read-only design candidate, 2026-11-07. Root judgment/activation is required before implementation. No product/spec edit, Cargo/test run, Git mutation or delegation. Package-query work is separate and must not be duplicated. Original A4 note8 is the scope: validated project.sources, shared selector, exact cited no-follow capture, refusal before registry writes, bounded legacy compatibility.

## Actual state and minimal seams

`ProjectSection` currently has name/group/version/spec_format/authors and deny_unknown_fields; project.sources is unsupported. `select_source_rows(coordinate, rows, file)` already implements package/project-independent exact-file > longest component-directory precedence and ambiguity; no new selector is needed. Package descriptors already validate file/directory exclusivity, normalized paths and media/interpretation/role pairs. Keep these checks; no second host dialect.

`source_policy::PolicyState::mode` consults package metadata only and treats every valid project as Legacy. Merely adding the field would silently activate suffix fallback. Extend the existing held observation/checkpoint to expose role-neutral source intent (coordinate + optional rows), while retaining package() and package_sources() for existing package-only consumers. mode must recognize explicit host presence, including empty. Existing unsupported guards then refuse host operations they cannot yet handle. Keep observation identity/manifest rechecks and no-follow source capability; no new authority/world or installed-record fiction for host trees.

`facts::sync` currently gets host identity from a second ambient Manifest::read through host_package, recursively enumerates md/xml under spec root, filters canonical cited paths, projects by suffix, parses markers, builds the complete snapshot, then mutates registry. Refactor only the explicit branch: obtain coordinate from the same captured validated manifest (consumer_node/group), classify exact cited files and parse held bytes by declared media. The legacy branch remains separately named, including existing absent-source deletion behavior; do not accidentally apply new unavailable-source refusal to old projects.

`observe_authored_source` already branches on policy.mode; its declared_capture assumes policy.package(). After host mode activates it would honestly refuse, not parse suffixes, until a host adapter exists. Do not manufacture PackageMeta for a project to satisfy that API. A small role-neutral pure classifier/read input (coordinate + rows + allowed vocabulary) can reuse select_source_rows and existing media parser; package Doc gating stays package-specific. Sync need not require an entire authored-source census: only cited native files are relevant.

## Grammar and source compatibility

Propose `ProjectSection.sources: Option<Vec<SourceDescriptor>>`, absent None, explicit [] Some(empty), nonempty explicit rows. Reject explicit JSON null rather than treating it as legacy; reuse existing manifest optional-nonnull conventions. Serialize absence omitted and explicit [] retained. Require valid self-coordinate for nonempty semantic host rows; missing group cannot mint spec addresses. Empty needs no invented source identity until a source operation requires it. Root should settle this exact validation edge in the permanent law before activation.

Adding the public field is a Rust source compatibility change to struct literals, not a new wire schema. Current repository has four construction sites besides its definition: `vibe-core/src/manifest/document.rs::new_project`, `vibe-cli/src/commands/init/root_role.rs`, `vibe-install/tests/recording_behaviour.rs`, `vibe-workspace/src/extension_world/tests.rs`. Set sources None in legacy constructors; do not silently opt scaffolds into empty semantics. Check downstream compile users with scoped compile after edits. Root should log authored TOML compatibility in FORMAT-CHANGES: older strict readers reject the new field, including shared manifest handling for install -g; no alternate global layout, lock/slot epoch or machine-report schema follows from this field.

## Citation resolution and directory distinction

The pinned prototype implementation at `crates/vibe-facts/src/sync.rs` rejects any directory descriptor before looking at cited paths. Its test adds a directory row after already creating a two-native-form ambiguity and expects the blanket census error. That test does not distinguish a directory requested as content from an exact native file whose policy comes from a directory default. Do not port its fixed byte/count caps, StableFileSnapshot single-link assumptions, or blanket global directory census.

For exact file declarations, build the finite metadata candidate set whose `canonical_doc_path(original path)` equals the requested document address. Run the shared selector on each candidate, so exact overrides and conflicting rows are respected. Raw/CommonMark/XML-data candidates do not count as native alternatives; their same-stem coexistence must remain legal. Zero selected native candidates refuses; two native candidates claiming the same address refuses. Arbitrary suffix/native media is valid and must not be guessed from extension.

A citation resolving to an actual directory must refuse. A directory-default descriptor that selects an exact regular file is a policy default, not a request for directory enumeration. Prefer the existing held resolver's targeted canonical-name lookup for that cited path, with declared selection before interpretation and no whole-tree fact scan. Only the immediate candidate directory/spellings needed to invert the canonical address may be inspected; noncited directories/files must not be read for semantics. Exact rows plus unrelated directory defaults can work without any census. Root requires useful completed directory-default support in this host slice; an exact-row-only implementation is not the endpoint. A specific unresolved/ambiguous citation can still refuse, but a regular exact native file selected by a directory default must succeed without global census.

### Existing canonical mapping owner and isolated discriminator

`vibe-spec/src/resolver/lookup.rs::canonical_doc_path_with_formats` removes the incumbent new/legacy spec-root prefix, strips admitted .md/.xml suffixes, and truncates PROP/FEAT numeric descriptive slugs. The same module owns ID matching and ambiguity selection. `resolver/observation.rs::locate_in` already descends a held parent directory and uses `Project::child_names` for slug candidates; its existing explicit-policy guard must stay until a classified branch is supplied. Do not call the suffix-only legacy selector on raw candidates and then classify afterward: that would recreate same-stem false collisions.

Factor/reuse its held layout/targeted-name discovery, then compare each candidate's original relative spelling with the shared canonical function and select descriptor before admitting native candidates. For declared media under an arbitrary suffix, the original canonical spelling (including retained suffix when not .md/.xml) is authoritative; do not force guessed extensions. Bounded inventory is the candidate parent(s) implied by the cited logical address and established layout/explicit rows, not all descendants of every directory descriptor. Retain held parent/name observations, reject actual unsafe paths or multiple native claimants, and perform the selected source read through the same held authority. Raw same-stem candidates are not native claimants.

The independent positive discriminator is a fresh host with ONLY a directory-default native row, one file `vibevm/vibespecs/common/PROP-123-readable.xml`, citation `common/PROP-123#LAW`, and an unrelated malformed native file elsewhere. No competing exact native row exists. Sync must update LAW and not read/parse the unrelated file. Then add `PROP-123-other.md` selected as native in the same bounded parent: ambiguity must refuse before writes. In a separate clean fixture add ordinary same-stem XML/Markdown with an exact raw override: the single native claimant must still succeed. Finally cite an actual directory and prove read refusal. These separate cases avoid the prototype's confounded already-ambiguous fixture.

## Read/validation/publication order

1. Determine requested host document addresses from origin=spec registry entries. If none, keep the incumbent read-free no-op. Verify their host coordinate against the captured consumer-node identity, rather than deleting a foreign-coordinate record accidentally.
2. Capture policy once through held root and resolve every cited exact native file as above. No registry mutation during selection, reading or parsing.
3. Read through the existing held SourceObservation/bytes seam; preserve shared regular inode support. Refuse symlink/special/directory/unavailable paths. Parse the returned buffer using declared media and the existing VibeSpec grammar; ordinary text never enters the parser. Keep original logical canonical address and projection-relative diagnostics distinct.
4. Build the complete snapshot with the existing marked-fact/status extraction, full-address duplicate rejection and canonical slug law. Reuse the current progress-core marker extraction; no second status/void grammar. Existing VOID meaning must be pinned by equivalent native fixture; do not reinterpret it as missing source.
5. Before the first registry write, recheck captured root/manifest and the finite cited byte observations. A late failure in a later cited file leaves registry bytes and in-memory entries unchanged. Recheck failure is an error, not an empty snapshot. Then compute mismatches and apply existing upsert/remove tie-break. This is prevalidation, not a whole-registry transaction or hostile CAS guarantee; ordinary publication failures may still partially apply as today.

## Absence and refusal decisions

| Observed state | Explicit host sync result |
|---|---|
| Admitted native document parses; cited anchor remains with status | Copy that status into host registry; leave source untouched. |
| Admitted native document parses; cited anchor genuinely absent or no longer a marked host fact | Existing remove-overlay result, after all cited documents validate. |
| Native file missing/unreadable, declaration removed, raw-only selection, no selected native candidate | Refuse before writes; cannot infer semantic deletion from unavailable or reclassified source. |
| Ambiguous declarations/native addresses, malformed native syntax, unsupported Doc vocabulary | Refuse before writes with specific reason. |
| Explicit empty and registry contains host citations | Refuse unresolved citations; do not mass-delete merely because policy is empty. Empty with no citations remains no-op. |
| Unrelated malformed or raw file not cited | Do not parse it; it cannot poison cited sync. Manifest schema errors remain manifest errors. |
| Directory default unrelated to cited exact native row | Shared exact-row precedence; no blanket census refusal. |
| Actual directory/symlink/special path requested by citation | Refuse the source read. |

## Activation perimeter and fallback audit

Core grammar plus policy mode must land together. Every existing root-aware guard that switches Explicit/ExplicitEmpty needs a host fixture: resolver observation; facts observe; progress grounding; tree in-place directives; boot-directory/WAL checks; source conversion guard; documentation pages/self-citation/borrowed examples. Some will continue precise Unsupported until their A4 consumer slice. A guard that only matches package() or has_explicit_source_policy on package metadata can still miss host intent, so inspect actual bodies rather than assuming renamed mode covers everything.

Known unguarded semantic consumers include sync (fixed here) and `show effective` authored files_written suffix projection; these must either receive selected host policy or explicitly refuse opted-in host interpretation at activation. `node_own_boot`/authored boot scanning must also refuse or classify host policy before loading directives. Generated STATIC remains its separate artifact lane, never host authored descriptors. Pure specdoc APIs remain filesystem-free, caller-classified primitives and need no ambient manifest checks.

Suggested write perimeter: core project.rs/project tests/document validation and four literal constructors; spec source_policy.rs/tests plus narrow host resolution seam if needed; facts sync.rs/new sync child tests and role-neutral pure observer seam only when needed; existing call-site guard branches that otherwise allow host fallback; governing PROP-072 and campaign FORMAT-CHANGES/LOG by root. Do not touch package-query files concurrently; coordinate any shared facts observer seam before assignment.

## Foundation-first validation matrix

- Grammar absence/empty/nonempty/null/unknown fields, exact-vs-directory precedence and coordinate validation; legacy constructor output unchanged.
- Real project manifest → host registry → check/reconcile: XML declared under arbitrary suffix and Markdown counterpart yield equivalent canonical address/status/void; update and missing-anchor removal preserve source bytes.
- Two cited native documents: first needs update, second malformed/raw/missing/ambiguous; reconcile refuses with both disk registry and in-memory entries unchanged.
- Ordinary same-stem pair and directive-looking literals remain byte-exact and mint zero facts; unrelated malformed native file is not opened for cited-only sync.
- Exact native row plus unrelated/broader directory default succeeds; exact row overriding raw default succeeds; actual directory citation refuses; directory-default-only exact lookup has the mandatory isolated positive described above.
- Held path/manifest replacement/symlink and missing file refuse without writes; hardlinked source remains accepted under existing source-read law. No fabricated global caps.
- Host grammar activation probes each unsupported consumer for precise refusal/no suffix parse and no product writes; bound consumers receive mixed-source equivalence tests. Final root review distinguishes completed host slice from the remaining A4 matrix.

## Root decisions after the design review

This is the next sequential implementation slice after package-query acceptance, not activated grammar. Preserve the useful directory-default-only case with targeted held candidate resolution and isolated tests; an intermediate refusal is not the final acceptance boundary. Reuse canonical path/alias rules and distinguish missing file from missing anchor.

Project rows use existing selector/descriptor validation without a blanket new project.group requirement merely because rows exist. Operations that mint or resolve host spec addresses require the existing valid self coordinate and refuse before writes when unavailable; never invent a group or package identity. Raw declarations and non-addressing inspection do not themselves mint spec addresses.

Activate policy mode and affected guards together. Unsupported consumers must refuse explicitly until their A4 integration; they cannot silently gain suffix semantics. Coordinate shared facts seams with the package-query slice. Root verified both pinned prototype blobs and18of20current source hashes; lib.rs and scan/declared.rs changed under the live approved package-query reader/dispatch work. This is a design snapshot, not a final host source seal, and those two seams must be revalidated before implementation.
