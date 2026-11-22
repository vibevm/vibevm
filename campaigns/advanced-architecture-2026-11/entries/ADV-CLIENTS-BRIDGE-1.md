# ADV-CLIENTS-BRIDGE-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** On Unix, `vibe skill install` publishes every projected file whose bytes start with `#!` with mode 0755, and a project-scope reinstall over a copy that lost its execute bit restores it and reports `updated`.

**Write.** `crates/vibe-agent-projection/src/pkgskill/exec_mode.rs`, `crates/vibe-agent-projection/src/pkgskill/exec_mode_tests.rs`, `crates/vibe-agent-projection/src/pkgskill/snapshot.rs`, `crates/vibe-agent-projection/src/pkgskill/receipt/transaction.rs`

**Check.** `cargo test -p vibe-agent-projection --lib pkgskill::snapshot::exec_mode::tests 2>&1 | grep -F 'test result: ok. 3 passed'`

**Test.** crates/vibe-agent-projection/src/pkgskill/exec_mode_tests.rs (module path pkgskill::snapshot::exec_mode::tests), three tests: `projected_shebang_script_is_owner_executable_and_markdown_is_not`, `reinstall_restores_a_lost_execute_bit_as_an_update`, `write_snapshot_marks_only_shebang_files_executable`. Today the filter runs 0 tests, so the grep fails.

**Read first.**
- crates/vibe-agent-projection/src/pkgskill/receipt/transaction.rs: fn publish_target_cas and fn publish
- crates/vibe-agent-projection/src/pkgskill/snapshot.rs: fn write_snapshot
- crates/vibe-safefs/src/file/project_methods.rs: Project::write_atomic_in_with_mode (signature `pub fn write_atomic_in_with_mode(&self, directory: &Pinned, relative: &str, bytes: &[u8], unix_mode: Option<u32>) -> Result<Published, PublishError>`)
- crates/vibe-safefs/src/file/source_read.rs: Project::read_source_observation_in (`pub fn read_source_observation_in(&self, directory: &Pinned, relative: &str) -> Result<Option<SourceObservation>>`) and SourceObservation::permissions (`pub fn permissions(&self) -> &std::fs::Permissions`)
- crates/vibe-safefs/src/file/stable.rs: set_unix_mode (the non-unix version returns an Unsupported error when given Some(mode))
- crates/vibe-agent-projection/src/pkgskill/projection/tests.rs: fn project_with_skill and test project_orchestrator_is_idempotent_preserves_neighbor_and_cleans_owned_stale_files (fixture and call pattern to copy)

**Steps.**
1. Create crates/vibe-agent-projection/src/pkgskill/exec_mode.rs. First line is a `//!` doc: `Projected files whose bytes start with `#!` are published owner-executable (0755) on Unix; elsewhere the platform default applies.` Then `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill");`, `use std::path::Path;`, `use vibe_safefs::{Pinned, Project};`. Add three pairs of functions. Each pair is two separate fns, one gated `#[cfg(unix)]` and one gated `#[cfg(not(unix))]`. Never write one fn with cfg blocks inside; that keeps clippy needless_return quiet. (a) `pub(crate) fn projected_mode(bytes: &[u8]) -> Option<u32>`: the unix version returns `bytes.starts_with(b"#!").then_some(0o755)`; the non-unix version does `let _ = bytes;` and returns `None`. (b) `pub(crate) fn mark_executable(path: &Path, bytes: &[u8]) -> std::io::Result<()>`: the unix version, when `projected_mode(bytes)` is `Some(mode)`, calls `std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))`, with `use std::os::unix::fs::PermissionsExt;` inside the fn, and otherwise returns `Ok(())`; the non-unix version does `let _ = (path, bytes);` and returns `Ok(())`. (c) `pub(crate) fn needs_execute_heal(project: &Project, directory: &Pinned, relative: &str, bytes: &[u8]) -> anyhow::Result<bool>`: the unix version returns `Ok(false)` when `projected_mode(bytes)` is `None`, otherwise `Ok(project.read_source_observation_in(directory, relative)?.is_some_and(|observed| observed.permissions().mode() & 0o100 == 0))`, with PermissionsExt imported inside the fn; the non-unix version does `let _ = (project, directory, relative, bytes);` and returns `Ok(false)`. End the file with exactly:
```
#[cfg(all(test, unix))]
#[path = "exec_mode_tests.rs"]
mod tests;
```
2. In crates/vibe-agent-projection/src/pkgskill/snapshot.rs add these lines directly below the `use super::*;` line:
```
#[path = "exec_mode.rs"]
pub(super) mod exec_mode;
```
The path resolves next to snapshot.rs, i.e. src/pkgskill/exec_mode.rs. In fn write_snapshot, after the `fs::write(&dest, bytes).map_err(...)?;` statement inside the loop, add:
```
        exec_mode::mark_executable(&dest, bytes).map_err(|source| PackageSkillError::Write {
            path: dest.clone(),
            source,
        })?;
```
3. In crates/vibe-agent-projection/src/pkgskill/receipt/transaction.rs fn publish, replace the call `.write_atomic_in(directory, &file.path, bytes)` with `.write_atomic_in_with_mode(directory, &file.path, bytes, crate::pkgskill::snapshot::exec_mode::projected_mode(bytes))`. Keep the following `.map(|_| ())` and `.map_err(vibe_safefs::PublishError::into_report)` unchanged.
4. In the same file, fn publish_target_cas, replace the match arm
```
            (Some(current), _) if current == &file.sha256 => continue,
```
with
```
            (Some(found), _) if found == &file.sha256 => {
                let bytes = current.as_deref().unwrap_or_default();
                if crate::pkgskill::snapshot::exec_mode::needs_execute_heal(
                    project, &directory, &file.path, bytes,
                )? {
                    publish(project, stage, &directory, file)?;
                    changed = true;
                }
            }
```
Here `current` is the existing local `let current = project.read_file_in(&directory, &file.path)?;` (an `Option<Vec<u8>>`); the arm binding is renamed to `found` so it no longer shadows it. Run `cargo fmt -p vibe-agent-projection`, then `wc -l` on transaction.rs: it is 570 lines before and must stay at or below 600.
5. Create crates/vibe-agent-projection/src/pkgskill/exec_mode_tests.rs with `use std::collections::BTreeMap; use std::fs; use std::os::unix::fs::PermissionsExt;` and a fixture fn `project_with_script() -> tempfile::TempDir`. It writes vibe.toml with exactly this content:
```
[package]
group = "org.example"
name = "demo"
kind = "tool"
version = "0.1.0"
authors = ["Fixture"]
license = "UPL-1.0"
description = "fixture"
keywords = ["fixture"]

[[skill]]
name = "demo"
path = "skills/demo"
agents = ["claude"]
```
It writes skills/demo/SKILL.md with bytes `demo body` and skills/demo/scripts/run.sh with the Rust string literal `"#!/bin/sh\necho ok\n"`, then sets run.sh to mode 0o644 with fs::set_permissions. Test `projected_shebang_script_is_owner_executable_and_markdown_is_not`: call `crate::pkgskill::project_declared_skills_project_scope(project.path(), &crate::pkgskill::DeclaredSkillFilter::all(), false).unwrap()`, assert `reports[0].status == "created"`, assert the mode of .claude/skills/demo/scripts/run.sh `& 0o777 == 0o755`, and assert the mode of .claude/skills/demo/SKILL.md `& 0o111 == 0`. Test `reinstall_restores_a_lost_execute_bit_as_an_update`: install once, set the projected .claude/skills/demo/scripts/run.sh to 0o644, install again and assert `reports[0].status == "updated"` and run.sh mode `& 0o777 == 0o755`, then install a third time and assert status `"unchanged"`. Test `write_snapshot_marks_only_shebang_files_executable`: build a `BTreeMap<String, Vec<u8>>` with "run.sh" -> `b"#!/bin/sh\n".to_vec()` and "SKILL.md" -> `b"body".to_vec()`, call `crate::pkgskill::snapshot::write_snapshot(&dir.path().join("t"), &map).unwrap()` on a fresh tempdir, and assert run.sh `& 0o777 == 0o755` and SKILL.md `& 0o111 == 0`.
6. Run `cargo fmt -p vibe-agent-projection`, `cargo clippy -p vibe-agent-projection --all-targets -- -D warnings`, then the check.

**Decisions already made.**
- Executable rule: a projected file is executable exactly when its bytes start with `#!`. The source file's mode is not read or propagated. Reason (verified in code): the pipeline carries files as `BTreeMap<String, Vec<u8>>` (snapshot_source, the composed-skill cache, Stage, receipts), the composed cache key hashes bytes only, and receipt rows are generated vibe-wire types (vibe_wire::generated::package_skill::e3::receipt::PackageSkillFile { path, sha256 }). Propagating source modes would need a wire-schema change plus jtd codegen. Deriving the mode from the bytes needs no receipt field, so receipts, drift checks and old receipts stay as they are.
- Mode values: exactly 0o755 for shebang files on Unix. Every other file keeps today's behaviour (`None`, the platform default after umask). On non-Unix nothing changes: projected_mode returns None, because safefs set_unix_mode errors on Some(mode) there.
- When a target's bytes already match but its owner-execute bit is missing, the file is republished with the same bytes and counts as a change, so the report status is `updated`. This is never drift and never a refusal. The republish works because both callers that reach publish_target_cas with files hold a stage containing every desired digest: mutate_subset in receipt/activation.rs (`Stage::create(&project, &staged)`, with staged starting as `files.clone()`) and reconcile in receipt/reconcile.rs. Plan::changed_keys always includes the transaction's own key, so the own binding's targets are always visited.
- Dry-run previews are unchanged: a pending mode heal still previews as `unchanged`, because the dry-run branch of mutate_subset compares receipt rows only.
- write_snapshot has two callers: standalone.rs reconcile_standalone_files and the composed-skill cache writer in projection.rs (`super::write_snapshot(&temporary.join("tree"), desired)`). Marking shebang files executable in that cache tree is intended and harmless, because the cache is compared by bytes only (`snapshot_source(&tree, &[])` equality). The embedded-source cache named in not_in_entry is vibe_registry's cache, which this entry does not touch.
- The helper module lives under snapshot.rs (`crate::pkgskill::snapshot::exec_mode`), so this entry does not edit pkgskill.rs or projection.rs.
- Ordering: this entry and ADV-CLIENTS-BRIDGE-2 both build, lint and test the crate vibe-agent-projection. They run one after another in the same worktree, this entry first; ADV-CLIENTS-BRIDGE-2 lists this entry in depends_on for that reason.
- The test module is `#[cfg(all(test, unix))]`; the check is run on Linux or macOS.
- No spec marker is added; the new module carries the existing scope `PROP-018#vibe-skill`.

**Not in this entry.** Not in this entry: healing the execute bit for user-scope standalone projections whose bytes already match (install_managed_skill_snapshot_with_dirs short-circuits on owned_projection_matches, so write_snapshot never runs); making the package-phase probe (`probe_project_skill_binding`, used by vibe-orchestrator for `vibe install`) notice a lost execute bit; adding an `executable` field to any receipt or wire schema; changing vibe_registry's embedded-source cache copy.

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
