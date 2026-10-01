//! Read-only historical redemption inspection.

use std::fs;

use super::tests::World;
use super::*;

fn cold_expectation<'a>(
    verified: &'a VerifiedApproval,
    redeemed: &'a RedemptionReceipt,
    run_id: &'a str,
    request_sha256: &'a str,
) -> ColdRedemptionExpectation<'a> {
    ColdRedemptionExpectation {
        authority_id: &verified.approval_id,
        run_id,
        marker_sha256: &redeemed.marker_sha256,
        plan_sha256: &verified.plan_sha256,
        c1_plan_sha256: &verified.c1_plan_sha256,
        request_sha256,
        approval_record_sha256: &verified.record_sha256,
    }
}

#[test]
fn cold_redemption_survives_lease_exit_and_historical_expiry() {
    let world = World::new();
    let verified = world.load().unwrap(); // record expires at test time t=20
    let namespace =
        SubmissionReservations::issue_namespace(&world.lease, &world.scope, 60_000).unwrap();
    let run_id = world.reserve(&namespace, "key-a");
    let redeemed = world
        .home
        .redeem_inner(
            &world.lease,
            &namespace,
            &world.scope,
            "key-a",
            &run_id,
            b"exact-request",
            &verified,
            10,
            false,
        )
        .unwrap();
    let row = SubmissionReservations::inspect_existing_key(
        &world.lease,
        &namespace,
        &world.scope,
        "key-a",
    )
    .unwrap()
    .unwrap();
    let expected = cold_expectation(&verified, &redeemed, &run_id, &row.request_sha256);
    let World {
        settings,
        _workspace,
        home,
        lease,
        scope,
        ..
    } = world;
    drop(home);
    drop(lease);
    let lease = LifecycleLease::acquire(_workspace.path()).unwrap();
    let home = ApprovalHome::open_selected(settings.path()).unwrap();
    let marker = settings
        .path()
        .join(HOME)
        .join(REDEEMED)
        .join(marker_name(expected.authority_id));
    let before = fs::read(&marker).unwrap();
    assert_eq!(
        home.inspect_redemption_cold(&lease, &namespace, &scope, "key-a", &expected,)
            .unwrap(),
        redeemed
    );
    assert_eq!(
        fs::read(marker).unwrap(),
        before,
        "cold inspection is read-only"
    );
    let wrong_digest = sha(b"different approval record");
    let wrong = ColdRedemptionExpectation {
        approval_record_sha256: &wrong_digest,
        ..expected
    };
    assert!(matches!(
        home.inspect_redemption_cold(&lease, &namespace, &scope, "key-a", &wrong),
        Err(ApprovalError::RecoveryRequired(_))
    ));
}

#[test]
fn cold_missing_or_tampered_marker_is_recovery_required() {
    let world = World::new();
    let verified = world.load().unwrap();
    let namespace =
        SubmissionReservations::issue_namespace(&world.lease, &world.scope, 60_000).unwrap();
    let run_id = world.reserve(&namespace, "key-a");
    let redeemed = world
        .home
        .redeem_inner(
            &world.lease,
            &namespace,
            &world.scope,
            "key-a",
            &run_id,
            b"exact-request",
            &verified,
            10,
            false,
        )
        .unwrap();
    let row = SubmissionReservations::inspect_existing_key(
        &world.lease,
        &namespace,
        &world.scope,
        "key-a",
    )
    .unwrap()
    .unwrap();
    let expected = cold_expectation(&verified, &redeemed, &run_id, &row.request_sha256);
    let marker = world
        .settings
        .path()
        .join(HOME)
        .join(REDEEMED)
        .join(marker_name(expected.authority_id));
    let before = fs::read(&marker).unwrap();
    fs::remove_file(&marker).unwrap();
    assert!(matches!(
        world.home.inspect_redemption_cold(
            &world.lease,
            &namespace,
            &world.scope,
            "key-a",
            &expected,
        ),
        Err(ApprovalError::RecoveryRequired(_))
    ));
    let mut changed = before;
    changed.push(b' ');
    fs::write(&marker, changed).unwrap();
    assert!(matches!(
        world.home.inspect_redemption_cold(
            &world.lease,
            &namespace,
            &world.scope,
            "key-a",
            &expected,
        ),
        Err(ApprovalError::RecoveryRequired(_))
    ));
}

#[test]
fn cold_foreign_scope_key_or_run_refuses() {
    let world = World::new();
    let verified = world.load().unwrap();
    let namespace =
        SubmissionReservations::issue_namespace(&world.lease, &world.scope, 60_000).unwrap();
    let run_id = world.reserve(&namespace, "key-a");
    let redeemed = world
        .home
        .redeem_inner(
            &world.lease,
            &namespace,
            &world.scope,
            "key-a",
            &run_id,
            b"exact-request",
            &verified,
            10,
            false,
        )
        .unwrap();
    let row = SubmissionReservations::inspect_existing_key(
        &world.lease,
        &namespace,
        &world.scope,
        "key-a",
    )
    .unwrap()
    .unwrap();
    let expected = cold_expectation(&verified, &redeemed, &run_id, &row.request_sha256);
    let foreign = SubmissionScope {
        principal: "foreign".into(),
        ..world.scope.clone()
    };
    assert!(matches!(
        world
            .home
            .inspect_redemption_cold(&world.lease, &namespace, &foreign, "key-a", &expected,),
        Err(ApprovalError::Refused(_))
    ));
    assert!(matches!(
        world.home.inspect_redemption_cold(
            &world.lease,
            &namespace,
            &world.scope,
            "other-key",
            &expected,
        ),
        Err(ApprovalError::Refused(_))
    ));
    let other_run = vibe_lifecycle::process::propose_run_id(world.lease.root()).unwrap();
    let wrong = ColdRedemptionExpectation {
        run_id: &other_run,
        ..expected
    };
    assert!(matches!(
        world
            .home
            .inspect_redemption_cold(&world.lease, &namespace, &world.scope, "key-a", &wrong,),
        Err(ApprovalError::Refused(_))
    ));
}

#[test]
fn retired_c1_namespace_cannot_be_presented_as_usable_redemption() {
    let world = World::new();
    let verified = world.load().unwrap();
    let namespace =
        SubmissionReservations::issue_namespace(&world.lease, &world.scope, 60_000).unwrap();
    let run_id = world.reserve(&namespace, "key-a");
    let redeemed = world
        .home
        .redeem_inner(
            &world.lease,
            &namespace,
            &world.scope,
            "key-a",
            &run_id,
            b"exact-request",
            &verified,
            10,
            false,
        )
        .unwrap();
    let row = SubmissionReservations::inspect_existing_key(
        &world.lease,
        &namespace,
        &world.scope,
        "key-a",
    )
    .unwrap()
    .unwrap();
    let expected = cold_expectation(&verified, &redeemed, &run_id, &row.request_sha256);
    SubmissionReservations::retire_namespace(&world.lease, &namespace, &world.scope).unwrap();
    assert!(matches!(
        world.home.inspect_redemption_cold(
            &world.lease,
            &namespace,
            &world.scope,
            "key-a",
            &expected,
        ),
        Err(ApprovalError::Refused(_))
    ));
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let destination = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

#[test]
fn copied_settings_directory_at_same_path_cannot_revalidate_marker() {
    let world = World::new();
    let verified = world.load().unwrap();
    let namespace =
        SubmissionReservations::issue_namespace(&world.lease, &world.scope, 60_000).unwrap();
    let run_id = world.reserve(&namespace, "key-a");
    let redeemed = world
        .home
        .redeem_inner(
            &world.lease,
            &namespace,
            &world.scope,
            "key-a",
            &run_id,
            b"exact-request",
            &verified,
            10,
            false,
        )
        .unwrap();
    let row = SubmissionReservations::inspect_existing_key(
        &world.lease,
        &namespace,
        &world.scope,
        "key-a",
    )
    .unwrap()
    .unwrap();
    let expected = cold_expectation(&verified, &redeemed, &run_id, &row.request_sha256);
    let World {
        settings,
        home,
        lease,
        scope,
        ..
    } = world;
    drop(home);
    let path = settings.path().to_path_buf();
    let old = path.with_extension("original");
    fs::rename(&path, &old).unwrap();
    copy_tree(&old, &path);
    let copied = ApprovalHome::open_selected(&path).unwrap();
    assert!(matches!(
        copied.inspect_redemption_cold(&lease, &namespace, &scope, "key-a", &expected,),
        Err(ApprovalError::RecoveryRequired(_))
    ));
}
