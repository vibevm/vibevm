use super::*;
use vibe_lifecycle::{JournalAdmission, SubmissionNamespace};

fn ready_world(binary: &Path) -> World {
    let world = world(binary);
    world.stage_links(2);
    let approved = world.held();
    assert!(matches!(
        retain_complete_pending(
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
    // Inspector-only fixture: this manually selects C.1 Ready after every
    // Pending byte store/aggregate landed. No owner seal or effect grant.
    let namespace = SubmissionNamespace::from_host_token(world.namespace.clone()).unwrap();
    SubmissionReservations::mark_journal_ready(
        &world.lease,
        &namespace,
        &world.scope,
        "aggregate",
        &world.run_id,
    )
    .unwrap();
    world
}

#[test]
fn two_link_ready_rehashes_every_store_and_distinct_flattened_tree() {
    let Some(binary) = binary() else { return };
    let world = ready_world(&binary);
    let index_tree = git(&binary, world.source.root_path(), &["write-tree"]);
    let (_, proof) = package_git_operation::ready_proof_cold(
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
        "gitlink index tree is not flattened output tree"
    );
    let GitIndexCensus::PopulatedCandidate { links, .. } =
        package_git_gitlink::census(&world.source, &binary, limits().census)
    else {
        panic!("complete index absent")
    };
    for link in &links {
        assert!(matches!(
            package_git_submodule_store::inspect_one_cold(
                &world.lease,
                &world.namespace,
                &world.scope,
                "aggregate",
                &link.path,
                None,
                None,
                &binary,
                limits(),
                JournalAdmission::Ready,
            ),
            LinkStoreInspection::ByteStoreCompleteButNotReady {
                source_reobserved: false,
                ..
            }
        ));
        assert!(
            package_git_submodule_store::proved_link_receipt(
                &world.lease,
                &world.namespace,
                &world.scope,
                "aggregate",
                &link.path,
                None,
                None,
                &binary,
                limits(),
                JournalAdmission::Ready,
            )
            .is_ok()
        );
    }
    assert_eq!(
        inspect_complete_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            None,
            None,
            &binary,
            limits(),
            JournalAdmission::Ready,
        ),
        AggregateInspection::AggregateCompleteButNotReady {
            run_id: world.run_id.clone(),
            flattened_submodules_sha256: world.digest.clone(),
            source_reobserved: false,
        }
    );
    let approved = world.held();
    assert!(matches!(
        inspect_complete_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            Some(&world.source),
            Some(&approved),
            &binary,
            limits(),
            JournalAdmission::Ready,
        ),
        AggregateInspection::AggregateCompleteButNotReady {
            source_reobserved: true,
            ..
        }
    ));
}

#[test]
fn pending_stage_entrypoints_refuse_after_c1_ready_without_changing_receipts() {
    let Some(binary) = binary() else { return };
    let world = ready_world(&binary);
    let aggregate = journal::read_bytes(&world.lease, &world.run_id, LANDED)
        .unwrap()
        .unwrap();
    let GitIndexCensus::PopulatedCandidate { links, .. } =
        package_git_gitlink::census(&world.source, &binary, limits().census)
    else {
        panic!()
    };
    assert!(matches!(
        package_git_submodule_store::stage_one_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &world.approved[0],
            &links[0],
            &binary,
            limits(),
        ),
        LinkStoreInspection::Uncertain { .. }
    ));
    let approved = world.held();
    assert!(matches!(
        retain_complete_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved,
            &binary,
            limits(),
        ),
        AggregateInspection::Uncertain { .. }
    ));
    assert_eq!(
        journal::read_bytes(&world.lease, &world.run_id, LANDED)
            .unwrap()
            .unwrap(),
        aggregate
    );
}

#[test]
fn missing_or_tampered_one_ready_link_refuses_aggregate() {
    let Some(binary) = binary() else { return };
    for tamper in [false, true] {
        let world = ready_world(&binary);
        let receipt = fs::read_dir(world.root.path().join(".vibe/package-git-operations"))
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
            .expect("per-link Landed receipt absent");
        if tamper {
            fs::write(&receipt, b"not-json").unwrap();
        } else {
            fs::remove_file(&receipt).unwrap();
        }
        assert!(matches!(
            inspect_complete_cold(
                &world.lease,
                &world.namespace,
                &world.scope,
                "aggregate",
                None,
                None,
                &binary,
                limits(),
                JournalAdmission::Ready,
            ),
            AggregateInspection::Uncertain { .. }
        ));
    }
}

#[test]
fn changed_retained_blob_after_ready_refuses_whole_aggregate() {
    let Some(binary) = binary() else { return };
    let world = ready_world(&binary);
    let blob = git(
        &binary,
        &world.source.root_path().join("vendor/a"),
        &["rev-parse", "HEAD:data.txt"],
    );
    let home = world.root.path().join(".vibe/package-git-operations");
    let loose = fs::read_dir(home)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains(".gitlink-")
        })
        .map(|path| path.join(format!("objects/{}/{}", &blob[..2], &blob[2..])))
        .find(|path| path.is_file())
        .expect("selected Gitlink blob not retained");
    fs::remove_file(loose).unwrap();
    assert!(matches!(
        inspect_complete_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            None,
            None,
            &binary,
            limits(),
            JournalAdmission::Ready,
        ),
        AggregateInspection::Uncertain { .. }
    ));
}
