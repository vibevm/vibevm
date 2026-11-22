# ADV-FIX-BIN-NEWEST-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** When a slot holds both target/debug/<name> and target/release/<name>, DeclaredBinary::artifact() returns the one with the later mtime (tie or unreadable mtime: debug; only one present: that one; none: release path), DeclaredBinary::selected_profile() names which profile was chosen, and PROP-025 carries a DISPATCH-FRESHEST fact (impl/done) stating that rule.

**Write.** `crates/vibe-workspace/src/bins.rs`, `crates/vibe-workspace/src/bins/tests.rs`, `vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-workspace --lib bins::tests::artifact_prefers_newest_build`

**Test.** crates/vibe-workspace/src/bins/tests.rs, fn artifact_prefers_newest_build (replaces artifact_prefers_debug_over_release, which is deleted). Builds a DeclaredBinary on a tempdir slot exactly like the old test, then asserts in order: (a) no files: artifact()==release_artifact() and selected_profile()==BuildProfile::Release; (b) only debug file written: artifact()==debug_artifact(), selected_profile()==BuildProfile::Debug; (c) release file written, debug mtime set to UNIX_EPOCH+2_000_000s and release mtime to UNIX_EPOCH+1_000_000s: artifact()==debug_artifact(); (d) release mtime set to UNIX_EPOCH+3_000_000s: artifact()==release_artifact(), selected_profile()==BuildProfile::Release, selected_profile().as_str()=="release"; (e) debug mtime also set to UNIX_EPOCH+3_000_000s (tie): artifact()==debug_artifact(), selected_profile().as_str()=="debug". Before the change the filter matches zero tests (bins::tests in bins/tests.rs has 7 #[test] functions today, none with this name).

**Read first.**
- crates/vibe-workspace/src/bins.rs:151-257
- crates/vibe-workspace/src/bins/tests.rs:96-119
- crates/vibe-workspace/src/vibedeps/tests_diff.rs:66-76
- vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml:61-75

**Steps.**
1. In vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml, insert as a new line immediately after the line ending `pinned version is what runs, always.</BIN-EXEC>` (line 74) exactly the text given in this entry's `spec_edit` field (it already carries status="impl/done"). Change nothing else in that file.
2. In crates/vibe-workspace/src/bins.rs, above `impl DeclaredBinary`, add `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum BuildProfile { Debug, Release }` with a one-line doc comment on the enum and each variant, and `impl BuildProfile { pub fn as_str(self) -> &'static str }` returning "debug" / "release".
3. In `impl DeclaredBinary` add `pub fn selected_profile(&self) -> BuildProfile` with a one-line doc comment and marker `#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#DISPATCH-FRESHEST")]`: let debug = self.debug_artifact(), release = self.release_artifact(); if !debug.exists() return Release; if !release.exists() return Debug; read both with `std::fs::metadata(p).and_then(|m| m.modified())`; return Release only when both are Ok and release_mtime > debug_mtime (strict), otherwise Debug.
4. Rewrite `artifact()`'s body to `match self.selected_profile() { BuildProfile::Debug => self.debug_artifact(), BuildProfile::Release => self.release_artifact() }`; change its marker from `PROP-025#dispatch` to `PROP-025#DISPATCH-FRESHEST`; keep its doctest unchanged. Replace the seven doc lines of `artifact()` from `/// The artifact dispatch should launch: the **debug** build when one` through `/// through here, so the launched binary stays consistent across them.` (lines 220-226) with doc lines saying: 'The artifact dispatch should launch: the newest slot-resident build. When both the debug and the release build exist, the one with the later filesystem mtime wins; a tie or an unreadable mtime selects debug. When only one exists it is selected; when neither exists this is the release path (the only artifact `vibe bin build` produces). MCP-server registration, `vibe bin exec`, `vibe bin path` and `vibe bin list` all resolve through here.'
5. Replace whole doc blocks, not substrings (the old phrases wrap across `///` lines): in `release_artifact()`, replace the three lines 176-178 (`/// The slot-resident **release** artifact (PROP-025 §3) — where a` / `/// consent-gated \`cargo build --release\` lands, and the stable` / `/// fallback dispatch uses when no debug build is present.`) with `/// The slot-resident **release** artifact (PROP-025 §3) — where a` / `/// consent-gated \`cargo build --release\` lands; dispatch selects it` / `/// when it is the newer build or the only one (see [\`Self::artifact\`]).`; in `debug_artifact()`, replace the three lines 207-209 (`/// The slot-resident **debug** artifact — where a plain \`cargo build\`` / `/// (no \`--release\`) in the slot lands. Preferred over the release` / `/// build when it exists on disk (see [\`Self::artifact\`]).`) with `/// The slot-resident **debug** artifact — where a plain \`cargo build\`` / `/// (no \`--release\`) in the slot lands. Selected over the release` / `/// build when it is the newer of the two (see [\`Self::artifact\`]).`. Leave the release_artifact() doctest that follows untouched.
6. In crates/vibe-workspace/src/bins/tests.rs delete `artifact_prefers_debug_over_release` and add `artifact_prefers_newest_build` as described in `test`, importing `BuildProfile`; set mtimes through a local helper `fn set_mtime(path: &std::path::Path, secs: u64)` whose body is `std::fs::OpenOptions::new().write(true).open(path).unwrap().set_times(std::fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs))).unwrap()` (the pattern of vibedeps/tests_diff.rs:69-74).

**Decisions already made.**
- Type name `BuildProfile`, variants `Debug` and `Release`, defined in crates/vibe-workspace/src/bins.rs so it is reachable as `vibe_workspace::bins::BuildProfile`; `as_str` returns exactly "debug" / "release".
- Method name `selected_profile(&self) -> BuildProfile`; `artifact()` keeps its signature `-> PathBuf`.
- Freshness rule: filesystem mtime only (no build record); strict greater-than for release; tie, unreadable mtime on either file, or only debug present => Debug; only release present or neither present => Release.
- Spec marker id for artifact() and selected_profile(): spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#DISPATCH-FRESHEST. This entry creates that anchor itself (step 1), so the markers never point at a missing anchor.
- The DISPATCH-FRESHEST fact covers exec, path and MCP registration only; it says nothing about `vibe bin list` output (that is BIN-LIST's text, changed in ADV-FIX-BIN-NEWEST-2), so impl/done is true as soon as this entry lands.
- Callers (vibe-cli bin.rs, vibe-cli mcp/package_registration.rs and mcp/status.rs, vibe-install lifecycle.rs:479-502, vibe-orchestrator dispatch/backends.rs:211-234) are not edited; they pick up the new rule through artifact().
- Commit: `fix(bins): dispatch the newest slot build instead of any debug build` with body explaining that artifact() now compares debug/release mtimes, ties keep debug, selected_profile() exposes the choice, and PROP-025 gains DISPATCH-FRESHEST.

**Not in this entry.** Not in this entry: a `--profile debug|release` flag on `vibe bin exec`, any stderr warning, reading a build/artifact record instead of mtime, editing the BIN-LIST spec text, or editing any caller of artifact() outside vibe-workspace (including vibe bin list output, which is ADV-FIX-BIN-NEWEST-2).

**Spec edit (apply as a step of this entry).**

File vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml, inside `<dispatch title="4. The `vibe bin` family and dispatch">`, in the first `<facts ordered="false">` block, insert as a new line immediately after the line ending `</BIN-EXEC>`:
      <DISPATCH-FRESHEST fact="true" status="impl/done">Dispatch (`vibe bin exec`, `vibe bin path` and the registration of a package's MCP server) launches the newest slot-resident build: when both `target/debug/&lt;name&gt;` and `target/release/&lt;name&gt;` exist, the one with the later filesystem mtime is selected, and a tie or an unreadable mtime selects debug. When only one exists it is selected; when neither exists the release path is reported.</DISPATCH-FRESHEST>

Entries that write the same file run one after another, never in parallel.
