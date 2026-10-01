use super::*;
use crate::package_git_approval::link::LinkInspection;

fn link(world: &World, verified: &VerifiedApproval, lost: bool, stop: bool) -> LinkInspection {
    world.home.link_inner(
        &world.lease,
        &world.namespace,
        &world.scope,
        "approval",
        Some(&world.source),
        None,
        &world.binary,
        limits(),
        verified,
        Some(10),
        lost,
        stop,
    )
}

fn cold(world: &World, approval_id: &str) -> LinkInspection {
    world.home.inspect_link_cold(
        &world.lease,
        &world.namespace,
        &world.scope,
        "approval",
        None,
        None,
        &world.binary,
        limits(),
        approval_id,
    )
}

#[test]
fn pending_precedes_marker_and_crash_does_not_redeem_on_retry() {
    let Some(binary) = binary() else { return };
    let world = World::new(&binary);
    let verified = world.load().unwrap();
    assert!(matches!(
        cold(&world, &verified.record.approval_id),
        LinkInspection::NotLinked { .. }
    ));
    assert!(matches!(
        link(&world, &verified, false, true),
        LinkInspection::RecoveryRequired { .. }
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "approval-link-pending")
            .unwrap()
            .is_some()
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "approval-link")
            .unwrap()
            .is_none()
    );
    assert!(!world.marker_path(&verified.record.approval_id).exists());
    assert!(matches!(
        link(&world, &verified, false, false),
        LinkInspection::RecoveryRequired { .. }
    ));
    assert!(!world.marker_path(&verified.record.approval_id).exists());
}

#[test]
fn returned_redemption_and_cold_marker_prove_one_local_link() {
    let Some(binary) = binary() else { return };
    let world = World::new(&binary);
    let verified = world.load().unwrap();
    let status = link(&world, &verified, false, false);
    assert!(
        matches!(&status, LinkInspection::Landed { run_id, .. } if run_id == &world.run_id),
        "{status:?}"
    );
    assert_eq!(cold(&world, &verified.record.approval_id), status);
    assert!(world.marker_path(&verified.record.approval_id).is_file());
    assert!(matches!(
        link(&world, &verified, false, false),
        LinkInspection::RecoveryRequired { .. }
    ));
}

#[test]
fn lost_reply_retains_pending_and_marker_without_landing_or_replay() {
    let Some(binary) = binary() else { return };
    let world = World::new(&binary);
    let verified = world.load().unwrap();
    assert!(matches!(
        link(&world, &verified, true, false),
        LinkInspection::RecoveryRequired { .. }
    ));
    let marker = fs::read(world.marker_path(&verified.record.approval_id)).unwrap();
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "approval-link-pending")
            .unwrap()
            .is_some()
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "approval-link")
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        cold(&world, &verified.record.approval_id),
        LinkInspection::RecoveryRequired { .. }
    ));
    assert!(matches!(
        link(&world, &verified, false, false),
        LinkInspection::RecoveryRequired { .. }
    ));
    assert_eq!(
        fs::read(world.marker_path(&verified.record.approval_id)).unwrap(),
        marker
    );
}

#[test]
fn occupied_marker_and_tampered_marker_never_land() {
    let Some(binary) = binary() else { return };
    let occupied = World::new(&binary);
    let verified = occupied.load().unwrap();
    fs::create_dir_all(occupied.settings_root.path().join(HOME).join(REDEEMED)).unwrap();
    let path = occupied.marker_path(&verified.record.approval_id);
    fs::write(&path, b"another-publication-domain").unwrap();
    assert!(matches!(
        link(&occupied, &verified, false, false),
        LinkInspection::RecoveryRequired { .. }
    ));
    assert!(
        journal::read_bytes(&occupied.lease, &occupied.run_id, "approval-link-pending")
            .unwrap()
            .is_some()
    );
    assert!(
        journal::read_bytes(&occupied.lease, &occupied.run_id, "approval-link")
            .unwrap()
            .is_none()
    );
    assert_eq!(fs::read(&path).unwrap(), b"another-publication-domain");

    let landed = World::new(&binary);
    let verified = landed.load().unwrap();
    assert!(matches!(
        link(&landed, &verified, false, false),
        LinkInspection::Landed { .. }
    ));
    fs::write(
        landed.marker_path(&verified.record.approval_id),
        b"tampered",
    )
    .unwrap();
    assert!(matches!(
        cold(&landed, &verified.record.approval_id),
        LinkInspection::RecoveryRequired { .. }
    ));
}

#[test]
fn changed_pending_or_landed_record_refuses_cold_attach() {
    let Some(binary) = binary() else { return };
    for suffix in ["approval-link-pending", "approval-link"] {
        let world = World::new(&binary);
        let verified = world.load().unwrap();
        assert!(matches!(
            link(&world, &verified, false, false),
            LinkInspection::Landed { .. }
        ));
        fs::write(
            world.root.path().join(format!(
                ".vibe/package-git-operations/{}.package-git-{suffix}.json",
                world.run_id
            )),
            b"not-json",
        )
        .unwrap();
        assert!(matches!(
            cold(&world, &verified.record.approval_id),
            LinkInspection::RecoveryRequired { .. }
        ));
    }
}
