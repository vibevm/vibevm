//! Pure semantic validation and framed identity of one remote effect plan.
//!
//! Remote observations are explicit inputs. This cell neither observes a
//! release host nor grants permission to create a tag, replace a draft or
//! change visibility.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#PUBLICATION-EFFECT-PLAN");

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};
use vibe_wire::generated::publication_effect_plan::{
    PublicationAssetEffect, PublicationEffectPlan, PublicationObservedAsset,
    PublicationObservedLatestRelease, PublicationObservedRelease, PublicationObservedTag,
    PublicationPartition, PublicationPartitionCleanup, PublicationPartitionFinalAssets,
    PublicationPartitionFinalize, PublicationPartitionFinalizeAction,
    PublicationPartitionFinalizeMakeLatest, PublicationPartitionPlatformAssets,
    PublicationPartitionRelease, PublicationPartitionReleaseAction, PublicationPartitionTag,
    PublicationPartitionTagAction, PublicationPriorAsset, PublicationPriorAssetAbsent,
    PublicationRemoteTarget,
};
use vibe_wire::generated::publication_subject::{PublicationAsset, PublicationSubject};

use crate::publication_subject::{
    PublicationSubjectExpectations, publication_subject_digest, require_apply_ready_default_smoke,
    validate_publication_subject,
};
use crate::release_manifest::{
    DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME, SUPPORTED_DISTRIBUTION_TARGETS,
};

/// Explicit owner selection, independent of an authored plan body.
pub struct PublicationEffectPlanExpectations<'a> {
    pub subject: PublicationSubjectExpectations<'a>,
    pub host: &'a str,
    pub owner: &'a str,
    pub repository: &'a str,
    /// `true` is a requested visibility change, not an authorization grant.
    pub publish: bool,
    /// Owner-selected willingness to plan replacement of a known published
    /// mutable 1.1.0 release. The plan is still data; Mode A approval is
    /// separate and required before any remote deletion.
    pub allow_replace_published: bool,
}

/// Complete remote evidence captured by its owner before any plan mutation.
/// This is supplied data, not a network-reading capability or an effect grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedPublicationRemote {
    pub generation_ref: String,
    pub tag: PublicationObservedTag,
    pub release: PublicationObservedRelease,
    /// Independently read latest published full release, or explicit 404.
    /// Unknown/unavailable latest must not be represented as Absent.
    pub observed_latest_release: PublicationObservedLatestRelease,
}

/// Pure construction result. Its digest is identity data, not authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPublicationEffectPlan {
    pub plan: PublicationEffectPlan,
    pub digest: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PublicationEffectPlanError {
    #[error("publication subject is invalid: {0}")]
    Subject(String),
    #[error("publication effect plan `{field}` is invalid: {reason}")]
    Invalid { field: String, reason: String },
}

fn invalid(field: impl Into<String>, reason: impl Into<String>) -> PublicationEffectPlanError {
    PublicationEffectPlanError::Invalid {
        field: field.into(),
        reason: reason.into(),
    }
}

fn nonblank(field: &str, value: &str) -> Result<(), PublicationEffectPlanError> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(invalid(field, "blank, padded or control-bearing text"));
    }
    Ok(())
}

fn draft_name(version: &str) -> String {
    format!("vibevm {version}")
}

fn draft_body(version: &str) -> String {
    format!(
        "Mutable vibevm {version} distribution. Verify downloads through \
         `{DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME}`."
    )
}

fn bounded_release_text(
    field: &str,
    value: &str,
    cap: usize,
) -> Result<(), PublicationEffectPlanError> {
    if value.len() > cap {
        return Err(invalid(
            field,
            "release metadata exceeds bounded byte limit",
        ));
    }
    Ok(())
}

fn lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn sha256(field: &str, value: &str) -> Result<(), PublicationEffectPlanError> {
    if !value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && lower_hex(hex))
    {
        return Err(invalid(field, "expected sha256:<64 lowercase hex>"));
    }
    Ok(())
}

fn decimal(field: &str, value: &str, positive: bool) -> Result<u64, PublicationEffectPlanError> {
    let number = value
        .parse::<u64>()
        .map_err(|_| invalid(field, "expected canonical decimal u64"))?;
    if (positive && number == 0) || number.to_string() != value {
        return Err(invalid(field, "zero ID/size or noncanonical decimal u64"));
    }
    Ok(number)
}

fn observed_asset(
    field: &str,
    asset: &PublicationObservedAsset,
) -> Result<(), PublicationEffectPlanError> {
    nonblank(&format!("{field}.name"), &asset.name)?;
    decimal(&format!("{field}.asset_id"), &asset.asset_id, true)?;
    decimal(&format!("{field}.size_bytes"), &asset.size_bytes, true)?;
    sha256(&format!("{field}.sha256"), &asset.sha256)
}

fn desired_asset(
    field: &str,
    effect: &PublicationAssetEffect,
    expected: &PublicationAsset,
) -> Result<(), PublicationEffectPlanError> {
    if effect.name != expected.name
        || effect.media_type != expected.media_type
        || effect.desired_size_bytes != expected.size_bytes
        || effect.desired_sha256 != expected.sha256
    {
        return Err(invalid(
            field,
            "desired asset differs from the validated local subject",
        ));
    }
    decimal(
        &format!("{field}.desired_size_bytes"),
        &effect.desired_size_bytes,
        true,
    )?;
    sha256(&format!("{field}.desired_sha256"), &effect.desired_sha256)?;
    match effect.prior {
        PublicationPriorAsset::Absent(_) => Ok(()),
        PublicationPriorAsset::Present(_) => Err(invalid(
            field,
            "old draft asset IDs are deleted by release replacement and cannot be prior occupants of the new draft",
        )),
    }
}

fn validate_remote_observation(
    subject: &PublicationSubject,
    allow_replace_published: bool,
    generation_ref: &str,
    observed_tag: &PublicationObservedTag,
    observed_release: &PublicationObservedRelease,
    observed_latest_release: &PublicationObservedLatestRelease,
) -> Result<(), PublicationEffectPlanError> {
    nonblank("remote_generation_ref", generation_ref)?;
    let tag = format!("v{}", subject.version);
    match observed_tag {
        PublicationObservedTag::Absent(_) => {}
        PublicationObservedTag::Present(value) => {
            if value.reference != format!("refs/tags/{tag}")
                || value.object_kind != "commit"
                || value.object_sha != subject.source.commit
            {
                return Err(invalid(
                    "observed_tag",
                    "tag is not the exact source commit ref",
                ));
            }
        }
    }
    match observed_release {
        PublicationObservedRelease::Absent(_) => {}
        PublicationObservedRelease::Present(value) => {
            decimal("observed_release.release_id", &value.release_id, true)?;
            nonblank("observed_release.generation_ref", &value.generation_ref)?;
            nonblank("observed_release.target_commitish", &value.target_commitish)?;
            bounded_release_text(
                "observed_release.target_commitish",
                &value.target_commitish,
                256,
            )?;
            if let Some(name) = &value.name {
                bounded_release_text("observed_release.name", name, 1024)?;
            }
            if let Some(body) = &value.body {
                bounded_release_text("observed_release.body", body, 256 * 1024)?;
            }
            if (!value.draft && !allow_replace_published)
                || value.tag != tag
                || !value.inventory_complete
            {
                return Err(invalid(
                    "observed_release",
                    "published replacement was not selected, or exact tag/complete inventory is absent",
                ));
            }
            let mut names = BTreeSet::new();
            let mut ids = BTreeSet::new();
            for (index, asset) in value.assets.iter().enumerate() {
                observed_asset(&format!("observed_release.assets[{index}]"), asset)?;
                if !names.insert(&asset.name) || !ids.insert(&asset.asset_id) {
                    return Err(invalid(
                        "observed_release.assets",
                        "duplicate name or remote ID",
                    ));
                }
            }
        }
    }
    match observed_latest_release {
        PublicationObservedLatestRelease::Absent(_) => {}
        PublicationObservedLatestRelease::Present(latest) => {
            decimal(
                "observed_latest_release.release_id",
                &latest.release_id,
                true,
            )?;
            nonblank("observed_latest_release.tag", &latest.tag)?;
            bounded_release_text("observed_latest_release.tag", &latest.tag, 512)?;
            if latest.tag == tag {
                return Err(invalid(
                    "observed_latest_release",
                    "the target tag is already the latest published release or remote reads disagree",
                ));
            }
            if matches!(observed_release, PublicationObservedRelease::Present(old)
                if old.release_id == latest.release_id)
            {
                return Err(invalid(
                    "observed_latest_release",
                    "delete-first replacement of the current latest release has no pre-effect fallback proof",
                ));
            }
        }
    }
    Ok(())
}

fn fresh_asset(asset: &PublicationAsset) -> PublicationAssetEffect {
    PublicationAssetEffect {
        name: asset.name.clone(),
        media_type: asset.media_type.clone(),
        desired_size_bytes: asset.size_bytes.clone(),
        desired_sha256: asset.sha256.clone(),
        // The release partition creates a NEW draft after deleting the old
        // one. Its prior asset IDs cannot become occupants of this draft.
        prior: PublicationPriorAsset::Absent(Box::new(PublicationPriorAssetAbsent {})),
    }
}

/// Construct the entire initial prepare→finalize plan from a validated local
/// subject and a complete owner-supplied remote observation. Temporary final
/// assets created after platform upload are not in this initial snapshot, so
/// cleanup remains explicitly empty until a later observation/replan exists.
pub fn plan_from_captured_remote(
    subject: &PublicationSubject,
    expected: &PublicationEffectPlanExpectations<'_>,
    captured: &CapturedPublicationRemote,
) -> Result<PreparedPublicationEffectPlan, PublicationEffectPlanError> {
    validate_publication_subject(subject, &expected.subject)
        .map_err(|error| PublicationEffectPlanError::Subject(error.to_string()))?;
    validate_remote_observation(
        subject,
        expected.allow_replace_published,
        &captured.generation_ref,
        &captured.tag,
        &captured.release,
        &captured.observed_latest_release,
    )?;
    let subject_sha256 = publication_subject_digest(subject, &expected.subject)
        .map_err(|error| PublicationEffectPlanError::Subject(error.to_string()))?;
    let tag = format!("v{}", subject.version);
    let release_ref = "created:draft-release".to_owned();
    let mut partitions = vec![
        PublicationPartition::Tag(Box::new(PublicationPartitionTag {
            action: match captured.tag {
                PublicationObservedTag::Absent(_) => PublicationPartitionTagAction::Create,
                PublicationObservedTag::Present(_) => PublicationPartitionTagAction::Retain,
            },
            tag: tag.clone(),
            desired_commit: subject.source.commit.clone(),
            recovery_ref: "tag-ref".into(),
        })),
        PublicationPartition::Release(Box::new(PublicationPartitionRelease {
            action: match captured.release {
                PublicationObservedRelease::Absent(_) => PublicationPartitionReleaseAction::Create,
                PublicationObservedRelease::Present(_) => {
                    PublicationPartitionReleaseAction::Replace
                }
            },
            desired_tag: tag,
            desired_draft: true,
            desired_target_commit: subject.source.commit.clone(),
            desired_name: draft_name(&subject.version),
            desired_body: draft_body(&subject.version),
            desired_prerelease: false,
            recovery_ref: "draft-release".into(),
        })),
    ];
    for target in &subject.targets {
        partitions.push(PublicationPartition::PlatformAssets(Box::new(
            PublicationPartitionPlatformAssets {
                target: target.target.clone(),
                target_release_ref: release_ref.clone(),
                assets: [&target.bundle, &target.bootstrap, &target.fragment]
                    .into_iter()
                    .map(fresh_asset)
                    .collect(),
                recovery_ref: format!("platform:{}", target.target),
            },
        )));
    }
    partitions.push(PublicationPartition::Cleanup(Box::new(
        PublicationPartitionCleanup {
            target_release_ref: release_ref.clone(),
            assets: Vec::new(),
            recovery_ref: "cleanup-final".into(),
        },
    )));
    partitions.push(PublicationPartition::FinalAssets(Box::new(
        PublicationPartitionFinalAssets {
            target_release_ref: release_ref.clone(),
            assets: std::iter::once(&subject.aggregate)
                .chain(subject.installers.iter().map(|installer| &installer.asset))
                .map(fresh_asset)
                .collect(),
            recovery_ref: "final-assets".into(),
        },
    )));
    partitions.push(PublicationPartition::Finalize(Box::new(
        PublicationPartitionFinalize {
            action: if expected.publish {
                PublicationPartitionFinalizeAction::Publish
            } else {
                PublicationPartitionFinalizeAction::RetainDraft
            },
            make_latest: if expected.publish {
                PublicationPartitionFinalizeMakeLatest::Legacy
            } else {
                PublicationPartitionFinalizeMakeLatest::Unchanged
            },
            target_release_ref: release_ref,
            recovery_ref: "visibility".into(),
        },
    )));
    let plan = PublicationEffectPlan {
        schema: 3,
        subject_sha256,
        target: PublicationRemoteTarget {
            host: expected.host.into(),
            owner: expected.owner.into(),
            repository: expected.repository.into(),
        },
        remote_generation_ref: captured.generation_ref.clone(),
        observed_tag: captured.tag.clone(),
        observed_release: captured.release.clone(),
        observed_latest_release: captured.observed_latest_release.clone(),
        smoke_policy_id: subject.smoke.policy_id.clone(),
        partitions,
    };
    let digest = publication_effect_plan_digest(&plan, subject, expected)?;
    Ok(PreparedPublicationEffectPlan { plan, digest })
}

/// Validate a plan against the exact already-validated local subject and a
/// separately selected destination. The observed release inventory must be
/// complete before replacing its draft because deletion implicitly removes
/// every asset in that release.
pub fn validate_publication_effect_plan(
    plan: &PublicationEffectPlan,
    subject: &PublicationSubject,
    expected: &PublicationEffectPlanExpectations<'_>,
) -> Result<(), PublicationEffectPlanError> {
    validate_publication_subject(subject, &expected.subject)
        .map_err(|error| PublicationEffectPlanError::Subject(error.to_string()))?;
    let subject_digest = publication_subject_digest(subject, &expected.subject)
        .map_err(|error| PublicationEffectPlanError::Subject(error.to_string()))?;
    if plan.schema != 3 || plan.subject_sha256 != subject_digest {
        return Err(invalid(
            "subject_sha256",
            "schema or exact subject digest differs",
        ));
    }
    for (field, actual, selected) in [
        ("target.host", plan.target.host.as_str(), expected.host),
        ("target.owner", plan.target.owner.as_str(), expected.owner),
        (
            "target.repository",
            plan.target.repository.as_str(),
            expected.repository,
        ),
    ] {
        nonblank(field, selected)?;
        if actual != selected {
            return Err(invalid(
                field,
                "plan destination differs from owner selection",
            ));
        }
    }
    if expected.host != "github.com"
        || expected.owner != "vibevm"
        || expected.repository != "vibevm"
    {
        return Err(invalid(
            "target",
            "this distribution plan supports only the selected GitHub repository",
        ));
    }
    if plan.smoke_policy_id != subject.smoke.policy_id {
        return Err(invalid(
            "smoke_policy_id",
            "plan and subject use different smoke policy",
        ));
    }
    validate_remote_observation(
        subject,
        expected.allow_replace_published,
        &plan.remote_generation_ref,
        &plan.observed_tag,
        &plan.observed_release,
        &plan.observed_latest_release,
    )?;
    let tag = format!("v{}", subject.version);
    if plan.partitions.len() != 2 + SUPPORTED_DISTRIBUTION_TARGETS.len() + 3 {
        return Err(invalid(
            "partitions",
            "expected tag, release, five platform triples, cleanup, final assets and finalize",
        ));
    }
    let PublicationPartition::Tag(tag_step) = &plan.partitions[0] else {
        return Err(invalid("partitions[0]", "tag effect must come first"));
    };
    nonblank("partitions[0].recovery_ref", &tag_step.recovery_ref)?;
    if tag_step.tag != tag
        || tag_step.desired_commit != subject.source.commit
        || !matches!(
            (&plan.observed_tag, &tag_step.action),
            (
                PublicationObservedTag::Absent(_),
                PublicationPartitionTagAction::Create
            ) | (
                PublicationObservedTag::Present(_),
                PublicationPartitionTagAction::Retain
            )
        )
    {
        return Err(invalid(
            "partitions[0]",
            "tag action or desired commit differs from observation",
        ));
    }
    let PublicationPartition::Release(release_step) = &plan.partitions[1] else {
        return Err(invalid("partitions[1]", "release effect must follow tag"));
    };
    nonblank("partitions[1].recovery_ref", &release_step.recovery_ref)?;
    if release_step.desired_tag != tag
        || !release_step.desired_draft
        || release_step.desired_target_commit != subject.source.commit
        || release_step.desired_name != draft_name(&subject.version)
        || release_step.desired_body != draft_body(&subject.version)
        || release_step.desired_prerelease
        || !matches!(
            (&plan.observed_release, &release_step.action),
            (
                PublicationObservedRelease::Absent(_),
                PublicationPartitionReleaseAction::Create
            ) | (
                PublicationObservedRelease::Present(_),
                PublicationPartitionReleaseAction::Replace
            )
        )
    {
        return Err(invalid(
            "partitions[1]",
            "release action differs from observed draft",
        ));
    }
    let target_release_ref = format!("created:{}", release_step.recovery_ref);
    let mut recovery_refs = BTreeSet::from([
        tag_step.recovery_ref.as_str(),
        release_step.recovery_ref.as_str(),
    ]);
    if recovery_refs.len() != 2 {
        return Err(invalid(
            "recovery_ref",
            "partition recovery refs must be distinct",
        ));
    }
    for (index, target) in subject.targets.iter().enumerate() {
        let PublicationPartition::PlatformAssets(step) = &plan.partitions[index + 2] else {
            return Err(invalid(
                format!("partitions[{}]", index + 2),
                "platform partition missing or out of order",
            ));
        };
        if step.target != target.target
            || step.target_release_ref != target_release_ref
            || step.assets.len() != 3
        {
            return Err(invalid(
                format!("partitions[{}]", index + 2),
                "target, release ref or asset triple differs",
            ));
        }
        nonblank("platform.recovery_ref", &step.recovery_ref)?;
        if !recovery_refs.insert(&step.recovery_ref) {
            return Err(invalid("platform.recovery_ref", "duplicate recovery ref"));
        }
        for (asset_index, (effect, wanted)) in step
            .assets
            .iter()
            .zip([&target.bundle, &target.bootstrap, &target.fragment])
            .enumerate()
        {
            desired_asset(
                &format!("platform[{index}].assets[{asset_index}]"),
                effect,
                wanted,
            )?;
        }
    }
    let cleanup_index = 2 + subject.targets.len();
    let PublicationPartition::Cleanup(cleanup) = &plan.partitions[cleanup_index] else {
        return Err(invalid(
            "cleanup",
            "cleanup partition missing or out of order",
        ));
    };
    if cleanup.target_release_ref != target_release_ref || !cleanup.assets.is_empty() {
        return Err(invalid(
            "cleanup",
            "only an explicitly empty cleanup is supported before a post-upload inventory exists",
        ));
    }
    nonblank("cleanup.recovery_ref", &cleanup.recovery_ref)?;
    if !recovery_refs.insert(&cleanup.recovery_ref) {
        return Err(invalid("cleanup.recovery_ref", "duplicate recovery ref"));
    }
    let PublicationPartition::FinalAssets(final_assets) = &plan.partitions[cleanup_index + 1]
    else {
        return Err(invalid(
            "final_assets",
            "final assets partition missing or out of order",
        ));
    };
    if final_assets.target_release_ref != target_release_ref || final_assets.assets.len() != 4 {
        return Err(invalid(
            "final_assets",
            "release ref or final asset quartet differs",
        ));
    }
    nonblank("final_assets.recovery_ref", &final_assets.recovery_ref)?;
    if !recovery_refs.insert(&final_assets.recovery_ref) {
        return Err(invalid(
            "final_assets.recovery_ref",
            "duplicate recovery ref",
        ));
    }
    for (index, (effect, wanted)) in final_assets
        .assets
        .iter()
        .zip(
            std::iter::once(&subject.aggregate)
                .chain(subject.installers.iter().map(|row| &row.asset)),
        )
        .enumerate()
    {
        desired_asset(&format!("final_assets.assets[{index}]"), effect, wanted)?;
    }
    let PublicationPartition::Finalize(finalize) = &plan.partitions[cleanup_index + 2] else {
        return Err(invalid(
            "finalize",
            "finalize partition missing or out of order",
        ));
    };
    if finalize.target_release_ref != target_release_ref
        || !matches!(
            (&finalize.action, &finalize.make_latest, expected.publish),
            (
                PublicationPartitionFinalizeAction::RetainDraft,
                PublicationPartitionFinalizeMakeLatest::Unchanged,
                false
            ) | (
                PublicationPartitionFinalizeAction::Publish,
                PublicationPartitionFinalizeMakeLatest::Legacy,
                true
            )
        )
    {
        return Err(invalid(
            "finalize",
            "visibility differs from owner-selected posture",
        ));
    }
    nonblank("finalize.recovery_ref", &finalize.recovery_ref)?;
    if !recovery_refs.insert(&finalize.recovery_ref) {
        return Err(invalid("finalize.recovery_ref", "duplicate recovery ref"));
    }
    Ok(())
}

/// Domain-separated, unambiguous recursive frames over the validated typed
/// model. Object keys are sorted explicitly; each collection emits
/// its count, every value emits its type, and canonical decimal IDs/sizes and
/// SHA-256 values are encoded as binary rather than rendered JSON text.
struct Frames(Sha256);

impl Frames {
    fn new() -> Self {
        let mut hash = Sha256::new();
        hash.update(b"vibevm/publication-effect-plan/e3\0");
        Self(hash)
    }

    fn add(&mut self, tag: &str, bytes: &[u8]) {
        self.0.update((tag.len() as u16).to_be_bytes());
        self.0.update(tag.as_bytes());
        self.0.update((bytes.len() as u64).to_be_bytes());
        self.0.update(bytes);
    }

    fn value(&mut self, tag: &str, value: &serde_json::Value) {
        match value {
            serde_json::Value::Null => self.add(tag, b"n"),
            serde_json::Value::Bool(value) => self.add(tag, &[b'b', u8::from(*value)]),
            serde_json::Value::Number(value) => {
                let number = value.as_u64().expect("validated unsigned schema integer");
                self.add(tag, &[b'i']);
                self.add(tag, &number.to_be_bytes());
            }
            serde_json::Value::String(value) => {
                let numeric = matches!(
                    tag,
                    "asset_id" | "release_id" | "size_bytes" | "desired_size_bytes"
                );
                let binary_digest = matches!(tag, "sha256" | "desired_sha256" | "subject_sha256");
                let git_oid = matches!(tag, "desired_commit" | "object_sha");
                if numeric {
                    self.add(tag, &[b'i']);
                    self.add(
                        tag,
                        &value
                            .parse::<u64>()
                            .expect("validated decimal")
                            .to_be_bytes(),
                    );
                } else if binary_digest || git_oid {
                    self.add(tag, &[b'd']);
                    let hex = if binary_digest {
                        value.strip_prefix("sha256:").expect("validated digest")
                    } else {
                        value.as_str()
                    };
                    let bytes: Vec<u8> = hex
                        .as_bytes()
                        .chunks_exact(2)
                        .map(|pair| {
                            (pair[0] as char).to_digit(16).expect("hex") as u8 * 16
                                + (pair[1] as char).to_digit(16).expect("hex") as u8
                        })
                        .collect();
                    self.add(tag, &bytes);
                } else {
                    self.add(tag, &[b's']);
                    self.add(tag, value.as_bytes());
                }
            }
            serde_json::Value::Array(values) => {
                self.add(tag, &[b'a']);
                self.add("count", &(values.len() as u64).to_be_bytes());
                for value in values {
                    self.value("item", value);
                }
            }
            serde_json::Value::Object(values) => {
                self.add(tag, &[b'o']);
                self.add("count", &(values.len() as u64).to_be_bytes());
                let mut keys: Vec<&String> = values.keys().collect();
                keys.sort_unstable();
                for key in keys {
                    self.add("key", key.as_bytes());
                    self.value(key, &values[key]);
                }
            }
        }
    }
}

/// Framed digest over the complete validated plan. This is identity data,
/// never approval or an effect grant.
pub fn publication_effect_plan_digest(
    plan: &PublicationEffectPlan,
    subject: &PublicationSubject,
    expected: &PublicationEffectPlanExpectations<'_>,
) -> Result<String, PublicationEffectPlanError> {
    validate_publication_effect_plan(plan, subject, expected)?;
    let value = serde_json::to_value(plan).map_err(|error| invalid("plan", error.to_string()))?;
    let mut frames = Frames::new();
    frames.value("plan", &value);
    Ok(format!("sha256:{:x}", frames.0.finalize()))
}

/// The default apply gate is separate from inspectable plan identity. A
/// skipped or unavailable smoke probe remains an honest subject/plan value,
/// but cannot be treated as passed without a future explicit policy rule.
pub fn require_apply_ready_effect_plan(
    plan: &PublicationEffectPlan,
    subject: &PublicationSubject,
    expected: &PublicationEffectPlanExpectations<'_>,
) -> Result<(), PublicationEffectPlanError> {
    validate_publication_effect_plan(plan, subject, expected)?;
    require_apply_ready_default_smoke(subject, &expected.subject)
        .map_err(|error| PublicationEffectPlanError::Subject(error.to_string()))
}

#[cfg(test)]
#[path = "publication_effect_plan/tests.rs"]
mod tests;
