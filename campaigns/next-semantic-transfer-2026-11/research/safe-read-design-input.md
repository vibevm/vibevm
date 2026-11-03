# Safe-source continuation: design inputs from current consumers

Read-only investigation in `/fast/git/wt/next-semantic-transfer`, following accepted F01 lexical containment. No source, build, map or spec writes, and no tests executed for this investigation. Main implementation evidence is the current worktree; prototype references are the previously pinned next commit `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. These are design inputs, not an implementation packet or an accepted architecture.

## Conclusions that constrain the next atom

1. **Hardlinked package source is a supported main feature.** Directly replacing `FsSectionSource` reads with `Project::read_file_snapshot_bounded` would break it, because that SafeFS API rejects every link count other than one.
2. **Relative/root-alias entry is supported by workspace discovery and permissive public resolver construction.** `Project::open` instead requires an absolute path and no-follow opens every ancestor. Root admission needs explicit handling; a blanket `is_absolute` refusal in FileResolver would introduce a compatibility regression.
3. **There is no existing 64 MiB source-size law.** Builtin source uses unbounded `read_to_string`, custom source uses unbounded `read`. A coherent read can fence growth by its initially observed finite length without introducing an unexplained document quota, but requires fallible allocation and a deliberate read-only shared-inode mode.
4. **The needed coherence is per document observation.** The existing public consumer can select a source, obtain one byte snapshot, project/parse that same buffer, and return its section. This does not require the prototype's prepared-world/resource-address/proof stack. It also does not establish a transaction across the entire recursive embed closure.
5. **Pattern duplicate refusal is a compatibility decision.** Main explicitly documents lexical-first deduplication in code. The permanent spec promises all matching document members and fail-loud uniqueness generally, but does not explicitly ratify that duplicate tie-break. The replacement needs a concrete amendment and tests; prototype behavior must not become authority merely by being newer.

## Existing SafeFS boundary

`crates/vibe-safefs/src/file/bounded.rs:Project::read_file_snapshot_bounded` already provides the relevant mechanical core:

- component-checked, capability-relative no-follow path walk;
- regular file plus **single-link** verification at initial open and after passes;
- metadata length checked against caller cap;
- two bounded reads over the same held file handle;
- comparison of the two byte vectors, opening/closing length and Unix mode;
- final filename reopened through the same held parent and compared by opaque object identity;
- `StableFileSnapshot { bytes, sha256, size, unix_mode, identity }` describing those exact bytes.

It is read-only. `Project::open` and `root_dir` do not create `.vibe`, a lock or a COW copy. Root pinning walks from the filesystem anchor through `Pinned::open_child`, rejecting links/reparse points in the root's ancestor path. `holder_of` descends relative parents without following links. `ensure_still_final_name` validates the final file through its pinned parent; it does not rewalk every original absolute ancestor and prove the old textual absolute path still names that same parent after directory rename. This is a finite observation window and capability anchoring, not global namespace immutability or hostile-ABA prevention.

`bounded_pass` currently uses `Vec::with_capacity(metadata_len as usize)` and `reserve_exact`. A huge caller cap does not turn allocation into a fallible typed observation. Passing `usize::MAX` fails cap+1 immediately; choosing `usize::MAX-1` merely evades that check and is not a principled source-reading policy. `digest_file_in` can stream arbitrary-size content with fixed memory, but returns no bytes and also rejects hardlinks; hashing with it and then reading the name again would recreate the coherence defect.

## Supported materialization and roots

### Hardlinks are an actual production path

- `crates/vibe-core/src/manifest/package.rs:Materialization::Hardlink` parses authored `materialization = "hardlink"`.
- `crates/vibe-workspace/src/install/placement.rs:28` selects `CopyMode::Hardlink`.
- `crates/vibe-workspace/src/vibedeps.rs:CopyMode` states that the full slot tree is preserved, only byte-sharing differs.
- `vibedeps/slot_diff.rs:place_hardlink` uses `fs::hard_link(source, destination)` around line 483, with copy fallback only when linking fails.
- `vibedeps/tests.rs:materialise_hardlink_mode_places_the_full_tree` constructs this placement.
- `PROP-022#HARDLINK-MODE` explicitly allows initial cache/source-file hardlinks, retains equal files on refresh and unlinks changed destinations before replacement.

Therefore an unchanged `spec/DOC.md` in an installed package can legitimately have more than one link. The root/selected-world metadata does not currently carry a source-read exclusivity proof; `SelectedPackage` has only version and root. Ordinary authored checkout files also currently read successfully when hardlinked because `std::fs::read` imposes no link-count rule. I found no explicit promise of exclusive ownership for authored source reads and no source-specific test ratifying hardlink rejection. A blanket new rejection is not justified by the existing write/state-file single-link law.

A reasonable design candidate is a **separate read-only regular-source snapshot operation** that permits an existing shared inode, preserves no-follow file/directory traversal, verifies stable object identity and content/length/mode, and makes no exclusivity claim. Keep mutation/state APIs' existing single-link behavior intact. If link-count stability itself is required, record and compare opening/closing count rather than requiring one. An extra alias can mutate the object after observation; that is already true of a writer with the original name and is not solved by silently breaking hardlinks. No read-triggered COW is appropriate.

This operation can be explicitly named at the SafeFS API boundary; introducing a whole source-authority wrapper hierarchy is not necessary. Root should choose whether shared-source observation is general for all source reads or requires additional materialization metadata. The latter would widen consumers and still needs a policy for hardlinked authored checkout files.

### In-place is a directory, not a symlink placement mode

`vibedeps.rs:materialise_in_place` moves a fetched git clone to the unversioned `vibedeps/<group>.<name>/` slot, retaining `.git`; `in_place_slot_abs_path` owns that path. `extension_world.rs:slot_root` and `install/bootgen/materialised_read.rs:durable_row` distinguish InPlace from versioned Copy/Hardlink roots. There is no symlink-based in-place materialization in these owners.

A selected provider can use that unversioned root. The prompt resolver uses `request.provider_root` as self root and exact selected roots for embedded dependencies; it does not rederive a newest slot. Preserve this. Safe read must not assume every selected package root ends in a version directory.

### Relative paths and root aliases

`FileResolver::new`, `with_roots`, `with_selected_world` and `SelectedPackage::new` store caller paths without canonicalization or an absolute-path precondition. Existing `FileResolver::new(Path::new("."), ...)` tests primarily exercise refusal/pattern grammar, so they alone are not successful-relative-read proof. However the public implementation and plain filesystem joins currently accept relative readable roots. `vibe-doc::citations::SpecSources::for_checkout`, `with_store`, `with_in_tree` and `with_lock` likewise preserve caller paths and delegate to FileResolver; the source chain does not impose absolute-only roots.

Production workspace composition is stronger:

- `Workspace::discover`, `discover_selected`, `discover_with_selected_manifest` and `load` call `canonical` once at their public entry.
- `crates/vibe-workspace/src/discovery.rs` explicitly explains that relative/`./`/trailing-separator/symlink/junction spellings must bind one canonical selected-node epoch, and why canonicalizing twice after retargeting is wrong.
- `lib.rs:canonical` calls filesystem canonicalize then strips Windows verbatim prefix.
- workspace member expansion canonicalizes each matching directory.
- boot and extension-world callers ordinarily derive their roots from this canonical Workspace.

This is evidence that explicit caller root aliases are legitimate entry spellings, not that every symlink below a selected root is legitimate source traversal. No evidence was found for a separately supported symlinked *installed slot* mode; copy/slot reconciliation already refuses a slot that is itself a symlink.

The design should distinguish:

1. **Explicit caller-owned workspace/checkout/provider root selection**: resolve its relative/alias spelling once in the existing composition/admission epoch, then pin that admitted physical root. Preserve the user's logical/display spelling and exact overlay keys separately if needed.
2. **Package slot derived from a workspace and document/layout descendants**: walk below that admitted workspace/package root no-follow. Do not `canonicalize` a final document or a discovered slot to make a SafeFS open succeed; that would bless an out-of-root redirect.
3. **Caller-supplied selected map**: the contract says its root is authority. Root must state how that authority is admitted and when it is revalidated; silently recanonicalizing it on each read can change which physical provider it names. A map built from lock-slot paths should obtain no-follow physical admission from its existing owner rather than treating every filesystem symlink as new intent.

The relative-root successful-read and explicit root-symlink behavior need new compatibility controls before changing these constructors/consumers. They can be tested without changing process-global cwd: create an owned fixture under the existing cwd or use an already relative fixture path, and keep all constructed paths scoped; a cwd mutation in parallel unit tests is not an acceptable fixture mechanism.

## Size and representation

`crates/vibe-specdoc/src/load.rs:load_spec_text_with` calls `std::fs::read_to_string` without a byte cap. `FsSectionSource::read_resolved` uses `std::fs::read` for custom bytes; overlay bytes arrive already held as `Arc<[u8]>`. I found no source-byte maximum in the inspected compiler/source contracts or read entrypoints. Existing external ABI limits, if any, would be separate transport/execution boundaries and cannot justify a universal filesystem-source cap.

The prototype's `SOURCE_CAP = 64 * 1024 * 1024` is an implementation choice, not a main requirement. Two defensible designs to choose explicitly:

- Preserve current source-size semantics: open the held regular file, inspect finite initial length, checked-convert to usize, allocate fallibly, fence each pass at initial length+1, reject growth/length mismatch and unavailable allocation. Return a clear observation failure, never truncation or panic. This provides a measured stability fence without imposing a new policy quota.
- Introduce a product-level configurable/declared source resource budget with default, override and compatibility rationale. That is additional feature scope and should be stated as such; do not hide it as a security constant.

Both still retain the source in memory because existing Markdown/XML/custom text consumers require a complete buffer. A raw digest can be computed from that buffer; performing a second path-based read for a digest or parser is wrong. A large-file test only needs to cross the *chosen* boundary if a boundary is chosen; allocating 65 MiB solely to disprove the prototype number is not the best oracle for a no-policy-ceiling design. Allocation/size-fence hooks can exercise errors deterministically, backed by a moderate real file.

## One observed buffer through current consumers

Current source entrypoints are `FsSectionSource::section_text`, `source_metadata`, and `read_resolved_source`/private `read_resolved` in `crates/vibe-spec/src/embed.rs`.

Suggested flow, without committing to new public types:

1. Existing resolver determines the selected root and exact source path using existing routing and active formats. Path spelling was already handled by F01; future no-follow/alias checks should use the admitted root/relative path.
2. Check the **existing exact lexical overlay key** before filesystem content reading. If present, borrow the held Arc bytes; never fall back after overlay parse/UTF-8 failure and never open/read disk merely to hash the overlay.
3. Otherwise obtain one regular-source snapshot through SafeFS and borrow its bytes. Both builtin and custom branches use this same seam.
4. Builtin Markdown/XML uses existing pure `vibe_specdoc::project_spec_text(path, utf8_text)`; it already contains the one extension dispatch and XML→Markdown projection. Then run the existing section selection on that projected buffer.
5. Admitted custom frontend text remains raw UTF-8, with the existing physical stem and format identity; do not run custom bytes through builtin projection. `compiler/pass_tier/frontend.rs:AdmittedSource::load` must still admit a selected frontend **before** `read_resolved_source` is called.
6. Preserve logical/display paths and exact overlay keys. Canonicalizing a root for physical access must not cause a previously exact overlay key to miss or a case-near-miss key to start matching.

Main `ResolvedSource` currently carries selected text/format/stem, not a raw-digest public report. A private byte source or snapshot return in the internal FileResolver helper may be enough. Do not add durable capture identities or prepared world wrappers unless a present consumer needs them.

An existing race remains between public `source_metadata` and later read: metadata intentionally locates/admit-checks before reading. The read must validate its own root-relative path and capture safely instead of trusting a public metadata object's path as fresh physical proof. A changed source between separate calls may legitimately be a new observation or may need typed stale behavior; do not claim metadata itself pins bytes unless actually changed to hold a handle/snapshot. The narrow full pipeline guarantee can be “each executed read returns a coherent current snapshot or refusal”, not “every path lookup earlier in the command belongs to the same filesystem epoch.”

## Alias and race test matrix through actual consumers

Useful existing tests/seams:

- SafeFS `file/bounded_tests.rs:stable_snapshot_binds_bytes_state_and_identity_to_one_epoch`.
- `arm_before_bounded_read`: one-shot, thread-local, fires after held open/metadata and before reads; can replace the final name with a same-byte different inode on Unix, append content, or mutate through a second hardlink.
- `arm_bounded_read_identity_check`: deterministic final identity refusal on platforms that disallow rename-under-open; must report it as simulated comparison, not a native rename test.
- `arm_between_stream_passes` currently belongs to streamed digest. **Snapshot two-pass code does not invoke it.** Do not claim a snapshot same-length-between-passes test exists merely because the stream test does. If that exact snapshot window is required, add a bounded snapshot hook at the correct owner/test-only seam.
- `component::identity_key` and `judge_selection` exist; SafeFS `Pinned::open_child` itself does not enumerate sibling names to reject a case/NFC alias. A no-follow snapshot alone is not portable spelling identity proof.

Required actual consumer regressions for the safe-read atom:

1. `FsSectionSource` builtin MD and XML succeed from ordinary source and from hardlink materialization; compare returned text and unchanged source/slot bytes, inode link count and filesystem inventory. No COW/lock creation.
2. Same-byte final-name replacement during actual `section_text` refuses; portable identity-hook case separate from native Unix rename. Final path remains the injected state; no repair.
3. Growth during held snapshot refuses instead of returning an accepted prefix; same-length in-place rewrite between passes refuses if an appropriate snapshot hook is added.
4. Custom frontend admitted read uses the same held boundary and preserves exact raw text; unselected custom frontend still has zero content reads/invocations.
5. Overlay wins over differing disk and does not trigger the filesystem read hook. Invalid overlay bytes refuse without fallback. XML overlay still projects; exact-case near-miss stays a miss.
6. Relative root control and explicit caller-owned symlink root admission control; symlinked descendant directory/final source and reparse analogue must refuse. Treat selected in-place unversioned provider root as a separate positive control.
7. Portable alias cases: exact spelling with a competing case/NFC sibling, requested alias to an existing different spelling, nested parent alias, active custom-format alias/collision. Use `identity_key` to compare relevant candidates; do not apply a whole-directory global veto for unrelated files without a selected-set requirement. Native case-folding/macOS evidence stays separate from portable tests on Linux.
8. SelectedWorldPromptResolver regression: exact provider and locked dependency still read, unselected dependency/pin mismatch refuses; one actual prompt/embed race test demonstrates the consumer receives a refusal instead of changed prompt content.

Targeted verification can stay at `vibe-spec --lib` resolver/embed/custom frontend families, focused `vibe-safefs --lib` new source-snapshot family (plus existing strict readers to prove their link policy unchanged), and `vibe-lifecycle --lib` agent resolver cases. A workspace hardlink integration fixture should create placement through `materialise_with(...CopyMode::Hardlink...)`, assert a real link was obtained on that host, and read the package through the actual source consumer. Copy fallback must not count as a hardlink acceptance proof.

## Pattern ambiguity contract and compatibility

Current `resolver/glob.rs` explicitly encodes:

- `*` only in package **name**, matching any byte run including empty; repeated stars allowed; no `?`/`[...]` metacharacters.
- group-blind package-name matching; the requested group is copied into emitted addresses.
- membership = name match AND addressed document present.
- empty set succeeds; non-pattern expansion returns its input without checking its document.
- sorted byte-order names, newest installed version per member.
- point resolution of a pattern refuses.

The comment at the top saying group is absent from slot directory names is historical/stale relative to current `<group>.<name>` layout; code still deliberately strips at the last dot and ignores the group. Do not silently reinterpret that grammar while fixing failure handling.

At `glob.rs:109–119`, matched `(name, slot_dir)` pairs are sorted by name then path and deduplicated by name. The comment explicitly calls the surviving slot lexicographically smallest. At `:130`, document lookup is tested with `.is_ok()`, so ambiguity, unreadability and genuine absence all remove a member. `read_dir_or_empty` also suppresses directory/entry failures. These are distinct failure classes:

- **Document ambiguity/unreadability inside a selected matching member:** propagation follows the existing fail-loud document law; only real DocNotFound should mean nonmember.
- **Same name in multiple physical slots:** replacement of documented lexical-first choice needs an explicit duplicate-member law. Desired deterministic fail-loud behavior should identify competing roots and preserve ordering of all unambiguous results.
- **Only one of duplicate name slots actually carries the requested document:** prototype rejects duplicate names *before* checking document membership. That is broader than “two matching members are ambiguous” and conflicts with a literal membership=name+document interpretation. Root must decide whether name collision itself is forbidden at the selection boundary or ambiguity is determined after membership evaluation. Do not import this ordering accidentally.
- **Group-blind expansion produces addresses with caller group:** do not use a new qualified-root algorithm under the guise of refusal hardening. If the chosen duplicate policy exposes inconsistencies here, record a separate semantic decision.

Preserve main `g1_two_matches_in_name_order`, `g2_sort_independent_of_creation_order`, `g3_non_matching_excluded`, `g4_match_without_doc_is_not_a_member`, `g5_empty_set_is_legal`, `g6_freshest_version_selected`, `g8_star_matches_empty_and_all`, `g_non_pattern_expands_to_self`, `g_anchor_and_raw_round_trip`, and `g_is_pattern_only_name_star`. Add explicit duplicate-member tests in both creation orders, one-doc-present/one-doc-missing split, unreadable matched slot, ambiguous doc, real absent doc, absent dependency root, and multi-star success. Prototype `m_01_a_3_cases` provides useful refusal examples, not the whole compatibility oracle.

## Decisions root needs to freeze before dispatch

- Named source-read API that admits stable shared regular files while existing state/write readers remain single-link.
- Root admission: handling relative/trusted root aliases once, versus no-follow derived slot/document descendants; exact overlay key preservation.
- Observed-length/fallible-allocation semantics versus an explicitly authored resource budget; no unexplained 64 MiB.
- Scope of per-read coherence and namespace revalidation, without promising whole-closure atomicity.
- Duplicate pattern policy and its membership-order decision, preserving the existing wildcard language.

These are concrete compatibility decisions with existing consumers, not reasons to require the full prototype framework or finish unrelated campaign archaeology first.

## Final addendum — StableFileSnapshot consumers and reusable implementation primitives

F01/baseline are now integrated into `/fast/git/vibevm` main at `504b42f76`. The old worktree is no longer used by this worker; all prior commands exited, and no process or file handle from this task remains there. Remaining inspection target is main. This addendum answers the explicit follow-up on single-link authority.

### The existing snapshot reader's single-link contract is used beyond its own method

Direct production callers of `read_file_snapshot_bounded`/`read_snapshot_bounded` in the inspected main tree are concentrated in `vibe-scrape`, plus SafeFS external transaction-store wrappers:

- `crates/vibe-scrape/src/rewrite/text.rs:read_candidate` reads the exact inventoried rewrite target and compares snapshot digest, size, Unix mode, object identity and retained byte length against inventory before constructing a rewrite. It does not independently count links; it inherits the method's no-follow/single-link condition.
- `crates/vibe-scrape/src/plan/projected.rs:read_inventoried_bytes` uses the same comparisons when assembling the final projected tree. This is mutation planning, not simply showing a document.
- `crates/vibe-scrape/src/lib.rs:load_contract` parses held bytes into the execution contract, carries snapshot identity/digest and checks contained/delete-last versus external/preserve behavior. Loosening its file policy would change the accepted contract source boundary.
- `crates/vibe-scrape/src/health/prepare/modes.rs:prepare_bundle` checks inventory identity/digest/size/mode and retains verifier bundle bytes; health local/system asset readers bind exact launch assets to the observed opaque identity.
- `crates/vibe-safefs/src/transaction/external.rs:ExternalStore::read_stable_bounded` and `ExternalDirectory::read_stable_bounded` return **the same** StableFileSnapshot from the existing method. Their callers include `vibe-scrape/src/transaction/store/inherent_methods.rs:load_owner/load_journal`, which parse authoritative transaction owner seals and recovery journals; store trait methods read stable reports before/after durable publication and prior to retirement checks.
- `crates/vibe-scrape/src/transaction/traits.rs:TransactionFilesystem` explicitly names no-follow walks, **single-link regular files**, identity rechecks and namespace ownership in its contract.

No inspected downstream code reads an `nlink` field from StableFileSnapshot, because it has none. The property is guaranteed by successful reader return and inherited by callers, not re-evaluated through the value. The struct's fields are public and it is not an unforgeable authorization token; nevertheless its documentation says “regular single-link file” and existing APIs/consumers rely on that checked observation boundary. That is sufficient reason **not to widen StableFileSnapshot's advertised contract or make old readers permit shared inodes** to serve source reads.

A distinct source-read API may share low-level implementation, but should not silently return shared-inode content under the old single-link guarantee. If the only current source consumer needs bytes, returning the coherent bytes from a specifically named source-reading method can avoid inventing a proof object. If the new method must expose identity/mode/raw digest, use a distinct appropriately named observation data carrier with an honest shared-inode contract, not a reinterpretation of StableFileSnapshot. Root owns that return-shape choice.

### Existing reusable primitives and their visibility

Inside `vibe-safefs` the new method can reuse:

- `file::cap_options()` (`pub(crate)`): creates cap-std open options with `FollowSymlinks::No`.
- `Project::root_dir()` plus `Project::holder_of()` (currently private in the Project implementation): pin root and safely descend to a file's parent using shared `split_relative`.
- `Pinned.dir` (`pub(crate)`) and `open_child/open_child_checked`: capability-relative file/directory operations; no ambient re-open is needed.
- `file::identity::file_identity` and `number_of_links` (`pub(crate)`): already implement Windows file-information and Unix inode/link-count observation. No platform-specific reimplementation belongs in vibe-spec.
- `file::stable::unix_mode` and `std::fs::File::metadata`: held-file metadata.
- the existing bounded read loop, `READ_CHUNK`, `Read/Seek`, and race hooks.
- `Project::child_names`/`child_names_bounded` in `project/enumerate.rs`: enumeration through a retained directory, with entry/non-UTF8 failures propagated. This is available for any deliberately selected sibling-spelling checks; no ambient read_dir is required by the low-level API.

`verify_regular_single_link` and `ensure_still_final_name` currently encode the strict link-count policy. A shared internal mechanism can factor the regular-file/object-identity checks while preserving strict policy at every old call. Merely bypassing the initial `verify_regular_single_link` is insufficient: the final-name checker invokes it again. Neither existing helper should be broadly weakened.

### Candidate mechanics for the root's stated read-only design

The proposed second-pass chunk comparison fits the current machinery and avoids a second full byte vector:

1. Open the selected file once through the no-follow held parent; check that the held handle is regular. For the new source-only API, observe the opening link count if it is part of the chosen stability contract, without requiring one.
2. Read held metadata length; checked-convert it to address-space length. Create an empty Vec and use `try_reserve_exact`, returning a diagnostic on capacity/allocation failure. The initial length is a growth/coherence fence, not a policy quota.
3. Read at most the initial extent plus one detection byte in fixed chunks. Reject growth instead of extending beyond the measured extent; reject early EOF/length change rather than presenting a truncated prefix as success.
4. Seek the **same handle** back to zero. Compare each new chunk against the corresponding retained first-pass bytes; reject any mismatch, early EOF or extra byte. Keep only one full retained buffer plus the fixed chunk.
5. Compare held closing metadata/identity/mode and the chosen link-count observation. Reopen the final name through the **same parent capability**, verify regular type and equal object identity under the new source policy, then return the held first-pass bytes (and only the explicitly requested observation metadata).
6. Old bounded mutation/state readers retain their original single-link checks and caps. Sharing a low-level loop must be verified by their existing hardlink-refusal and cap tests as well as new source acceptance tests.

There is no need for a new universal memory policy, source-proof strength or arbitrary 64 MiB ceiling to implement this candidate. A failure after a caller-owned root is admitted remains a read failure; it must not trigger canonicalization of the document, a fallback std::fs::read, repair, COW or a fresh epoch retry hidden from the caller.

### Root admission and overlays, made concrete

Only the caller-owned **root selection** may be resolved from relative/symlink spelling to one physical root, at the owner that already performs selection. Preserve that admitted result for the read epoch. From there, choose current/legacy `vibevm/vibespecs` versus `spec` and package/version/document descendants by no-follow capability traversal. Do not canonicalize `resolved_file`, `spec` directory or a derived package slot merely because an open refused: those descendants are exactly what the safe-read boundary must police.

Current `specs_root_under` makes a path-based `is_dir` layout probe. A new read must not treat that probe as physical authority; if a symlinked `vibevm/vibespecs` wins that probe, subsequent no-follow admission must refuse, not fall back to a different layout or canonicalize it. Likewise a selected root supplied directly by a trusted embedding caller is different from a slot path derived internally from a trusted workspace; root should freeze how those two cases are admitted before dispatch.

The exact logical `PathBuf` emitted by lookup is still the overlay map key. Keep its exact spelling for that lookup and for format/physical-stem metadata; do not replace it with the canonical physical path. Once overlay bytes are selected, the source reader does not need a disk file snapshot. Tests should arm the filesystem hook and prove it does not fire for the exact overlay, while near-miss keys still use the disk path and its safe read. This preserves existing behavior without another public authority wrapper.
