# F06 — Metadata publication ownership and checked rollback

## Need and decision

The incumbent apply guard remembered a pathname and unconditionally restored its old bytes on failure. Actual public plan→apply reproduced one ordinary rollback control and four failures: a later byte change, a replacement object with identical bytes, a symlink substitution and a hardlink substitution were overwritten. A second public hosted-lifecycle reproduction found that the outer caller hid a rollback conflict behind a clean Delegated outcome.

The prototype's broad recovery image mechanism is not needed to correct these failures. The accepted design in research/apply-file-rollback-design.md keeps the existing slot transaction and three metadata owners: the selected manifest, workspace lock and provenance sidecar. Their parents may differ. No new recovery store or whole-slot snapshots are introduced.

## Implemented behavior

Core provides shared pure rendering from captured decoration. The apply holder admits actual mutation targets before effects, validates the selected manifest's recognized planned input, and renders exact intended bytes. An unchanged shared/read-only manifest remains a source read. Invalid UTF-8 metadata refuses explicitly.

SafeFS reuses its atomic stage/sync/rename path, retaining the actual create-new stage handle and identity before publication. Checked undo requires the observed current object, bytes, mode and link policy to match that owned after-state. Equal bytes on another object are insufficient. Never-published guards are inert; removal restoration uses no-replace creation. Conflicts preserve third states and carry the original operation error. Slot rollback order and post-durability disarm remain unchanged.

The hosted lifecycle caller now preserves rollback conflicts as SlotFailed→RollbackFailed, with reports and retained run/task references. Normal parking remains Delegated. This caller correction is necessary: a correct private guard alone did not ensure truthful public behavior.

## Compatibility and limits

No vibe.toml key or serialized schema/epoch changes. FMT-011 records the changed mutation admission, atomic replacement, mode preservation, additive Rust APIs and diagnostic behavior. Ordinary source hardlinks remain supported; actual shared-link/symlink/special mutation targets refuse. Other core writers retain their previous behavior.

This is bounded cooperative observation followed by atomic publication, not occupied-name kernel CAS. The final observation→mutation window against uncooperative writers remains. No multi-file atomicity, OS-crash recovery, original inode/link topology restoration or native Windows/macOS runtime proof is claimed.

## Evidence and acceptance

Frozen candidate: 23 files, 662 distinct selected tests/doctests green, scoped clippy and formatting green. Root reviewed production bodies and test assertions, verified the complete changed-file set and SHA256 manifest against an unchanged target source base, archived the evidence, and integrated the exact files. Candidate coverage includes 19 public apply cases, 3 direct production-guard cases, 138 SafeFS, 447 core manifest, 34 existing install, 13 held-source, one actual CLI rollback and seven doctests. Direct guard cases are explicitly not public-apply evidence.

Integrated main acceptance: actual apply19/19, direct guard3/3, ordinary CLI rollback1/1, owned API doctests4/4; fmt/diff and scoped core/SafeFS/install conform passed with0new findings (SafeFS11existing observations). Final specmap --check exited0:9066units/4755tagged items/3995edges,0suspects/0gated orphans/0unresolved host edges,43known warnings. All23 final source hashes match the reviewed candidate. The annotation/doc-only amendments corrected missing type examples and traceability; rejected wrong-scope and incomplete map outputs remain in the log. APPLY-FILE-UNDO is accepted at the explicit bounded guarantee above. Full recovery and campaign completion remain open.
