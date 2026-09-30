//! Test-only all-assets-ready dispatch seam. No GitHub client implements this
//! port, and no CLI branch calls it. It proves the complete captured subject
//! and pure effect plan are checked before the first fake remote method.

use anyhow::{Result, ensure};
use vibe_publish::publication_effect_plan::{
    PublicationEffectPlanExpectations, publication_effect_plan_digest,
    require_apply_ready_effect_plan,
};
use vibe_publish::publication_subject::publication_subject_digest;
use vibe_wire::generated::publication_effect_plan::{
    PublicationPartition, PublicationPartitionCleanup, PublicationPartitionFinalAssets,
    PublicationPartitionFinalize, PublicationPartitionPlatformAssets, PublicationPartitionRelease,
    PublicationPartitionTag,
};

use crate::dist::capture::PreparedPublicationComposition;

/// Only the disposable test host implements this private port. A live
/// adapter would additionally need C.1 journaling, approval provenance,
/// per-partition reobservation and partial-result handling.
trait OfflineReleaseHost {
    fn tag(&mut self, value: &PublicationPartitionTag) -> Result<()>;
    fn release(&mut self, value: &PublicationPartitionRelease) -> Result<()>;
    fn platform_assets(&mut self, value: &PublicationPartitionPlatformAssets) -> Result<()>;
    fn cleanup(&mut self, value: &PublicationPartitionCleanup) -> Result<()>;
    fn final_assets(&mut self, value: &PublicationPartitionFinalAssets) -> Result<()>;
    fn finalize(&mut self, value: &PublicationPartitionFinalize) -> Result<()>;
}

fn drive_offline_ready(
    composition: &PreparedPublicationComposition,
    expected: &PublicationEffectPlanExpectations<'_>,
    host: &mut dyn OfflineReleaseHost,
) -> Result<()> {
    let capture = &composition.capture;
    let subject = &capture.subject;
    let plan = &composition.plan;
    ensure!(
        capture.identity.commit == subject.source.commit
            && capture.identity.tree == subject.source.tree,
        "captured source identity differs from publication subject"
    );
    let subject_digest = publication_subject_digest(subject, &expected.subject)?;
    ensure!(
        capture.subject_digest == subject_digest,
        "captured subject digest differs from complete subject"
    );
    let plan_digest = publication_effect_plan_digest(plan, subject, expected)?;
    ensure!(
        composition.plan_digest == plan_digest,
        "captured effect plan digest differs from validated plan"
    );
    require_apply_ready_effect_plan(plan, subject, expected)?;

    for partition in &plan.partitions {
        match partition {
            PublicationPartition::Tag(value) => host.tag(value)?,
            PublicationPartition::Release(value) => host.release(value)?,
            PublicationPartition::PlatformAssets(value) => host.platform_assets(value)?,
            PublicationPartition::Cleanup(value) => host.cleanup(value)?,
            PublicationPartition::FinalAssets(value) => host.final_assets(value)?,
            PublicationPartition::Finalize(value) => host.finalize(value)?,
        }
    }
    Ok(())
}

#[path = "offline_ready/tests.rs"]
mod tests;
