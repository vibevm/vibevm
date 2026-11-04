# Apply file rollback: bounded NEED review

Read-only review of main. Source baseline `df040becb`; observed main HEAD at final read `77b645ff77eba99150edf95d4f990626549603ae`. `git diff --quiet df040becb --` the inspected apply/core-writer/SafeFS source paths exited0. No code edits, builds, tests, worktrees or agents. Counterexamples below are exact source-derived execution sequences, **not reproduced runtime results**. Paths are relative to `/fast/git/vibevm`.

## Verdict and concrete ownership gap

There is a real narrow NEED: `FileGuard` remembers only ambient pathname + old bytes/absence, then assumes unconditional right to overwrite/unlink whatever occupies that name when it drops. It records neither whether its writer actually published nor its own after-image/object. That can destroy a later writer's state, redirect restoration through a symlink, or truncate all aliases of a hardlinked file. This finding does not justify whole-slot snapshots or another transaction store.

`crates/vibe-install/src/apply/file_guard.rs:26` uses `std::fs::read` (follows links) and treats NotFound as absence, including a dangling link's missing target. Drop at:52 uses `fs::write(path, before)` or `remove_file(path)` unconditionally. Restore errors only warn; successful destruction of a third state emits no conflict. A saved byte vector does not prove ownership of a later directory entry.

### Actual apply windows

| Boundary | What is really armed / mutable |
|---|---|
| Selected manifest update, `apply.rs:409–415` | Guard captures selected `project_root/vibe.toml`, becomes armed **before** `Manifest::write`. Any render/read/write failure already drops it; no publication receipt separates “never wrote” from “partially wrote”. |
| Manifest write → lock capture | Guard stays armed across selected-manifest workspace discovery, workspace-root snapshot, deferred `source.materialise_in_place`, package placement/pre-install lifecycle, boot compilation and progress callbacks (`apply.rs:426–518`). Any error causes old manifest bytes to be written over the name as it then exists. This is not a microscopic race: arbitrary source/lifecycle work occurs inside the interval. |
| Lock/provenance captures | Array construction at `apply.rs:524` captures lock first, provenance second. If second capture errors, the first guard drops **although no lock write was attempted**. Both are armed before `record_lock` mutates/serializes anything. |
| Lock record and sidecar | `apply/lock.rs:73` writes lock;:76 writes or removes provenance. Failure in sidecar drops both file guards before outer `outcome.roll_back`; manifest remains armed until function return. Sidecar guard can restore a path its own writer never touched. |
| Durability boundary | Only after both writers return Ok does apply call `outcome.commit`, disarm manifest and both lock guards (`apply.rs:554–556`). Post-install lifecycle is after disarm; its error does not trigger these file undos. |
| Scope | Manifest guard is selected project manifest. `workspace.lockfile_path()` is `workspace.root/vibe.lock` (`vibe-workspace/src/lib.rs:162`), with sidecar beside it; in a workspace these need not share selected manifest's parent. Preserve these distinct authorities. |

The ordinary lifecycle lease cannot make arbitrary editor/callback writes disappear, and `FileGuard` carries no lock/capability itself. The implementation should not claim exclusive file ownership merely because the overall command is serialized against cooperating commands.

## Smallest counterexamples, separated by reachability

**Private helper only, zero concurrency:** start path=A; `guard = before_write(path)`; this operation writes B; another in-process actor writes C; drop guard → A replaces C. For originally absent path, drop deletes C. For same-byte replacement, replace B with a fresh inode containing B, then drop: the new object's ownership is also ignored. These describe `FileGuard` directly; its `pub(super)` visibility and crate `[lib] test=false` mean they are not public integration tests as written.

**Actual public apply seam, no threads/network/new production hook needed:** reuse `crates/vibe-install/tests/incremental_in_place.rs`, `general_install_defers_in_place_instead_of_recloning` (:120). Its real fixture starts with project manifest lacking requires, lock recording one existing in-place package, and an explicit root request (:214); `plan` produces Ready and `apply` demonstrably routes to the supplied `InstallSource::materialise_in_place` (:245). Replace only that mock method's successful body with:

1. Assert root manifest now contains the newly added requires (B), proving manifest write already happened.
2. Write valid distinct manifest C at the same root path (e.g. add an operator comment to B), then return the existing `RegistryError::MalformedMeta` or another ordinary RegistryError.
3. Assert `apply` returns Err and inspect the file. **Predicted from actual bodies:** it holds A, not C.

Reason this is a real apply path: `apply.rs:415` writes B; `apply.rs:443` calls deferred materialization; `apply/deferred.rs:34` invokes the injected source and immediately propagates Err. Unwinding drops the armed manifest guard. Existing test source already implements the exact public trait and reaches this method; it needs only a captured project path and error behavior, not synthetic guard injection or a fabricated apply phase. This proposed test was not built/run here. It proves in-process interleaving, not a claim that the stock Git provider normally edits the root manifest.

A variation atomically replaces root `vibe.toml` with C inside that callback; current Drop truncates the replacement. A variation substitutes a symlink to a synthetic sentinel; Drop follows it and writes A into the sentinel. Both use the same actual apply seam. Hardlink siblings also expose the earlier forward-writer defect: writing B already changes every alias because the writer truncates in place.

The existing public CLI test `crates/vibe-cli/tests/cli_install_rollback.rs:73`, `a_failed_static_compile_leaves_manifest_lock_slots_and_boot_untouched`, asserts ordinary no-third-state rollback restores the project byte tree. It must remain a control, but it supplies no third-state/object-substitution protection. No execution result from that test is claimed here.

## The intended after-image must come from the writer's input

Actual core bodies:

- `Manifest::write` (`manifest/document.rs:464`) and `Lockfile::write` (`manifest/lockfile.rs:543`) delegate to `manifest/mod.rs:124::write_toml`.
- `write_toml` pretty-serializes the in-memory value, **rereads the current path** for comments/decoration, calls private `merge_preserving_comments`, then `std::fs::write`. Read errors silently select the fresh rendered string. Neither forward writer is atomic today. A second `toml::to_string_pretty` alone is not necessarily its actual output because decorations matter.
- `LockProvenance::write_beside` (`lock_provenance.rs:120`) removes an existing sidecar when empty; otherwise sorts a cloned package list and uses the same TOML writer. Its intended after-image is **Absence or the exact sorted/decorated bytes**, not always a file. `record_lock` sets generated_at once before writing; do not regenerate timestamp while computing the supposed expectation.

Smallest defensible preparation seam: pure core rendering that takes the captured prior text/bytes as explicit decoration input, returns the exact byte vector, and is shared with the existing serializer/merge rules. Apply knows that vector **before** mutation and passes exactly it to the publication owner. Build lock/provenance in memory first; represent removal explicitly. Do not copy private comment-merge logic into vibe-install. Freeze old observation, intended bytes/removal, then write; if existing decoration changed meanwhile, treat it as changed input rather than silently adopting it in a hidden writer reread.

A post-write read can verify against the intended bytes. It must not define “our bytes”: otherwise C written before that reread becomes the rollback predicate and is later destroyed. Object ownership requires the identity of **our newly created stage obtained from its held handle before publication**, carried through publication success/possibly-published failure, not the identity of whatever the final name happens to contain on an after-write reopen. Digest equality alone misses a fresh same-byte replacement; identity equality alone misses in-place C on the same inode.

## Existing SafeFS pieces and exact limits

| Existing API/body | Reusable fact | Not supplied |
|---|---|---|
| `Project::open`, `root_dir`, `Pinned`; `file/bounded.rs:73::read_file_snapshot_bounded[_in]` | Held no-follow ancestry; stable bytes + digest + length + mode + opaque identity from one checked epoch. | Snapshot APIs require single link; caller cap is explicit, not a ready-made project-manifest size policy. A snapshot is not a later mutation capability. |
| `stable_file_state_with_identity` (`file/stable.rs:56`) | Stable streamed digest/length/mode + identity for comparisons. | Does not return old byte buffer or conditional replace operation; also single-link. |
| `read_source_bytes[_in]` (`file/source_read.rs:31,48`) | Held stable read supports shared regular files without inventing mutation ownership; allocation fenced by observed length. | Returns bytes only, not a reusable public identity-bearing rollback snapshot; must not be repurposed as write authorization. |
| `write_atomic[_in][_with_mode]`, durable variants (`file/project_methods.rs:5–64`) | Owned create-new stage, whole-byte publication by pinned rename; no truncate-in-place; bounded verification against known candidate; mode and PublishStage support. | No expected old-state argument/CAS. Destination is checked early then rename may replace a later occupant. Result returns directories/durability, **not minted stage identity/held file**. Verification compares candidate content/mode, not identity against stage. |
| `PublishStage::{BeforePublication,PossiblyPublished}` (`publish.rs`) | Distinguishes definitely unchanged destination from maybe-visible output; use this rather than unconditionally armed undo. | Does not prove ownership of current final name; a possibly-published error can coexist with third-state replacement. |
| `publish_new_in` (`file/create_new.rs:63`) | True atomic create-no-replace via staged hard-link publication. Can restore old bytes to an absent name without clobbering an occupant. | Can leave two-link stage residue on cleanup failure; result lacks held published identity/mode option. This is creation, not conditional replacement of an existing after-image. |
| `Pinned::inspect_child_state`, `remove_child_expected`, `rename_child_noreplace_to[_durable]` (`transaction/operations.rs:43,55,118`) | Existing expected state includes kind/content/mode/object identity; native operation checks source handle and preserves no-replace destination semantics. | Expected remove/rename execution is **Windows-only**; non-Windows returns Unsupported (`transaction/platform/operations.rs:225`; rename platform branches). Rename guards source and absent destination, not an atomic compare-and-replace of an occupied destination. These APIs reject shared hardlinks. |
| `Project::remove_file[_in]` (`file/project_methods.rs:495`) | No-follow/single-link admission. | Opens/checks and then unlinks by name; it is not expected-identity deletion and cannot substitute for `remove_child_expected`. |

Important hardlink distinction: existing mutation APIs deliberately reject `nlink != 1` (`file.rs:183`; atomic destination admission:70). They preserve other links by **refusing**, not by supporting shared-file replacement. If root requires shared manifest/lock acceptance, atomic replacement should detach only the named link and leave siblings unchanged; truncating the old held inode is wrong. That requires a narrowly chosen publication policy/API change, not weakening all mutation single-link rules or treating the read-only source reader as write ownership. Restoring bytes by atomic replacement also does not restore original inode/link topology; any such promise would require preserving the old object, a separate decision.

Inspected existing test bodies support bounded facts: `file/bounded_tests.rs:28::stable_snapshot_binds_bytes_state_and_identity_to_one_epoch` checks bytes/identity/cap; `file/create_new_tests.rs:89::a_hard_linked_destination_refuses_and_keeps_both_names` checks refusal and both names unchanged. They do not establish a portable occupied-file rollback CAS.

## Minimal decision boundary for root

A finite in-memory guard can own only these three files, captured parents/prior state, exact intended operation and a publication receipt. It should stay idle on BeforePublication, restore only a still-owned after-image, leave third-state bytes/object/links untouched and surface a rollback conflict alongside the original failure. There is no need for whole-tree census, prototype slot copying, durable recovery ledger or additional store.

However, **“compare then ordinary rename/unlink” is not a race-free conditional mutation**. It fixes the deterministic C-before-drop counterexample but leaves a final compare→syscall window. Existing public SafeFS APIs cannot honestly close that window for occupied-file replacement on every supported platform. Root must freeze the desired threat/compatibility boundary before implementation: bounded cooperative in-process ownership check with its remaining window stated, or a small shared platform-specific expected-state publication primitive with unsupported cases explicit. Do not silently advertise portable hostile-race safety from a read check plus `write_atomic`.

Suggested finite discriminators: unchanged B rolls back to A; before-absent B is removed; before-absent third C remains; same inode changed bytes C remains; different inode same B remains; symlink/special-file substitution untouched; failed publication before any write performs no undo; empty provenance removal can restore A only when name remains absent; ordinary no-third-state rollback still restores manifest/lock/provenance; shared-link sibling unchanged or explicit pre-effect refusal; independent selected-manifest/workspace-lock parents stay bound. First test should be the actual deferred-source apply seam above, with ordinary CLI rollback as control. Add final-window tests only for guarantees the chosen mutation API truly provides.
