use std::fs;
use std::path::Path;

use super::*;
use crate::package_git_operation::{self, PendingInspection};
use crate::package_git_provenance_populated::tests::{World, binary, git, limits, world};
use crate::package_git_provenance_populated::{self, AggregateInspection};
use vibe_lifecycle::JournalAdmission;

fn complete(binary: &Path) -> World {
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
    world
}

#[test]
fn complete_two_link_aggregate_stages_exact_flattened_commit_tag_under_pending() {
    let Some(binary) = binary() else { return };
    let world = complete(&binary);
    let index_tree = git(&binary, world.source.root_path(), &["write-tree"]);
    let (_, proof) = package_git_operation::pending_proof_cold(
        &world.lease,
        &world.namespace,
        &world.scope,
        "aggregate",
        Some(&world.source),
    )
    .unwrap();
    let plan = proof.unwrap().plan;
    assert_ne!(
        index_tree, plan.staged.git_tree_oid,
        "source gitlink tree is not flattened output tree"
    );
    let approved = world.held();
    let status = stage_parentless_populated_objects_pending(
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
        matches!(
            status,
            ObjectInspection::ObjectsCompleteButNotReady {
                source_reobserved: true,
                ..
            }
        ),
        "{status:?}"
    );
    let receipt = journal::read::<ObjectReceipt>(&world.lease, &world.run_id, "objects")
        .unwrap()
        .unwrap();
    let ObjectProvenance::Populated {
        pending_sha256,
        landed_sha256,
        flattened_submodules_sha256,
    } = receipt.provenance
    else {
        panic!("populated receipt downgraded to no-Gitlinks")
    };
    assert_eq!(flattened_submodules_sha256, world.digest);
    assert_eq!(
        pending_sha256,
        sha(
            &journal::read_bytes(&world.lease, &world.run_id, "gitlink-aggregate-pending")
                .unwrap()
                .unwrap()
        )
    );
    assert_eq!(
        landed_sha256,
        sha(
            &journal::read_bytes(&world.lease, &world.run_id, "gitlink-aggregate")
                .unwrap()
                .unwrap()
        )
    );
    assert_eq!(receipt.root_tree_oid, plan.staged.git_tree_oid);
    assert!(matches!(
        inspect_parentless_populated_objects_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            None,
            None,
            &binary,
            limits(),
        ),
        ObjectInspection::ObjectsCompleteButNotReady {
            source_reobserved: false,
            ..
        }
    ));
    assert!(matches!(
        package_git_operation::pending_proof_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            None,
        )
        .unwrap()
        .0,
        PendingInspection::VaultCompleteButNotReady { .. }
    ));
}

#[test]
fn incomplete_or_tampered_aggregate_refuses_before_new_object_write() {
    let Some(binary) = binary() else { return };
    for defect in ["missing-link", "tampered-link", "tampered-aggregate"] {
        let world = if defect == "missing-link" {
            let world = world(&binary);
            world.stage_links(1);
            world
        } else {
            complete(&binary)
        };
        if defect != "missing-link" {
            let home = world.root.path().join(".vibe/package-git-operations");
            let target = if defect == "tampered-aggregate" {
                home.join(format!(
                    "{}.package-git-gitlink-aggregate.json",
                    world.run_id
                ))
            } else {
                fs::read_dir(&home)
                    .unwrap()
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .find(|path| {
                        let name = path.file_name().unwrap().to_string_lossy();
                        name.contains(".package-git-gitlink-")
                            && name.ends_with(".json")
                            && !name.contains("-pending")
                            && !name.contains("aggregate")
                    })
                    .expect("per-link Landed absent")
            };
            fs::write(target, b"not-json").unwrap();
        }
        let approved = world.held();
        let status = stage_parentless_populated_objects_pending(
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
            !matches!(status, ObjectInspection::ObjectsCompleteButNotReady { .. }),
            "{defect}: {status:?}"
        );
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, "objects")
                .unwrap()
                .is_none()
        );
        let (_, proof) = package_git_operation::pending_proof_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            None,
        )
        .unwrap();
        let plan = proof.unwrap().plan;
        let objects = journal::objects(&world.lease, &world.run_id, false)
            .unwrap()
            .unwrap();
        let git = LocalGit::new(&binary, &objects, plan.oid_hex_len).unwrap();
        assert!(
            !git.object_present(&plan.staged.tag_target_commit_oid)
                .unwrap()
        );
        assert!(
            !git.object_present(plan.staged.annotated_tag_oid.as_deref().unwrap())
                .unwrap()
        );
    }
}

#[test]
fn changed_index_after_aggregate_landed_refuses_object_stage() {
    let Some(binary) = binary() else { return };
    let world = complete(&binary);
    git(
        &binary,
        world.source.root_path(),
        &["update-index", "--force-remove", "vendor/a"],
    );
    let approved = world.held();
    assert!(!matches!(
        stage_parentless_populated_objects_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved,
            &binary,
            limits(),
        ),
        ObjectInspection::ObjectsCompleteButNotReady { .. }
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "objects")
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        package_git_provenance_populated::inspect_complete_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            None,
            None,
            &binary,
            limits(),
            JournalAdmission::Pending,
        ),
        AggregateInspection::AggregateCompleteButNotReady {
            source_reobserved: false,
            ..
        }
    ));
}
