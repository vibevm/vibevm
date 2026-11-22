# ADV-RELEASE-AARCH64-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** An installed vibe reads a DISTRIBUTIONS.json that also lists targets it does not know. It skips those entries and takes its own, and the publishing rule still refuses them.

**Write.** `crates/vibe-wire/src/behaviour/release_manifest.rs`, `crates/vibe-wire/src/behaviour/release_manifest/validation.rs`, `crates/vibe-publish/src/release_manifest/tests.rs`, `crates/vibe-cli/src/commands/vvm/bundle/selection.rs`, `vibevm/vibespecs/common/PROP-019-version-manager.xml`, `specmap.json`

**Check.** `cargo test -p vibe-publish --lib release_manifest::tests::a_consumer_skips_entries_for_targets_it_does_not_know -- --exact 2>&1 | grep -F 'test result: ok. 1 passed'`

**Test.** New `#[test] fn a_consumer_skips_entries_for_targets_it_does_not_know` in crates/vibe-publish/src/release_manifest/tests.rs. (1) Take `aggregate()` and push `fragment("riscv64gc-unknown-linux-gnu")`. `validate_for_target(SUPPORTED_DISTRIBUTION_TARGETS[0])` is Ok. `AggregateDistributionManifest::from_json_slice_for_target(&serde_json::to_vec(&that).unwrap(), SUPPORTED_DISTRIBUTION_TARGETS[0])` is Ok and keeps every platform: `platforms.len() == SUPPORTED_DISTRIBUTION_TARGETS.len() + 1`. (2) The same aggregate under the publisher `validate()` returns `Err(ReleaseManifestError::UnsupportedTarget { .. })`. (3) An aggregate whose only entry is the riscv64 fragment returns `Err(ReleaseManifestError::TargetAbsent { .. })` from `validate_for_target(SUPPORTED_DISTRIBUTION_TARGETS[0])`, and the error text contains `riscv64gc-unknown-linux-gnu`. (4) The aggregate from (3) with `product` set to `"other"` returns an error that is not `TargetAbsent`: the identity of the document is still checked when every entry is skipped.

**Read first.**
- AggregateDistributionManifest::validate_for_target and validate_entries in crates/vibe-wire/src/behaviour/release_manifest.rs
- validate_target, validate_schema and validate_identity in crates/vibe-wire/src/behaviour/release_manifest/validation.rs
- ReleaseManifestError::TargetAbsent and TargetMatrix in crates/vibe-wire/src/behaviour/release_manifest/error.rs
- fn aggregate, fn fragment and test a_consumer_accepts_an_aggregate_holding_its_own_target_alone in crates/vibe-publish/src/release_manifest/tests.rs
- the doc comment of read_aggregate in crates/vibe-cli/src/commands/vvm/bundle/selection.rs
- RELEASE-SOURCE-PARTIAL-AGGREGATE in vibevm/vibespecs/common/PROP-019-version-manager.xml

**Steps.**
1. This entry runs first. ADV-RELEASE-AARCH64-2, -3 and -5 edit the same files after it, one after another.
2. In validation.rs add `pub(super) fn retain_supported(aggregate: &AggregateDistributionManifest) -> AggregateDistributionManifest`. It clones `aggregate` and keeps only the platforms where `SUPPORTED_DISTRIBUTION_TARGETS.contains(&platform.target.as_str())`. Add `AggregateDistributionManifest` to that file's `use super::{...}` list.
3. Replace the first statement of `pub fn validate_for_target(&self, target: &str) -> Result<(), ReleaseManifestError>`, which is `self.validate_entries()?;`, and the `if !self` condition that follows it with this code:
```
        let known = retain_supported(self);
        if known.platforms.is_empty() {
            if self.platforms.is_empty() {
                return self.validate_entries();
            }
            validate_schema(self.schema_version)?;
            validate_identity(
                &self.product,
                &self.repository,
                &self.version,
                &self.tag,
                &self.source_commit,
            )?;
        } else {
            known.validate_entries()?;
        }
        if !known
```
The rest of the function stays the same. That rest is `.platforms.iter().any(|platform| platform.target == target)`, then `TargetAbsent` with `actual` built from `self.platforms`, which means all entries including the skipped ones, then `Ok(())`. Leave `validate`, `validate_entries`, `from_json_slice` and `from_json_slice_for_target` unchanged.
4. In the doc comment above validate_for_target, replace
```
    /// that also describes the other four is the published shape and
    /// satisfies this too.
```
with
```
    /// that also describes every other supported target is the published
    /// shape and satisfies this too.
```
Then replace
```
    /// can rewrite one entry can write five consistent ones just as cheaply,
```
with
```
    /// can rewrite one entry can write all of them consistently just as cheaply,
```
Then replace
```
    /// Still refused: an entry for an unsupported target, the same target
    /// twice, an aggregate with no entries, and an aggregate that does not
    /// describe `target` — that last one by name, because the operator's
    /// question is which file to copy next.
```
with
```
    /// An entry for a target this vibe does not support is skipped unread,
    /// so a release that adds a target stays installable by consumers from
    /// this version on; the publishing `validate` still refuses it.
    ///
    /// Still refused: the same supported target twice, an aggregate with no
    /// entries, and an aggregate that does not describe `target` — that last
    /// one by name, because the operator's question is which file to copy
    /// next.
```
5. In selection.rs, change only the doc comment of read_aggregate. Code stays as it is. Replace
```
/// The complete five-target matrix is the publishing rule, not a condition
/// of reading: a release drop an administrator filled by hand may hold only
/// the platforms that site runs, and nothing about the other four entries
```
with
```
/// The complete target matrix is the publishing rule, not a condition
/// of reading: a release drop an administrator filled by hand may hold only
/// the platforms that site runs, and nothing about the other entries
```
and replace
```
/// `#release-sources`, B-247). What is still refused: an unsupported
/// target, the same target twice, and an aggregate that does not describe
/// `target` — that one naming the target the drop is missing.
```
with
```
/// `#release-sources`, B-247). Entries for targets this vibe does not
/// support are skipped. What is still refused: the same target twice, and
/// an aggregate that does not describe `target` — that one naming the
/// target the drop is missing.
```
6. In the existing test a_consumer_accepts_an_aggregate_holding_its_own_target_alone, change the comment line `// Still refused: the same target twice, an unsupported target, and an` to `// Still refused: the same target twice, an aggregate holding only unknown targets, and an`. In the `unknown` block, change the expected `Err(ReleaseManifestError::UnsupportedTarget { .. })` to `Err(ReleaseManifestError::TargetAbsent { .. })`. Change nothing else in that test.
7. Add the new test.
8. Apply the spec_edit to PROP-019 with Edit.
9. Run `cargo xtask specmap` to refresh specmap.json. If it fails with `Vibe specification authority is missing`, leave specmap.json unchanged and say so in the report.
10. Run `cargo test -p vibe-publish --lib release_manifest` and `cargo test -p vibe-cli --lib commands::vvm::bundle`. Both must stay green. Then run `cargo fmt --all` and `cargo clippy -p vibe-wire -p vibe-publish -p vibe-cli --all-targets -- -D warnings`.
11. Run `wc -l crates/vibe-wire/src/behaviour/release_manifest.rs`. The result must be 600 or less; it is 545 today and this entry adds about 20 lines. If it is over 600, move the body into validation.rs as `pub(super) fn validate_for_target(aggregate: &AggregateDistributionManifest, target: &str) -> Result<(), ReleaseManifestError>` and have the method call it. validation.rs has 246 lines.

**Decisions already made.**
- Only the consumer path skips unknown entries, meaning `validate_for_target` and `from_json_slice_for_target`. The publisher `validate()` and the mirror parse `from_json_slice` still refuse them.
- A skipped entry is never checked for asset-name collisions or identity mismatches. The consumer downloads only its own entry, by name, size and digest. The document identity (schema, product, repository, version, tag, source commit) is still checked even when every entry is skipped.
- No new error variant. An aggregate that holds only unknown targets gets TargetAbsent, which names the targets it does hold.
- The text says 'consumers from this version on'. Vibes released before this entry still refuse an aggregate with an unknown target, and how those are moved over is in owner_questions.
- The spec fact keeps `status="impl/done"` because this entry implements the changed text.

**Not in this entry.** Do not add aarch64 or any other target to SUPPORTED_DISTRIBUTION_TARGETS (that is entry -2). Do not change install.sh/install.ps1, the code in selection.rs (only its doc comment), or the publisher rule. Do not add a 'warn on unknown target' message. Leave the comment in crates/vibe-cli/src/commands/vvm/bundle/tests/drops.rs that says 'five-target aggregate' as it is.

**Spec edit (apply as a step of this entry).**

In vibevm/vibespecs/common/PROP-019-version-manager.xml, inside RELEASE-SOURCE-PARTIAL-AGGREGATE, replace
```
  five-target matrix is the *publishing* rule
```
with
```
  complete target matrix is the *publishing* rule
```
and replace
```
  who could forge one could forge five consistently. The cold-start installers
  already read an aggregate this way. Still refused: an entry for an
  unsupported target, the same target twice, an aggregate with no entries, and
  an aggregate that does not describe the running host's target — that one
```
with
```
  who could forge one could forge them all consistently. The cold-start
  installers already read an aggregate this way. An entry for a target this
  vibe does not support is skipped unread, so a release that adds a target
  stays installable by consumers from this version on; vibes released before
  this rule refuse such an aggregate. Still refused: the same supported target
  twice, an aggregate with no entries, and an aggregate that does not describe
  the running host's target — that one
```
Keep status="impl/done".

Entries that write the same file run one after another, never in parallel.
