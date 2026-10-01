use std::fs;
use std::path::Path;

use super::*;
use crate::package_git_operation::{PendingInspection, prepare_pending};
use crate::package_git_provenance_populated::tests::{
    World, binary, git, limits, payload_limits, world,
};
use crate::package_git_provenance_populated::{self, AggregateInspection};

fn staged(binary: &Path) -> World {
    let world = world(binary);
    world.stage_links(2);
    let approved = world.held();
    assert!(matches!(
        package_git_provenance_populated::retain_complete_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved,
            binary,
            limits(),
        ),
        AggregateInspection::AggregateCompleteButNotReady { .. }
    ));
    assert!(matches!(
        package_git_objects::stage_parentless_populated_objects_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved,
            binary,
            limits(),
        ),
        ObjectInspection::ObjectsCompleteButNotReady { .. }
    ));
    world
}

#[test]
fn two_link_parentless_populated_seal_reaches_c1_ready_and_cold_rehashes() {
    let Some(binary) = binary() else { return };
    let world = staged(&binary);
    let approved = world.held();
    let status = seal_parentless_ready(
        &world.lease,
        &world.namespace,
        &world.scope,
        "aggregate",
        &world.source,
        &approved,
        &binary,
        limits(),
    );
    assert!(
        matches!(status, PopulatedInspection::ReadyForLocalProof { .. }),
        "{status:?}"
    );
    let bytes = journal::read_bytes(&world.lease, &world.run_id, SEAL)
        .unwrap()
        .unwrap();
    let seal: ReadySeal = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(seal.format, journal::READY_FORMAT);
    assert_eq!(
        seal.provenance,
        ReadyProvenance::Populated {
            pending_sha256: sha(&journal::read_bytes(
                &world.lease,
                &world.run_id,
                "gitlink-aggregate-pending"
            )
            .unwrap()
            .unwrap()),
            landed_sha256: sha(&journal::read_bytes(
                &world.lease,
                &world.run_id,
                "gitlink-aggregate"
            )
            .unwrap()
            .unwrap()),
            flattened_submodules_sha256: world.digest.clone(),
        }
    );
    let objects: ObjectReceipt = journal::read(&world.lease, &world.run_id, "objects")
        .unwrap()
        .unwrap();
    assert!(matches!(
        objects.provenance,
        ObjectProvenance::Populated { .. }
    ));
    let namespace = SubmissionNamespace::from_host_token(world.namespace.clone()).unwrap();
    assert_eq!(
        SubmissionReservations::journal_admission(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            &world.run_id
        )
        .unwrap(),
        JournalAdmission::Ready
    );
    assert!(matches!(
        inspect_ready_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            None,
            None,
            &binary,
            limits()
        ),
        PopulatedInspection::ReadyForLocalProof { .. }
    ));
    assert!(matches!(
        super::super::inspect_ready_seal_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            None,
            None,
            &binary,
            ReadyLimits {
                census: limits().census,
                parent: None
            }
        ),
        ReadyInspection::RecoveryRequired { .. }
    ));
    let (_, proof) = package_git_operation::ready_proof_cold(
        &world.lease,
        &world.namespace,
        &world.scope,
        "aggregate",
        Some(&world.source),
    )
    .unwrap();
    assert!(
        matches!(prepare_pending(&world.lease, &namespace, &world.scope, "aggregate",
        &world.source, &proof.unwrap().plan, payload_limits()).unwrap(),
        PendingInspection::ReadyRetainedNeedsColdProof { run_id } if run_id == world.run_id)
    );
}

#[test]
fn populated_seal_crash_before_c1_mark_resumes_same_run_without_rewrite() {
    let Some(binary) = binary() else { return };
    let world = staged(&binary);
    let approved = world.held();
    let first = seal_inner(
        &world.lease,
        &world.namespace,
        &world.scope,
        "aggregate",
        &world.source,
        &approved,
        &binary,
        limits(),
        true,
    );
    assert!(
        matches!(first, PopulatedInspection::SealRetainedC1Pending { .. }),
        "{first:?}"
    );
    let before = journal::read_bytes(&world.lease, &world.run_id, SEAL)
        .unwrap()
        .unwrap();
    let namespace = SubmissionNamespace::from_host_token(world.namespace.clone()).unwrap();
    assert_eq!(
        SubmissionReservations::journal_admission(
            &world.lease,
            &namespace,
            &world.scope,
            "aggregate",
            &world.run_id
        )
        .unwrap(),
        JournalAdmission::Pending
    );
    assert!(matches!(
        seal_parentless_ready(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved,
            &binary,
            limits()
        ),
        PopulatedInspection::ReadyForLocalProof { .. }
    ));
    assert_eq!(
        journal::read_bytes(&world.lease, &world.run_id, SEAL)
            .unwrap()
            .unwrap(),
        before
    );
}

#[test]
fn damaged_link_aggregate_or_object_arm_refuses_before_ready_publication() {
    let Some(binary) = binary() else { return };
    for defect in ["link", "aggregate", "foreign-arm", "object"] {
        let world = staged(&binary);
        let home = world.root.path().join(".vibe/package-git-operations");
        match defect {
            "link" => {
                let row = fs::read_dir(&home)
                    .unwrap()
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .find(|p| {
                        let n = p.file_name().unwrap().to_string_lossy();
                        n.contains(".package-git-gitlink-")
                            && n.ends_with(".json")
                            && !n.contains("-pending")
                            && !n.contains("aggregate")
                    })
                    .unwrap();
                fs::remove_file(row).unwrap();
            }
            "aggregate" => fs::write(
                home.join(format!(
                    "{}.package-git-gitlink-aggregate.json",
                    world.run_id
                )),
                b"not-json",
            )
            .unwrap(),
            "foreign-arm" => {
                let path = home.join(format!("{}.package-git-objects.json", world.run_id));
                let mut receipt: ObjectReceipt =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                receipt.provenance = ObjectProvenance::NoGitlinks {
                    landed_sha256: sha(b"foreign"),
                };
                fs::write(path, serde_json::to_vec(&receipt).unwrap()).unwrap();
            }
            "object" => fs::write(
                home.join(format!("{}.package-git-objects.json", world.run_id)),
                b"not-json",
            )
            .unwrap(),
            _ => unreachable!(),
        }
        let approved = world.held();
        let status = seal_parentless_ready(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved,
            &binary,
            limits(),
        );
        assert!(
            !matches!(status, PopulatedInspection::ReadyForLocalProof { .. }),
            "{defect}: {status:?}"
        );
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, SEAL)
                .unwrap()
                .is_none()
        );
        let namespace = SubmissionNamespace::from_host_token(world.namespace.clone()).unwrap();
        assert_eq!(
            SubmissionReservations::journal_admission(
                &world.lease,
                &namespace,
                &world.scope,
                "aggregate",
                &world.run_id
            )
            .unwrap(),
            JournalAdmission::Pending
        );
    }
}

#[test]
fn changed_source_index_refuses_populated_seal_without_losing_historical_objects() {
    let Some(binary) = binary() else { return };
    let world = staged(&binary);
    let approved = world.held();
    assert!(!matches!(
        seal_parentless_ready(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved[..1],
            &binary,
            limits()
        ),
        PopulatedInspection::ReadyForLocalProof { .. }
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, SEAL)
            .unwrap()
            .is_none()
    );
    git(
        &binary,
        world.source.root_path(),
        &["update-index", "--force-remove", "vendor/a"],
    );
    let status = seal_parentless_ready(
        &world.lease,
        &world.namespace,
        &world.scope,
        "aggregate",
        &world.source,
        &approved,
        &binary,
        limits(),
    );
    assert!(
        !matches!(status, PopulatedInspection::ReadyForLocalProof { .. }),
        "{status:?}"
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, SEAL)
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        package_git_objects::inspect_parentless_populated_objects_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            None,
            None,
            &binary,
            limits()
        ),
        ObjectInspection::ObjectsCompleteButNotReady { .. }
    ));
}
