//! The unsupported portable rename: a plan-level refusal proved before any
//! stage, durable intent, or write exists.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use vibe_wire::generated::package_skill_receipt::PackageSkillBinding as ReceiptBinding;

use super::containment::{FoldKey, fold_key};

/// Receipt ownership is exact: a prior `SKILL.md` owns `SKILL.md` and
/// nothing else. A desired path that fold-collides with a differently
/// spelled prior-owned path is therefore not an update but a rename no
/// portable filesystem can express — on a case-insensitive host the two
/// spellings are one file, so publishing the new spelling would overwrite
/// the old bytes while the exactly-spelled old row stays owned, undeleted,
/// and unverifiable; on a case-sensitive host it would strand the old file.
///
/// Refuse the whole transaction here, *before* a stage, an `applying`
/// intent, or any mutation exists: every visible byte and the previous
/// receipt survive untouched, and the operator renames in two runs (drop
/// the old spelling, then introduce the new one).
pub(super) fn ensure_no_portable_rename(
    before: &[ReceiptBinding],
    after: &[ReceiptBinding],
) -> Result<()> {
    let before = before
        .iter()
        .map(|row| (row.key.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    for desired in after {
        let Some(prior) = before.get(desired.key.as_str()) else {
            continue;
        };
        for target in &desired.target {
            let Some(prior_target) = prior
                .target
                .iter()
                .find(|candidate| candidate.agent == target.agent)
            else {
                continue;
            };
            let owned = prior_target
                .file
                .iter()
                .map(|file| (fold_key(&file.path), file.path.as_str()))
                .collect::<BTreeMap<FoldKey, &str>>();
            for file in &target.file {
                if let Some(previous) = owned.get(&fold_key(&file.path))
                    && *previous != file.path.as_str()
                {
                    bail!(
                        "package skill `{}` renames owned file `{previous}` to `{}` in target \
                         `{}`; the two spellings are one file on a case-insensitive host, so \
                         this portable rename is unsupported — drop the old spelling in its \
                         own run before introducing the new one",
                        desired.key,
                        file.path,
                        target.path
                    );
                }
            }
        }
    }
    Ok(())
}
