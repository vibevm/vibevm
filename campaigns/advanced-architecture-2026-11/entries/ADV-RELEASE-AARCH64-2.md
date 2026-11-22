# ADV-RELEASE-AARCH64-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `aarch64-unknown-linux-musl` becomes the sixth supported distribution target. Its fragment validates, it sorts last in DISTRIBUTIONS.json, a published release must include it, and kit recipes accept it.

**Write.** `crates/vibe-wire/src/behaviour/release_manifest.rs`, `crates/vibe-wire/src/behaviour/release_manifest/validation.rs`, `crates/vibe-wire/src/behaviour/release_manifest/error.rs`, `crates/vibe-publish/src/release_manifest.rs`, `crates/vibe-publish/src/release_manifest/tests.rs`, `xtask/src/dist/build/tests.rs`, `schemas/kit_recipe.schema.json`, `docs/admin/closed-network.md`, `vibevm/vibespecs/common/PROP-019-version-manager.xml`, `specmap.json`

**Check.** `cargo test -p vibe-publish --lib release_manifest::tests::aarch64_linux_musl_is_a_published_release_target -- --exact 2>&1 | grep -F 'test result: ok. 1 passed'`

**Test.** New `#[test] fn aarch64_linux_musl_is_a_published_release_target` in crates/vibe-publish/src/release_manifest/tests.rs. It asserts `SUPPORTED_DISTRIBUTION_TARGETS[5] == "aarch64-unknown-linux-musl"`, that `fragment("aarch64-unknown-linux-musl").validate()` is Ok, and that `aggregate().validate()` is Ok. It parses `aggregate().to_json_bytes().unwrap()` as serde_json::Value and checks that `["platforms"][5]["target"]` is `"aarch64-unknown-linux-musl"`. Last, it removes that platform from `aggregate()` with retain and checks that `validate()` on the result returns `Err(ReleaseManifestError::TargetMatrix { .. })`.

**Read first.**
- SUPPORTED_DISTRIBUTION_TARGETS (today `pub const SUPPORTED_DISTRIBUTION_TARGETS: [&str; 5]`) in crates/vibe-wire/src/behaviour/release_manifest.rs
- target_sort_key (`pub(super) fn target_sort_key(target: &str) -> usize`) in crates/vibe-wire/src/behaviour/release_manifest/validation.rs
- ReleaseManifestError::UnsupportedTarget and SourceArchiveMismatch messages in crates/vibe-wire/src/behaviour/release_manifest/error.rs
- test supported_target_inventory_is_closed in xtask/src/dist/build/tests.rs
- test kit_recipe_schema_and_actual_reader_match_registered_corpus in crates/vibe-cli/tests/cli_kit.rs (it checks every SUPPORTED_DISTRIBUTION_TARGETS entry against schemas/kit_recipe.schema.json; read only)
- release-production section (RELEASE-TARGETS, RELEASE-CLEAN-SNAPSHOT, RELEASE-MUTABLE) in vibevm/vibespecs/common/PROP-019-version-manager.xml

**Steps.**
1. Run this after ADV-RELEASE-AARCH64-1, not at the same time. Both edit release_manifest.rs, the publish tests, PROP-019 and specmap.json.
2. Change the constant to `pub const SUPPORTED_DISTRIBUTION_TARGETS: [&str; 6]` and append `"aarch64-unknown-linux-musl"` as the last element.
3. In target_sort_key add the arm `"aarch64-unknown-linux-musl" => 5,` before the `_` arm.
4. In error.rs, change the UnsupportedTarget fix text `build one of the five targets in SUPPORTED_DISTRIBUTION_TARGETS` to `build one of the targets in SUPPORTED_DISTRIBUTION_TARGETS`. Change the SourceArchiveMismatch fix text `build all five platform bundles` to `build every platform bundle`.
5. In crates/vibe-wire/src/behaviour/release_manifest.rs, change the module doc line `//! shared by the five independent vibevm distribution builds.` to `//! shared by the independent per-target vibevm distribution builds.` Change the doc line `/// Independent parser limit for the five-platform aggregate manifest.` to `/// Independent parser limit for the complete aggregate manifest.`
6. In crates/vibe-publish/src/release_manifest.rs, change the first line `//! Wire contracts shared by the five independent vibevm distribution builds.` to `//! Wire contracts shared by the independent per-target vibevm distribution builds.`
7. In xtask/src/dist/build/tests.rs supported_target_inventory_is_closed, change `assert_eq!(SUPPORTED_DISTRIBUTION_TARGETS.len(), 5);` to `6`. Add `assert!(SUPPORTED_DISTRIBUTION_TARGETS.contains(&"aarch64-unknown-linux-musl"));`.
8. In schemas/kit_recipe.schema.json, on the line that holds the `kit.target` enum, append `"aarch64-unknown-linux-musl"` as the last value. Without it, the cli_kit corpus test fails for the new target.
9. In docs/admin/closed-network.md, replace the line
```
`x86_64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin`.
```
with
```
`x86_64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin`,
`aarch64-unknown-linux-musl`.
```
10. Add the new test. Apply the spec_edit with Edit, then run `cargo xtask specmap`. If it fails with `Vibe specification authority is missing`, leave specmap.json unchanged and report it.
11. Run these and check they pass; they are not the entry's check: `cargo test -p vibe-publish --lib release_manifest`, `cargo test -p xtask dist::`, `cargo test -p vibe-cli --test cli_kit kit_recipe_schema_and_actual_reader_match_registered_corpus`, `cargo test -p vibe-cli --lib commands::vvm::bundle` and `cargo test -p vibe-bootstrap`. Then run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`.

**Decisions already made.**
- Only the musl ABI ships for aarch64 Linux: one static binary that runs on glibc and musl hosts, in containers and in rootfs. This is the recommendation in owner_questions.
- The new target is appended last with sort key 5. The existing five targets keep their order and keys, so existing aggregates serialize to the same bytes as before.
- formats/EPOCHS.toml has `public = false` and `break_window_open = true`, so the change to the kit_recipe schema enum needs no break note.
- Do not merge this entry into a release until the owner has answered the transition question in owner_questions. Once a release lists aarch64, every vibe released before ADV-RELEASE-AARCH64-1 refuses its DISTRIBUTIONS.json.

**Not in this entry.** Mapping the running host to its target is entry -3. install.sh is entry -4. The CI workflow, the build script and xtask native_target/resolve_target are entry -5. Do not add a px pin for aarch64 Linux to xtask/dist/px.toml, because px is optional per target. Do not add aarch64-unknown-linux-gnu or a multi-arch docker image. The generated doc 'The five-platform release aggregate' comes from schemas/distribution/e1/aggregate_distribution_manifest.jtd.json and is left as is, because changing it would require running codegen.

**Depends on.** ADV-RELEASE-AARCH64-1

**Spec edit (apply as a step of this entry).**

In vibevm/vibespecs/common/PROP-019-version-manager.xml replace
```
<release-production title="2.18 Native release production — five targets, one verified release">
```
with
```
<release-production title="2.18 Native release production — six targets, one verified release">
```
replace
```
  `x86_64-apple-darwin`, and `aarch64-apple-darwin`. Each host builds only its
```
with
```
  `x86_64-apple-darwin`, `aarch64-apple-darwin`, and
  `aarch64-unknown-linux-musl`. Each host builds only its
```
replace
```
  portability gate rejects tracked paths that cannot have one identity on all
  five targets (including
```
with
```
  portability gate rejects tracked paths that cannot have one identity on all
  supported targets (including
```
replace
```
creates one fresh draft. Four hosts may
```
with
```
creates one fresh draft. Five hosts may
```
and replace
```
  requires and remotely verifies all fifteen platform assets, publishes
```
with
```
  requires and remotely verifies all eighteen platform assets, publishes
```
Keep every status="impl/done".

Entries that write the same file run one after another, never in parallel.
