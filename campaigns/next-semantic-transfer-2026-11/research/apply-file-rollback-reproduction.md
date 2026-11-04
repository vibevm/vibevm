# Actual apply file-rollback ownership reproduction

**Observed: 5 tests executed, 1 control passed, 4 intended ownership invariants failed; cargo exit101.** This is an actual public `vibe_install::plan` → `vibe_install::apply` run with the existing deferred `InstallSource::materialise_in_place` seam. No reimplementation or direct invocation of private FileGuard. Production remains unchanged; this is a retained TEST-ONLY RED candidate, not an accepted patch or selected fix.

Worktree `/fast/git/wt/apply-file-rollback`; branch `fix/apply-file-rollback`; base/HEAD `77b645ff77eba99150edf95d4f990626549603ae`. Own target is a real worktree-local directory, not a symlink. Fixture is Unix-scoped; this execution is Linux. No external source connection is made: solve/manifest are supplied in process and unexpected fetch panics. All paths and bytes under test are synthetic temporary fixtures.

## Results

| Actual apply case | Interleaving before ordinary source error | Observed after failed apply | Verdict |
|---|---|---|---|
| ordinary_failed_apply_restores_original_manifest | No third actor; keep apply's B | Original A restored; lock unchanged | PASS |
| failed_apply_preserves_same_inode_third_state_edit | Change B to valid distinct C in the same inode | C overwritten by A; same inode remains | Intended RED |
| failed_apply_preserves_same_bytes_new_object | Atomically rename a separately created file containing identical B over manifest; assert inode differs from apply's B | Replacement inode remains, but its B bytes overwritten by A | Intended RED |
| failed_apply_does_not_follow_substituted_symlink | Replace manifest leaf with symlink to synthetic sibling containing C | Symlink remains; its target/sibling overwritten by A | Intended RED |
| failed_apply_does_not_truncate_substituted_hardlink_sibling | Replace manifest leaf with hardlink to synthetic sibling containing C; assert shared dev/inode and nlink2 | Both names still share inode/nlink2; sibling and manifest bytes overwritten by A | Intended RED |

Every case first asserts that the callback sees **B actually published by apply**, different from A and containing the new `org.vibevm/giant` requirement; B and C parse as valid manifests. Planning calls the callback zero times; apply calls it exactly once. The callback always returns the ordinary RegistryError marker `SYNTHETIC_DEFERRED_FAILURE_AFTER_INTERLEAVING`; the public apply error contains that marker. Original lock bytes are unchanged in all five cases. Each observation is emitted before the ownership assertion fails, preserving bytes and pre/post inode/dev/link-count facts.

This proves an intervening writer's state is destroyed by unwind through the real apply route: `apply.rs:415` publishes manifest, `apply.rs:443` calls deferred materialization, `apply/deferred.rs:34` propagates the callback error, `apply/file_guard.rs:52` restores by ambient write. The callback is an intentional synthetic actor; no claim that the production Git provider normally edits the root manifest.

## Commands, freshness and mistakes

Final exact command, cwd the worktree:

```text
CARGO_TARGET_DIR=/fast/git/wt/apply-file-rollback/target cargo test --offline --locked -p vibe-install --test apply_file_rollback -- --nocapture --test-threads=1
```

Final result: exit101; 5 tests, 1 passed, 4 failed, 0 ignored, 0 filtered; command wall time1.58s. Raw `/tmp/nst-apply-file-rollback-probe.log` retains the compilation, executable path, all five JSON observations and assertion failures. `/tmp/nst-apply-file-rollback-command.json` records exact argv/environment/HEAD/test hash/time/exit. This binary was compiled by that cargo invocation from the final test source; no mtime-only provenance inference.

I made one fixture compiler mistake first: formatted `current_vibedeps_root()` (PathBuf) using Display at two path constructions. Initial cargo exit101 was **E0277 with zero tests**, not rollback evidence. I changed only the fixture helper to accept `AsRef<Path>` and used `Path::join`, then reran the same target. Original failed compiler log/command/checks remain as `/tmp/nst-apply-file-rollback-compiler-attempt-{probe.log,command.json,checks.json}`; its test SHA is in the initial command. No fixture runtime setup failure occurred after that correction.

Final `rustfmt --edition 2024 --check crates/vibe-install/tests/apply_file_rollback.rs` and `git diff --check` both exit0 (`...-checks.json`). Tracked source diff is empty; the complete nonignored changed set is exactly one new test file below. No unrelated suites rerun. No live cwd/executable/fd users of this worktree at freeze.

## Frozen artifacts

- Test: `crates/vibe-install/tests/apply_file_rollback.rs`; SHA256 `10930656391da9059a5d245977e353de6270ec60ec5dbde93449820db0ce7d75`.
- Executable: `/fast/git/wt/apply-file-rollback/target/debug/deps/apply_file_rollback-898e3dd8cd79bf1c`; SHA256 `f2d6a0c6ffa556786ca9aa4d53c0a958b0e7319d8b1d5445a35f22c32ffa951a`.
- `/tmp/nst-apply-file-rollback-observations.json`: all five structured observations, including actual synthetic bytes and OS object/link evidence.
- `/tmp/nst-apply-file-rollback-evidence.json`: complete changed-file and relevant source SHA256 values, executable hash, final and failed-compiler commands, artifact hashes, checks, counts and empty live-user census.
- `/tmp/nst-apply-file-rollback-files.json`: complete one-file candidate SHA256 manifest.

## Scope limits and next owner decision

This is manifest rollback evidence only. Lock/provenance partial-capture/write failures and prior-absence behavior remain **source-only findings** from `/tmp/nst-apply-file-rollback-review.md`; they were not exercised here. The same-byte replacement case checks both bytes and object identity, not only string equality. Hardlink substitution happens **after** B publication; this does not settle initially-hardlinked forward-writer acceptance/refusal policy. No helper-only observation is presented as an actual-apply test.

No crash recovery, hostile final compare→mutation race, Windows/macOS execution, whole-slot rollback, all-file atomicity, or SafeFS CAS capability is established. No publication/platform fix chosen; root retains NEED/design/acceptance, archival and eventual worktree cleanup. Test file and raw RED evidence are frozen pending that decision.
