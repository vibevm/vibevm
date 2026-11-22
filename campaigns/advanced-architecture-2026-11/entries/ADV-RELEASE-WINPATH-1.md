# ADV-RELEASE-WINPATH-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** When a fresh dependency slot would need a path longer than the host's limit, `vibe install` stops before it writes any slot and prints `path too long: `<path>` (<n> characters; this host's limit is <m>)` with the fix `move the project to a shorter root`; a Linux unit test simulates the limit.

**Write.** `crates/vibe-workspace/src/install/path_budget.rs`, `crates/vibe-workspace/src/install/tests_path_budget.rs`, `crates/vibe-workspace/src/install.rs`, `crates/vibe-workspace/src/errors.rs`

**Check.** `cargo test -p vibe-workspace --lib install::tests_path_budget:: 2>&1 | grep -F "test result: ok. 2 passed"`

**Test.** crates/vibe-workspace/src/install/tests_path_budget.rs: `deep_fresh_slot_is_refused_before_any_slot_is_written` and `present_slot_is_not_measured`, both under the lib test path `install::tests_path_budget::`. Before this entry the filter matches 0 lib tests (checked at 9d8428d07), so the output has no `ok. 2 passed` line and the grep fails.

**Read first.**
- crates/vibe-workspace/src/install.rs: fn materialise_resolution_with_prepared, struct MaterialiseOptions, struct Materialised, fn io_err, the `#[cfg(test)] #[path = "install/..."]` test module declarations at the end of the file
- crates/vibe-workspace/src/install/tests.rs: fn observed_materialise (pattern for calling materialise_resolution_with_spec_format from a test)
- crates/vibe-workspace/src/install/test_helpers.rs: fn dep_with_boot, fn ver
- crates/vibe-workspace/src/errors.rs: enum WorkspaceError, variant Io and the shape of its neighbours' #[error] messages
- crates/vibe-workspace/src/vibedeps.rs: fn slot_abs_path, fn is_materialised
- crates/vibe-workspace/src/vibedeps/slot_diff.rs: fn staged_file (the `.vibe-place-` staging file beside each payload)

**Steps.**
1. errors.rs: add this variant to `pub enum WorkspaceError` directly after the `Io { path: PathBuf, reason: String }` variant:
```
    /// A fresh dependency slot would need a path longer than this host can
    /// publish. Raised before any slot is written.
    #[error(
        "path too long: `{}` ({length} characters; this host's limit is {limit}) \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#TREE-VIBEDEPS; \
         fix: move the project to a shorter root)",
        .path.display()
    )]
    PathTooLong {
        path: PathBuf,
        length: usize,
        limit: usize,
    },
```
No other crate matches `WorkspaceError` exhaustively, so nothing else changes. Confirm with `grep -rn "WorkspaceError::Io" crates --include=*.rs` that every hit is a constructor or a single-arm match, not an exhaustive match.
2. Create crates/vibe-workspace/src/install/path_budget.rs. First lines, in this order: `//! The path-length budget of fresh dependency slots.`, a blank line, then `specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#TREE-VIBEDEPS");`, then these imports exactly:
```
use std::path::Path;

use super::ResolvedDep;
use crate::{WorkspaceError, vibedeps::slot_abs_path};
```
`super::is_in_place` and `super::io_err` are reached by path, not imported. Contents:
(a) `const WINDOWS_SLOT_PATH_LIMIT: usize = 240;` with this doc comment (exact wording):
```
/// MAX_PATH leaves 259 usable UTF-16 units. tempfile's Windows persist
/// (MoveFileExW / SetFileAttributesW on the path without the `\\?\` prefix)
/// runs on a staging file beside each payload, named `.vibe-place-` plus 6
/// random characters (18 units; vibedeps/slot_diff.rs fn staged_file). The
/// 19-unit `.vibe-record-` name at the slot root is the longest staging name,
/// so 259 - 19 = 240 is a conservative bound on a payload destination.
```
(b) A test override:
```
#[cfg(test)]
thread_local! {
    static TEST_LIMIT: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(super) fn set_test_limit(limit: Option<usize>) {
    TEST_LIMIT.with(|cell| cell.set(limit));
}
```
`None` in the override means 'use the host default', never 'no limit'.
(c) `pub(super) fn host_limit() -> Option<usize>`. Under `#[cfg(test)]` it first returns `Some(l)` when TEST_LIMIT holds `Some(l)`. Otherwise it returns `if cfg!(windows) { Some(WINDOWS_SLOT_PATH_LIMIT) } else { None }`.
(d) `pub(super) fn check_fresh_slots(workspace_root: &Path, resolution: &[ResolvedDep], limit: Option<usize>) -> Result<(), WorkspaceError>`. With `None` it returns Ok(()). Otherwise it skips each dep for which any of these holds: `super::is_in_place(dep)`, `crate::vibedeps::is_materialised(workspace_root, &dep.group, &dep.name, &dep.version)`, or `!dep.content_dir.is_dir()`. For every other dep it walks `dep.content_dir` recursively with `std::fs::read_dir`, entries sorted by file name. The walk skips entries named `.git` and entries whose `file_type()` is a symlink. For each regular file at relative path `rel` it measures `dest = slot_abs_path(workspace_root, &dep.group, &dep.name, &dep.version).join(rel)`: units = `dest.as_os_str().to_string_lossy().encode_utf16().count()`, plus 1 when the extension is `md`. It keeps the single longest (path, units) over the whole resolution; on a tie the first one found stays. If that length is greater than `limit`, it returns `Err(WorkspaceError::PathTooLong { path, length, limit })`. read_dir, entry and file_type errors map through `super::io_err(path, error)` with the directory or entry path being read.
3. install.rs: add `mod path_budget;` on the line after `use placement::{copy_mode_for, is_in_place, required_source_hash};`. In fn materialise_resolution_with_prepared, directly after the line `placement::validate_prepared(workspace_root, resolution, prepared)?;`, add `path_budget::check_fresh_slots(workspace_root, resolution, path_budget::host_limit())?;`. At the end of the file, after the last existing `#[cfg(test)] #[path = ...]` module, add
```
#[cfg(test)]
#[path = "install/tests_path_budget.rs"]
mod tests_path_budget;
```
4. Create crates/vibe-workspace/src/install/tests_path_budget.rs starting with
```
use super::test_helpers::*;
use super::*;
use tempfile::TempDir;
```
and a helper `fn materialise_all(root: &Path, resolution: &[ResolvedDep]) -> Result<Materialised, WorkspaceError>` that calls `materialise_resolution_with_spec_format(root, resolution, MaterialiseOptions { slot_integrity: SlotIntegrity::TrustPresence, spec_format: SpecFormat::Mixed, slot_verifier: None, lifecycle: SlotLifecycleMode::None, progress: vibe_core::progress::Progress::default(), rollback: None })`.
Both tests call `path_budget::set_test_limit(Some(..))` explicitly before every materialise call; neither test passes `None`, so a Windows run never falls back to the 240 default.
Test `deep_fresh_slot_is_refused_before_any_slot_is_written`, statements in this order:
1. `let root = TempDir::new().unwrap();`
2. `let (short, _a) = dep_with_boot("short", "1.0.0", "", "boot/a.md", "# a");`
3. `let deep_rel = format!("spec/{}/{}/leaf.md", "d".repeat(60), "e".repeat(60));` then `let (deep, _b) = dep_with_boot("deep", "1.0.0", "", &deep_rel, "# deep");`
4. Compute the limit BEFORE `short` moves into the array: `let limit = vibedeps::slot_abs_path(root.path(), &short.group, "short", &ver("1.0.0")).as_os_str().to_string_lossy().encode_utf16().count() + 40;`
5. `path_budget::set_test_limit(Some(limit));` then `let result = materialise_all(root.path(), &[short, deep]);`
6. Assert `matches!(&result, Err(WorkspaceError::PathTooLong { path, length, limit: l }) if *l == limit && *length > limit && path.ends_with("leaf.md"))`. Assert `result.unwrap_err().to_string().starts_with("path too long: ")` (take the error once; use `let error = result.unwrap_err();` after the matches assert if needed). Assert `!root.path().join(vibe_core::layout::current_vibedeps_root()).exists()`.
Test `present_slot_is_not_measured`:
1. `let root = TempDir::new().unwrap();`
2. Build the same deep dep as in test 1 (same `deep_rel`).
3. `path_budget::set_test_limit(Some(usize::MAX));` then assert `materialise_all(root.path(), std::slice::from_ref(&deep))` is Ok.
4. `path_budget::set_test_limit(Some(10));` then `let second = materialise_all(root.path(), std::slice::from_ref(&deep)).unwrap();` and `assert_eq!(second.skipped.len(), 1);`.
5. Run `cargo fmt --all`, then `cargo clippy -p vibe-workspace --all-targets -- -D warnings`. TEST_LIMIT and set_test_limit are `#[cfg(test)]` only and used by the test module, so no dead-code warning. WINDOWS_SLOT_PATH_LIMIT is referenced inside `cfg!(windows)` (an expression, not an attribute), so a Linux build compiles the reference and reports no dead code. Do not add `#[allow]` attributes; if clippy flags something else, fix the code. Run `wc -l` on install.rs (459 lines at planning, about 465 after), errors.rs (408 at planning, about 422 after) and the two new files; every one must be at or below 600.
6. Run the check.

**Decisions already made.**
- The guard measures only fresh slots: deps that are not in-place and whose slot directory does not exist yet. A slot that is already present, for example a vibedeps tree a teammate committed and cloned with core.longpaths, is never measured, so a trusted no-op install keeps working.
- It runs before the materialisation loop, in fn materialise_resolution_with_prepared, right after placement::validate_prepared. A materialisation error with no prepared placements returns without an undo, so a check inside the loop would leave earlier slots behind.
- The Windows limit is 240 UTF-16 units for a payload destination: MAX_PATH's 259 usable units minus the 19-unit `.vibe-record-` staging name, the longest staging name vibe uses (the `.vibe-place-` file beside a payload is 18 units). Other platforms have no limit (None). This entry does not read the LongPathsEnabled setting and adds no longPathAware manifest, so on Windows the limit applies whatever the registry says.
- The fix text is exactly `move the project to a shorter root`. It does not mention enabling long paths, because this entry's guard and tempfile's raw Win32 calls would still refuse such a path. Advice about long paths belongs to the later entry that makes deep roots work (B-132).
- The measurement is `to_string_lossy().encode_utf16().count()` of the joined destination, plus 1 unit for `.md` files. The +1 is applied whatever the spec_format, as a conservative margin for the `.md` to `.xml` conversion; do not pass spec_format into check_fresh_slots. The error reports the single longest destination over the whole resolution, so the user sees how much shorter the root must be.
- The message is exactly `path too long: `<path>` (<n> characters; this host's limit is <m>)` followed by the spec edge and the fix. The spec edge is the existing anchor PROP-009#TREE-VIBEDEPS (already impl/done); no spec file changes.
- The test simulates the limit with a `#[cfg(test)]` thread-local override, set explicitly before every materialise call. `Some(usize::MAX)` means unlimited; `None` means the host default. Production code reads no environment variable for it.
- Native Windows confirmation is not part of this entry and not part of its check. The planner runs it afterwards over SSH. The executor and the lead do nothing on Windows.

**Not in this entry.** Making deep-root installs succeed on Windows is not in this entry: no replacement of tempfile persist with std::fs::rename, no long-path detection (RtlAreLongPathsEnabled), no longPathAware manifest or build.rs in vibe-cli, no change to boot_artifacts, slot_record, slot_diff, slot_cow or vibe-registry (B-132). Not in this entry either: measuring in-place slots or slots that are already present, threading spec_format into the guard, and an env var or CLI flag for the limit.

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
