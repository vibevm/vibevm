use serde_json::{Value, json};
use vibe_wire::generated::publication_effect_plan::{
    PublicationEffectPlan, PublicationObservedRelease, PublicationPartition, PublicationPriorAsset,
    PublicationPriorAssetPresent,
};
use vibe_wire::generated::publication_subject::{PublicationSmokeProbeStatus, PublicationSubject};

use super::{
    CapturedPublicationRemote, PublicationEffectPlanExpectations, plan_from_captured_remote,
    publication_effect_plan_digest, require_apply_ready_effect_plan,
    validate_publication_effect_plan,
};
use crate::publication_subject::{PublicationSubjectExpectations, publication_subject_digest};
use crate::release_manifest::{
    DISTRIBUTION_PRODUCT, DISTRIBUTION_REPOSITORY, DISTRIBUTION_SOURCE_ARCHIVE_FILENAME,
};

const NEW: &str = include_str!(
    "../../../../formats/corpora/publication_effect_plan/e1/valid/new-draft-complete.json"
);
const REPLACE: &str = include_str!(
    "../../../../formats/corpora/publication_effect_plan/e1/valid/replace-known-draft.json"
);

fn subject_expected() -> PublicationSubjectExpectations<'static> {
    PublicationSubjectExpectations {
        version: "1.1.0",
        repository: DISTRIBUTION_REPOSITORY,
        smoke_policy_id: "distribution-smoke/e1",
        smoke_policy_epoch: 1,
    }
}

fn plan_expected() -> PublicationEffectPlanExpectations<'static> {
    PublicationEffectPlanExpectations {
        subject: subject_expected(),
        host: "github.com",
        owner: "vibevm",
        repository: "vibevm",
        publish: false,
        allow_replace_published: false,
    }
}

fn local_asset(
    effect: &vibe_wire::generated::publication_effect_plan::PublicationAssetEffect,
) -> Value {
    json!({
        "name": effect.name,
        "media_type": effect.media_type,
        "size_bytes": effect.desired_size_bytes,
        "sha256": effect.desired_sha256,
    })
}

fn subject_for(plan: &PublicationEffectPlan) -> PublicationSubject {
    let commit = match &plan.partitions[0] {
        PublicationPartition::Tag(value) => &value.desired_commit,
        _ => panic!("fixture starts with tag"),
    };
    let tree = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let archive_digest = format!("sha256:{}", "9".repeat(64));
    let targets: Vec<Value> = plan.partitions[2..7]
        .iter()
        .map(|partition| {
            let PublicationPartition::PlatformAssets(value) = partition else {
                panic!("fixture has five platform partitions")
            };
            let bootstrap = &value.assets[1];
            let suffix = if value.target == "x86_64-pc-windows-msvc" {
                ".exe"
            } else {
                ""
            };
            json!({
                "target": value.target,
                "bundle": local_asset(&value.assets[0]),
                "bootstrap": local_asset(bootstrap),
                "fragment": local_asset(&value.assets[2]),
                "components": [
                    {"name":"vibe", "path":format!("vibe{suffix}"),
                     "size_bytes":bootstrap.desired_size_bytes, "sha256":bootstrap.desired_sha256},
                    {"name":"vibe-index", "path":format!("vibe-index{suffix}"),
                     "size_bytes":"12", "sha256":format!("sha256:{}", "1".repeat(64))}
                ],
                "source_archive": {"path":DISTRIBUTION_SOURCE_ARCHIVE_FILENAME,
                    "size_bytes":"123", "sha256":archive_digest, "tree_oid":tree}
            })
        })
        .collect();
    let final_assets = match &plan.partitions[8] {
        PublicationPartition::FinalAssets(value) => &value.assets,
        _ => panic!("fixture has final assets"),
    };
    let installers: Vec<Value> = ["bash", "powershell", "cmd"]
        .iter()
        .enumerate()
        .map(|(index, kind)| json!({"kind":kind, "asset":local_asset(&final_assets[index + 1])}))
        .collect();
    let probes: Vec<Value> = targets
        .iter()
        .map(|row| {
            json!({"probe_id":"launch", "target":row["target"], "status":"passed",
                   "tested_sha256":row["bundle"]["sha256"],
                   "runner_ref":"fixture-runner", "evidence_ref":"fixture-evidence"})
        })
        .collect();
    serde_json::from_value(json!({
        "schema":1,
        "product":DISTRIBUTION_PRODUCT,
        "repository":DISTRIBUTION_REPOSITORY,
        "version":"1.1.0",
        "mutable":true,
        "source":{"commit":commit, "tree":tree,
                  "cargo_lock":{"name":"Cargo.lock", "media_type":"text/plain",
                                "size_bytes":"1", "sha256":format!("sha256:{}", "a".repeat(64))}},
        "targets":targets,
        "aggregate":local_asset(&final_assets[0]),
        "installers":installers,
        "smoke":{"policy_id":"distribution-smoke/e1", "policy_epoch":1, "probes":probes}
    }))
    .unwrap()
}

fn prepared(raw: &str) -> (PublicationEffectPlan, PublicationSubject) {
    let mut plan: PublicationEffectPlan = serde_json::from_str(raw).unwrap();
    let subject = subject_for(&plan);
    plan.subject_sha256 = publication_subject_digest(&subject, &subject_expected()).unwrap();
    (plan, subject)
}

fn captured(plan: &PublicationEffectPlan) -> CapturedPublicationRemote {
    CapturedPublicationRemote {
        generation_ref: plan.remote_generation_ref.clone(),
        tag: plan.observed_tag.clone(),
        release: plan.observed_release.clone(),
    }
}

#[test]
fn pure_constructor_matches_new_and_replace_partition_corpora() {
    let unused_workspace = tempfile::tempdir().unwrap();
    let before = std::fs::read_dir(unused_workspace.path()).unwrap().count();
    for raw in [NEW, REPLACE] {
        let (plan, subject) = prepared(raw);
        let built =
            plan_from_captured_remote(&subject, &plan_expected(), &captured(&plan)).unwrap();
        assert_eq!(built.plan, plan);
        assert_eq!(
            built.digest,
            publication_effect_plan_digest(&plan, &subject, &plan_expected()).unwrap()
        );
        assert_eq!(
            built.digest,
            plan_from_captured_remote(&subject, &plan_expected(), &captured(&plan))
                .unwrap()
                .digest
        );
    }
    assert_eq!(
        std::fs::read_dir(unused_workspace.path()).unwrap().count(),
        before
    );
}

#[test]
fn captured_remote_generation_id_and_visibility_change_constructed_identity() {
    let (plan, subject) = prepared(REPLACE);
    let baseline = plan_from_captured_remote(&subject, &plan_expected(), &captured(&plan)).unwrap();
    let mut changed = captured(&plan);
    changed.generation_ref.push_str("-next");
    assert_ne!(
        baseline.digest,
        plan_from_captured_remote(&subject, &plan_expected(), &changed)
            .unwrap()
            .digest
    );
    let mut changed = captured(&plan);
    let PublicationObservedRelease::Present(release) = &mut changed.release else {
        panic!("fixture replaces a draft")
    };
    release.release_id = "124".into();
    assert_ne!(
        baseline.digest,
        plan_from_captured_remote(&subject, &plan_expected(), &changed)
            .unwrap()
            .digest
    );
    let mut selected = plan_expected();
    selected.publish = true;
    let publish = plan_from_captured_remote(&subject, &selected, &captured(&plan)).unwrap();
    assert_ne!(baseline.digest, publish.digest);
    assert!(validate_publication_effect_plan(&publish.plan, &subject, &plan_expected()).is_err());
}

#[test]
fn constructor_refuses_incomplete_or_incompatible_remote_before_plan() {
    let (plan, subject) = prepared(REPLACE);
    let mut remote = captured(&plan);
    let PublicationObservedRelease::Present(release) = &mut remote.release else {
        panic!("fixture replaces a draft")
    };
    release.inventory_complete = false;
    assert!(plan_from_captured_remote(&subject, &plan_expected(), &remote).is_err());
    let mut remote = captured(&plan);
    let PublicationObservedRelease::Present(release) = &mut remote.release else {
        panic!("fixture replaces a draft")
    };
    release.draft = false;
    assert!(plan_from_captured_remote(&subject, &plan_expected(), &remote).is_err());
    let mut remote = captured(&plan);
    let vibe_wire::generated::publication_effect_plan::PublicationObservedTag::Present(tag) =
        &mut remote.tag
    else {
        panic!("fixture retains a tag")
    };
    tag.object_sha = "c".repeat(40);
    assert!(plan_from_captured_remote(&subject, &plan_expected(), &remote).is_err());
}

#[test]
fn published_mutable_release_requires_explicit_replacement_posture() {
    let (plan, subject) = prepared(REPLACE);
    let mut remote = captured(&plan);
    let PublicationObservedRelease::Present(release) = &mut remote.release else {
        panic!("fixture replaces a known release")
    };
    release.draft = false;
    assert!(plan_from_captured_remote(&subject, &plan_expected(), &remote).is_err());

    let mut selected = plan_expected();
    selected.allow_replace_published = true;
    let built = plan_from_captured_remote(&subject, &selected, &remote).unwrap();
    assert_eq!(built.plan.partitions.len(), 10);
    assert_eq!(subject.version, "1.1.0");
    let PublicationObservedRelease::Present(observed) = &built.plan.observed_release else {
        panic!("the plan must retain published-release deletion evidence")
    };
    assert!(!observed.draft);
    assert_eq!(observed.release_id, "123");
    assert!(observed.inventory_complete);
    assert_eq!(observed.assets.len(), 1);
    for partition in &built.plan.partitions {
        let assets = match partition {
            PublicationPartition::PlatformAssets(value) => &value.assets,
            PublicationPartition::FinalAssets(value) => &value.assets,
            _ => continue,
        };
        assert!(
            assets
                .iter()
                .all(|asset| matches!(asset.prior, PublicationPriorAsset::Absent(_)))
        );
    }
    assert!(validate_publication_effect_plan(&built.plan, &subject, &selected).is_ok());
    assert!(validate_publication_effect_plan(&built.plan, &subject, &plan_expected()).is_err());

    let mut next = remote;
    let PublicationObservedRelease::Present(release) = &mut next.release else {
        panic!("fixture replaces a known release")
    };
    release.release_id = "124".into();
    assert_ne!(
        built.digest,
        plan_from_captured_remote(&subject, &selected, &next)
            .unwrap()
            .digest
    );
}

#[test]
fn old_draft_assets_remain_only_in_deletion_observation() {
    let (plan, subject) = prepared(REPLACE);
    let mut remote = captured(&plan);
    let PublicationObservedRelease::Present(release) = &mut remote.release else {
        panic!("fixture replaces a draft")
    };
    release.assets.push(
        vibe_wire::generated::publication_effect_plan::PublicationObservedAsset {
            name: subject.targets[0].bundle.name.clone(),
            asset_id: "789".into(),
            size_bytes: "100".into(),
            sha256: format!("sha256:{}", "f".repeat(64)),
        },
    );
    let built = plan_from_captured_remote(&subject, &plan_expected(), &remote).unwrap();
    let PublicationObservedRelease::Present(prior_release) = &built.plan.observed_release else {
        panic!("built plan binds old release")
    };
    assert!(
        prior_release
            .assets
            .iter()
            .any(|asset| asset.asset_id == "789")
    );
    for partition in &built.plan.partitions {
        let assets = match partition {
            PublicationPartition::PlatformAssets(value) => &value.assets,
            PublicationPartition::FinalAssets(value) => &value.assets,
            _ => continue,
        };
        assert!(
            assets
                .iter()
                .all(|asset| matches!(asset.prior, PublicationPriorAsset::Absent(_)))
        );
    }
}

#[test]
fn two_complete_corpus_shapes_validate_after_binding_to_real_subject_digest() {
    for raw in [NEW, REPLACE] {
        let (plan, subject) = prepared(raw);
        validate_publication_effect_plan(&plan, &subject, &plan_expected()).unwrap();
        let first = publication_effect_plan_digest(&plan, &subject, &plan_expected()).unwrap();
        let second = publication_effect_plan_digest(&plan, &subject, &plan_expected()).unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with("sha256:"));
    }
}

#[test]
fn remote_generation_and_replaced_id_change_framed_digest() {
    let (plan, subject) = prepared(REPLACE);
    let baseline = publication_effect_plan_digest(&plan, &subject, &plan_expected()).unwrap();
    let mut changed = plan.clone();
    changed.remote_generation_ref.push_str("-new");
    assert_ne!(
        baseline,
        publication_effect_plan_digest(&changed, &subject, &plan_expected()).unwrap()
    );
    let mut changed = plan;
    let PublicationObservedRelease::Present(release) = &mut changed.observed_release else {
        panic!("fixture replaces a draft")
    };
    release.release_id = "124".into();
    assert_ne!(
        baseline,
        publication_effect_plan_digest(&changed, &subject, &plan_expected()).unwrap()
    );
}

#[test]
fn old_draft_asset_id_cannot_be_new_draft_prior() {
    let (plan, subject) = prepared(REPLACE);
    let observed = match &plan.observed_release {
        PublicationObservedRelease::Present(release) => release.assets[0].clone(),
        _ => panic!("fixture has prior draft"),
    };
    for partition_index in [2, 8] {
        let mut changed = plan.clone();
        let effect = match &mut changed.partitions[partition_index] {
            PublicationPartition::PlatformAssets(platform) => &mut platform.assets[0],
            PublicationPartition::FinalAssets(final_assets) => &mut final_assets.assets[0],
            _ => panic!("fixture has desired asset partition"),
        };
        effect.prior = PublicationPriorAsset::Present(Box::new(PublicationPriorAssetPresent {
            asset_id: observed.asset_id.clone(),
            sha256: observed.sha256.clone(),
        }));
        assert!(validate_publication_effect_plan(&changed, &subject, &plan_expected()).is_err());
    }
}

#[test]
fn nonempty_cleanup_needs_a_post_upload_observation() {
    let (mut plan, subject) = prepared(REPLACE);
    let observed = match &plan.observed_release {
        PublicationObservedRelease::Present(release) => release.assets[0].clone(),
        _ => panic!("fixture has prior draft"),
    };
    let PublicationPartition::Cleanup(cleanup) = &mut plan.partitions[7] else {
        panic!("fixture has cleanup partition")
    };
    cleanup.assets.push(observed);
    assert!(validate_publication_effect_plan(&plan, &subject, &plan_expected()).is_err());
}

#[test]
fn skipped_smoke_is_inspectable_but_not_apply_ready() {
    let (mut plan, mut subject) = prepared(NEW);
    subject.smoke.probes[0].status = PublicationSmokeProbeStatus::Skipped;
    plan.subject_sha256 = publication_subject_digest(&subject, &subject_expected()).unwrap();
    validate_publication_effect_plan(&plan, &subject, &plan_expected()).unwrap();
    assert!(publication_effect_plan_digest(&plan, &subject, &plan_expected()).is_ok());
    assert!(require_apply_ready_effect_plan(&plan, &subject, &plan_expected()).is_err());
}

#[test]
fn three_semantic_invalid_corpora_refuse() {
    let (_, subject) = prepared(REPLACE);
    for raw in [
        include_str!(
            "../../../../formats/corpora/publication_effect_plan/e1/semantic-invalid/changed-target-replacement.json"
        ),
        include_str!(
            "../../../../formats/corpora/publication_effect_plan/e1/semantic-invalid/incomplete-release-inventory.json"
        ),
        include_str!(
            "../../../../formats/corpora/publication_effect_plan/e1/semantic-invalid/wrong-partition-order.json"
        ),
    ] {
        let mut plan: PublicationEffectPlan = serde_json::from_str(raw).unwrap();
        plan.subject_sha256 = publication_subject_digest(&subject, &subject_expected()).unwrap();
        assert!(validate_publication_effect_plan(&plan, &subject, &plan_expected()).is_err());
    }
}

#[test]
fn four_structural_invalid_corpora_fail_generated_decoding() {
    for raw in [
        include_str!(
            "../../../../formats/corpora/publication_effect_plan/e1/structural-invalid/unknown-root.json"
        ),
        include_str!(
            "../../../../formats/corpora/publication_effect_plan/e1/structural-invalid/unknown-partition-arm.json"
        ),
        include_str!(
            "../../../../formats/corpora/publication_effect_plan/e1/structural-invalid/missing-prior-state.json"
        ),
        include_str!(
            "../../../../formats/corpora/publication_effect_plan/e1/structural-invalid/wrong-release-id-type.json"
        ),
    ] {
        assert!(serde_json::from_str::<PublicationEffectPlan>(raw).is_err());
    }
}
