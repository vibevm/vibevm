//! Pure comparison of observed remote objects across one planned partition.
//! Callback identity and an incremented generation are insufficient proof.

use std::collections::{BTreeMap, BTreeSet};

use vibe_wire::generated::publication_effect_plan::{
    PublicationAssetEffect, PublicationObservedAsset, PublicationObservedRelease,
    PublicationObservedReleasePresent, PublicationObservedTag, PublicationPartition,
    PublicationPartitionFinalizeAction, PublicationPartitionFinalizeMakeLatest,
    PublicationPartitionReleaseAction, PublicationPartitionTagAction, PublicationPriorAsset,
};

use super::RemoteObservation;

fn canonical_id(value: &str) -> bool {
    value
        .parse::<u64>()
        .ok()
        .is_some_and(|number| number > 0 && number.to_string() == value)
}

pub(super) fn effectful(partition: &PublicationPartition) -> bool {
    match partition {
        PublicationPartition::Tag(step) => step.action == PublicationPartitionTagAction::Create,
        PublicationPartition::Release(_) => true,
        PublicationPartition::PlatformAssets(step) => !step.assets.is_empty(),
        PublicationPartition::Cleanup(step) => !step.assets.is_empty(),
        PublicationPartition::FinalAssets(step) => !step.assets.is_empty(),
        PublicationPartition::Finalize(step) => {
            step.action == PublicationPartitionFinalizeAction::Publish
        }
    }
}

/// Closed fake capability set. In particular, an observed replacement
/// release ID does not prove deletion of the old release, and the epoch-one
/// fake has no protocol for deleting a nonempty cleanup inventory.
pub(super) fn supports(partition: &PublicationPartition) -> bool {
    match partition {
        PublicationPartition::Tag(_) => true,
        PublicationPartition::Release(step) => {
            step.action == PublicationPartitionReleaseAction::Create
        }
        PublicationPartition::PlatformAssets(step) => {
            !step.assets.is_empty()
                && step
                    .assets
                    .iter()
                    .all(|asset| matches!(&asset.prior, PublicationPriorAsset::Absent(_)))
        }
        PublicationPartition::Cleanup(step) => step.assets.is_empty(),
        PublicationPartition::FinalAssets(step) => {
            !step.assets.is_empty()
                && step
                    .assets
                    .iter()
                    .all(|asset| matches!(&asset.prior, PublicationPriorAsset::Absent(_)))
        }
        PublicationPartition::Finalize(step) => matches!(
            (&step.action, &step.make_latest),
            (
                PublicationPartitionFinalizeAction::RetainDraft,
                PublicationPartitionFinalizeMakeLatest::Unchanged
            ) | (
                PublicationPartitionFinalizeAction::Publish,
                PublicationPartitionFinalizeMakeLatest::Legacy
            )
        ),
    }
}

fn release(value: &PublicationObservedRelease) -> Option<&PublicationObservedReleasePresent> {
    match value {
        PublicationObservedRelease::Present(row) => Some(row),
        _ => None,
    }
}

fn asset_map(
    assets: &[PublicationObservedAsset],
) -> Option<BTreeMap<&str, &PublicationObservedAsset>> {
    let mut names = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for asset in assets {
        if !canonical_id(&asset.asset_id)
            || !ids.insert(asset.asset_id.as_str())
            || names.insert(asset.name.as_str(), asset).is_some()
        {
            return None;
        }
    }
    Some(names)
}

fn added_assets(
    before: &PublicationObservedReleasePresent,
    after: &PublicationObservedReleasePresent,
    wanted: &[PublicationAssetEffect],
    after_generation: &str,
) -> bool {
    if before.release_id != after.release_id
        || before.tag != after.tag
        || before.draft != after.draft
        || !after.inventory_complete
        || after.generation_ref != after_generation
    {
        return false;
    }
    let Some(prior) = asset_map(&before.assets) else {
        return false;
    };
    let Some(now) = asset_map(&after.assets) else {
        return false;
    };
    if now.len() != prior.len() + wanted.len() {
        return false;
    }
    if prior.iter().any(|(name, old)| now.get(name) != Some(old)) {
        return false;
    }
    wanted.iter().all(|asset| {
        now.get(asset.name.as_str()).is_some_and(|observed| {
            !prior.contains_key(asset.name.as_str())
                && observed.size_bytes == asset.desired_size_bytes
                && observed.sha256 == asset.desired_sha256
        })
    })
}

/// Every accepted transition is over typed independently observed objects.
/// The validated epoch-one cleanup is empty; a retained draft has no visibility
/// change. These no-op partitions may keep object state unchanged by design.
pub(super) fn verifies(
    before: &RemoteObservation,
    after: &RemoteObservation,
    partition: &PublicationPartition,
) -> bool {
    match partition {
        PublicationPartition::Tag(step) => {
            if after.release != before.release
                || after.latest_release_id != before.latest_release_id
            {
                return false;
            }
            match step.action {
                PublicationPartitionTagAction::Create => matches!(
                    (&before.tag, &after.tag),
                    (PublicationObservedTag::Absent(_), PublicationObservedTag::Present(tag))
                        if tag.reference == format!("refs/tags/{}", step.tag)
                            && tag.object_kind == "commit" && tag.object_sha == step.desired_commit
                ),
                PublicationPartitionTagAction::Retain => after.tag == before.tag,
            }
        }
        PublicationPartition::Release(step) => {
            if after.tag != before.tag || after.latest_release_id != before.latest_release_id {
                return false;
            }
            let Some(new) = release(&after.release) else {
                return false;
            };
            if !canonical_id(&new.release_id)
                || new.tag != step.desired_tag
                || new.draft != step.desired_draft
                || !new.inventory_complete
                || !new.assets.is_empty()
                || new.generation_ref != after.generation_ref
            {
                return false;
            }
            match step.action {
                PublicationPartitionReleaseAction::Create => {
                    matches!(before.release, PublicationObservedRelease::Absent(_))
                }
                PublicationPartitionReleaseAction::Replace => {
                    release(&before.release).is_some_and(|old| old.release_id != new.release_id)
                }
            }
        }
        PublicationPartition::PlatformAssets(step) => {
            if after.tag != before.tag || after.latest_release_id != before.latest_release_id {
                return false;
            }
            match (release(&before.release), release(&after.release)) {
                (Some(old), Some(new)) => {
                    added_assets(old, new, &step.assets, &after.generation_ref)
                }
                _ => false,
            }
        }
        PublicationPartition::Cleanup(step) => {
            step.assets.is_empty()
                && after.tag == before.tag
                && after.release == before.release
                && after.latest_release_id == before.latest_release_id
        }
        PublicationPartition::FinalAssets(step) => {
            if after.tag != before.tag || after.latest_release_id != before.latest_release_id {
                return false;
            }
            match (release(&before.release), release(&after.release)) {
                (Some(old), Some(new)) => {
                    added_assets(old, new, &step.assets, &after.generation_ref)
                }
                _ => false,
            }
        }
        PublicationPartition::Finalize(step) => match step.action {
            PublicationPartitionFinalizeAction::RetainDraft => {
                after.tag == before.tag
                    && after.release == before.release
                    && after.latest_release_id == before.latest_release_id
                    && step.make_latest == PublicationPartitionFinalizeMakeLatest::Unchanged
            }
            PublicationPartitionFinalizeAction::Publish => {
                let (Some(old), Some(new)) = (release(&before.release), release(&after.release))
                else {
                    return false;
                };
                after.tag == before.tag
                    && old.draft
                    && !new.draft
                    && old.release_id == new.release_id
                    && old.tag == new.tag
                    && old.assets == new.assets
                    && new.inventory_complete
                    && new.generation_ref == after.generation_ref
                    && after.latest_release_id.as_deref() == Some(new.release_id.as_str())
                    && step.make_latest == PublicationPartitionFinalizeMakeLatest::Legacy
            }
        },
    }
}
