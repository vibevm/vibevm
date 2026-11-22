# ADV-FIX-CONVFB-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** A PlannedInstall marked with refuse_conversion_fallbacks() fails apply with a SpecMaterialization error that names the unconverted file. The refusal comes before vibe.lock is written, so slots, boot and manifest are rolled back.

**Write.** `crates/vibe-install/src/plan.rs`, `crates/vibe-install/src/plan/acquire.rs`, `crates/vibe-install/src/apply.rs`, `crates/vibe-install/src/apply/native_materialisation_tests.rs`, `crates/vibe-install/tests/slot_integrity_verify.rs`, `specmap.json`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-install --test slot_integrity_verify strict_conversion`

**Test.** crates/vibe-install/tests/slot_integrity_verify.rs, two new tests that reuse the file's FixtureSource, fixture_pkg, write, request and run_install_format helpers. The fixture is fixture_pkg(outer, "pkg-a", "# package A content\n") plus fixture_pkg(outer, "pkg-b", "# H {#h}\n\n@fact:DUP one\n\n@fact:DUP two\n") and a project vibe.toml as in installed_project. (1) strict_conversion_off_reports_the_row_and_installs: run_install_format(..., SpecFormat::Xml) succeeds; report.outcome.conversion_fallbacks has exactly 1 row with package "org.vibevm/pkg-b", version "1.0.0", file == format!("{}/flows/pkg-b/SPEC.md", vibe_core::machine_json_path(&vibe_core::layout::current_specs_root())), and kept_format SpecFormat::Markdown; the project has vibe.lock afterwards. (2) strict_conversion_refuses_before_the_lock_and_rolls_back: plan with vibe_install::plan_with_spec_format(..., SpecFormat::Xml, &NullObserver), take Plan::Ready(p), call p.refuse_conversion_fallbacks(), then slot_apply::apply_with_spec_format(..., SpecFormat::Xml).map_err(slot_apply::inner_failure) returns Err. The error matches vibe_install::Error::Workspace(vibe_workspace::WorkspaceError::SpecMaterialization { reason, .. }) with reason containing "SPEC.md kept as Markdown" and "--strict-conversion". project/vibe.lock does not exist; project.join(SLOT_A) and project.join(SLOT_B) do not exist; project/vibe.toml bytes equal the bytes before the apply.

**Read first.**
- crates/vibe-install/src/plan.rs:20-110
- crates/vibe-install/src/apply.rs:309-345,470-520
- crates/vibe-install/src/plan/acquire.rs:155-185
- crates/vibe-install/tests/slot_integrity_verify.rs:215-330
- crates/vibe-install/tests/support/slot_apply.rs:1-130

**Steps.**
1. plan.rs: add `pub(crate) strict_conversion: bool` to PlannedInstall with a doc comment ("`--strict-conversion`: refuse the apply when a spec document stays unconverted"). Add `impl PlannedInstall { pub fn refuse_conversion_fallbacks(&mut self) { self.strict_conversion = true; } }` with a doc comment.
2. Add `strict_conversion: false` to the PlannedInstall literals in plan/acquire.rs and in apply/native_materialisation_tests.rs (2 literals).
3. apply.rs apply_metadata_checked: add `strict_conversion` to the explicit `let PlannedInstall { ... } = planned;` destructure.
4. apply.rs: add a private fn `refuse_unconverted(strict: bool, rows: &[vibe_workspace::install::ConversionFallback]) -> Result<()>`. When strict is true and rows is non-empty, it returns Err(vibe_workspace::WorkspaceError::SpecMaterialization { path: PathBuf::from(&rows[0].file), reason: format!("{}; --strict-conversion refuses a spec document that was not converted", rows[0]) }.into()). Otherwise it returns Ok(()).
5. apply.rs: in the `let recorded = progress.checkpoint().map_err(Error::from)` chain, insert `.and_then(|()| refuse_unconverted(strict_conversion, &outcome.conversion_fallbacks))` before the and_then that calls lock::record_lock. The existing failure branch (files.undo_records + outcome.roll_back) then restores everything.
6. Write both tests, run the check, then run `cargo xtask specmap` and commit specmap.json with the change.

**Decisions already made.**
- Setter name PlannedInstall::refuse_conversion_fallbacks(&mut self). There is no new parameter on any apply function and no InstallRequest field (InstallRequest has about 59 literal sites).
- The refusal is the existing WorkspaceError::SpecMaterialization carried in vibe_install::Error::Workspace. Adding no new Error variant keeps the exhaustive matches downstream unchanged.
- The refusal sits in the recorded chain immediately before lock::record_lock, which is the existing rollback boundary, so slots, boot and the manifest guard are undone
- Only the first row is named in the error
- No tracing::warn line for rows: the typed rows on InstallOutcome are the surface, and the CLI renders them once (ADV-FIX-CONVFB-4)
- Without the setter, behaviour is unchanged (non-fatal by default)

**Not in this entry.** A CLI flag, orchestrator plumbing, InstallProgress fields, or strictness for the scoped update (materialise_subtree) and reinstall --force paths.

**Depends on.** ADV-FIX-CONVFB-1

Entries that write the same file run one after another, never in parallel.
