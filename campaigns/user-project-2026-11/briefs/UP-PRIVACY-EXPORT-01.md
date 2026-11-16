# UP-PRIVACY-EXPORT-01 — reserved personal paths at publication admission

Date: 2026-11-16. Bounded implementation brief; builder owns implementation/tests,
architect owns contract/review. Existing relayed direct USER-PROJECT mandate governs
execution separately. No live publication, credentials, source commit, host/runtime
configuration change, cleanup or provider launch is authorized here.

## Exact inputs and scope

Committed source base `6a83be1dd35d7bec2de4911ee375c2be3cb8686a`, plus exact committed
version of this brief and BOOT-PRIVACY-DESIGN.md contract delivered in c10fdfd8.
Builder binds actual assigned root/branch and verifies base and contract commits.
No artifact-capture/root/epoch overlays, uncommitted prerequisites or peer-tree writes.
Read-only census `/fast/tmp/up-privacy-census-cyftbw8j/CENSUS.md`, reported SHA-256
`b0fe18ddbac64048c13b12438fe018e1f8af7e1b43f1d21a28033c5629d42335`, is design evidence,
not product input or blanket acceptance. No new dependency or global error redesign.

First atom covers central classification plus workspace/direct/layout publication
and tracked checker ONLY. CLI mirror checkout-index/history bundles and xtask source
archives remain explicit uncovered frontdoors; registry acquisition/hash identity
and app downloaded-archive handling remain separate. Do not claim global privacy
complete or enable personal artifact production based on this atom alone.

## Exact allowed paths

- `crates/vibe-core/src/layout.rs` (module/export registration only)
- `crates/vibe-core/src/layout/personal.rs` (new classifier; inline tests)
- `crates/vibe-workspace/src/publish/staging.rs`
- `crates/vibe-workspace/src/publish/tests_staging.rs`
- `crates/vibe-publish/src/git_publish/payload.rs`
- `crates/vibe-publish/src/git_publish/tests.rs`
- `crates/vibe-publish/src/git_publish/submodules.rs` (names admission before checkout reads)
- `crates/vibe-publish/src/orchestrator.rs` (privacy preflight ordering only)
- `crates/vibe-publish/src/layout_publish.rs`
- `crates/vibe-publish/src/layout_publish/personal_admission.rs` (new; inline tests)
- `crates/vibe-publish/src/layout_publish/git_tree.rs`
- `crates/vibe-publish/src/layout_publish/tests.rs`
- `crates/vibe-check/src/checks/boot_directory.rs` (registration/call only)
- `crates/vibe-check/src/checks/boot_directory/tracked_personal.rs` (new; inline tests)

No other product/test paths, manifest/generated schemas, hash_recipe, root ignore,
installed vibedeps, Git metadata or boot producers. Needed extra owner is a precise
perimeter request. Inline tests permitted in listed implementation owners; do not
artificially enlarge a module to avoid requesting a focused test split.

## Pure central classifier

Proposed `classify_personal_artifact(&Path) -> Result<PersonalArtifactClass,
DeclarantPathFault>`, Public/ReservedPersonal, using existing exported declarant-path
law without a safefs reverse dependency. Signature spelling may follow existing
public type conventions; no filesystem access, manifest discovery or contents.

Use central current/new/legacy layout constants. Reserve the directory itself and
all descendants of any nested supported `<specs>/boot/local`, including
`vibevm/vibespecs/boot/local` and legacy `spec/boot/local`. Match components, not
substring prefixes; boot/locality, arbitrary local elsewhere and shared boot files
remain public. A nested package root need not be discovered by reading its manifest.
Conservative ASCII case matching for reserved components protects Windows aliases
on any exporting host; reject unsupported ambiguous components through existing
portable component law, not homemade lossy normalization. Non-UTF8, absolute,
parent-traversal or malformed selected relative names refuse at publication.

When a reserved prefix is already established, do not inspect its descendants to
validate/classify them: the subtree is excluded without entry/content reads. Do not
traverse excluded data merely to produce a prettier diagnostic. Checker reports safe
tracked path names only, never file contents or external personal-root locations.

## Filesystem-copy frontdoors

Workspace stage_node/copy_tree_excluding and direct copy_dir_controlled/walk_controlled
classify each relative entry BEFORE descent, is_dir/is_file/file_type-dependent
following, reading or hashing. Prune reserved subtree immediately. Preserve legacy
nonreserved exclusions and cancellation behavior. No enumerate-everything-then-filter
list and no leaf-only predicate. Both staged and final payload omit reserved data.

Submodule paths are inputs too: workspace inspect_populated_submodules and direct
inspect_submodules must classify indexed gitlink names BEFORE checkout existence,
.git/status/manifest or descendant reads. Reserved tracked gitlinks refuse rather
than silently exporting a substituted identity. Direct orchestrator privacy admission
must precede repo creation, pushes or other external preparation; preserve existing
registry/consent policy and don't create an empty remote before discovering refusal.
No .gitmodules content under a reserved root may be used as a discovery shortcut.

Symlink/reparse/unsupported entries must not redirect traversal into personal data.
Use existing refusal/no-follow policy; if existing walker follows aliases, refuse
such entries rather than bless target canonicalization. No arbitrary external root
traversal or automatic deletion. This is not permission to redesign exporter
transactions or silently omit ordinary unsupported entries.

## Pinned GitTree publication admission

At publish_batch preflight, inspect every selected immutable tree's NAMES before
content_hash, filesystem checkout, git fetch/read-tree -u/checkout-index, insertion
or target publication. Use names-only NUL-delimited Git tree enumeration and strict
lossless decoding/component classification. Refuse any reserved path; do not open
personal blobs, strip tracked files or claim the original identity for altered data.
Tree object enumeration is permitted; personal blob content is not required.

Bind validated full tree OID to execution. Do not validate one ref then execute its
changed tip. git_tree::insert/checkout receive admitted immutable identity or apply
defense checks if separately reachable. Preserve advertised tree identity and final
index/tree verification. A foreign/unavailable/malformed tree or failed enumeration
is refusal, never empty inventory. Preflight entire batch before target effects.

A GitTree item also carries a filesystem payload used by content_hash. Establish
its binding to the validated selected tree before hashing/reading that payload;
otherwise clean tree names plus a mutable checkout containing extra private data
would bypass protection. Either prepare from admitted tree only after names pass,
or safely validate/prune the filesystem read boundary while preserving the pinned
identity and refusing mismatch. Do not hash personal payload first to discover the
mismatch. Existing mirror may already have read blobs upstream: that remains the
explicit separate mirror blocker, not proof this frontdoor is universally safe.

## Tracked checker

Add names-only cached-index census (NUL-delimited, no blob contents) and report
tracked reserved paths using existing BootDirectory check/report family. Preserve
existing generated-leaf checks and non-Git behavior. A nonrepository/absent Git may
remain silent under incumbent checker semantics; an actual publication boundary
must fail closed instead. Do not silently treat malformed tracked-name data as a
clean check. No wire/schema field additions or automatic git rm/ignore edits.

## Required proof and delivery

- Nested current/legacy layouts, exact subtree root, descendants, conservative case,
  locality/lookalike negatives, unsafe/nonUTF8 names and independent package roots.
- Excluded subtree with unreadable directory and symlink/FIFO poison: prove no
  descendant read/traversal/hash, not merely absence in archive. External sentinels
  unchanged; negative proof must work despite privileged test users where relevant.
- Direct/workspace/localdir/monorepo copy staging and final inventory; cancellation
  before/during preflight preserves existing outcomes without private diagnostic dumps.
- Synthetic Git selected tree with reserved tracked blob refuses before checkout,
  hash or target effects, even ignored-in-worktree files force-added to the tree.
  Dirty checkout with private extra path beside a clean selected tree also refuses
  before private read. Ref change cannot swap validated tree; no silent stripping.
- Clean pinned trees retain exact identity and ordinary publication behavior;
  multiple items with last invalid reserved tree refuse before target publication.
- Tracked checker detects reserved names, does not read blobs or untracked ignored
  private contents, preserves shared STATIC/INDEX and existing generated checks.
- Adjacent publication/layout/check tests, nonzero selected counts, Conform/fmt and
  actual commands/outcomes. No real remote push/publish or secret-file inspection.

Return immutable exact manifest/diff, consumer/frontdoor matrix, input provenance,
negative read-boundary evidence, failures/skips/unrun coverage. Architect reviews
this complete atom before acceptance. Mirror/history/archive blockers remain explicit;
this does not authorize local composition/boot producer activation or whole campaign
acceptance. Reuse valid unchanged evidence; do not manufacture full reruns by habit.
