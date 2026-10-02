use super::*;

#[test]
fn generated_submit_replay_cold_and_direct_owner_share_one_ready_key_and_copy() {
    let f = Fixture::new();
    let route = f.route();
    let absent = f.exchange(&route, "inspect", "copy");
    assert!(matches!(reply(&absent).reply, rep::Reply::Absent(a) if a.key == "copy"));
    let e::Body::Result(a) = absent.body else {
        unreachable!()
    };
    assert_eq!(a.effect_state, e::BodyResultEffectState::NoEffects);
    assert_eq!(a.completeness, e::BodyResultCompleteness::Complete);
    let first = f.exchange(&route, "submit", "copy");
    let run = completed(&first);
    let modified = std::fs::metadata(f.output()).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    assert_eq!(f.exchange(&route, "submit", "copy"), first);
    drop(route);
    let cold = f.route();
    assert_eq!(f.exchange(&cold, "inspect", "copy"), first);
    assert_eq!(f.exchange(&cold, "submit", "copy"), first);
    let lease = LifecycleLease::acquire(f.root.path()).unwrap();
    let owner = submit_fixed_static_file_package(
        &lease,
        &f.namespace,
        &f.scope,
        "copy",
        "2030-01-01T00:00:00Z",
    )
    .unwrap();
    assert_eq!(contract::project(&owner).unwrap(), reply(&first).reply);
    let row = SubmissionReservations::inspect_existing_key(&lease, &f.namespace, &f.scope, "copy")
        .unwrap()
        .unwrap();
    assert_eq!(row.domain_run_id, run);
    assert_eq!(
        row.journal_admission,
        vibe_lifecycle::JournalAdmission::Ready
    );
    assert_eq!(
        std::fs::metadata(f.output()).unwrap().modified().unwrap(),
        modified
    );
    assert_eq!(std::fs::read(f.output()).unwrap(), b"copy once");
    let files: Vec<_> = std::fs::read_dir(f.root.path().join(".vibe/package-operations"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(files.len(), 3, "exactly one intent, capsule and completion");
}
#[test]
fn direct_owner_first_attaches_through_generated_route() {
    let f = Fixture::new();
    let lease = LifecycleLease::acquire(f.root.path()).unwrap();
    let owner = submit_fixed_static_file_package(
        &lease,
        &f.namespace,
        &f.scope,
        "direct",
        "2026-10-03T00:00:00Z",
    )
    .unwrap();
    drop(lease);
    let before = std::fs::metadata(f.output()).unwrap().modified().unwrap();
    assert_eq!(
        reply(&f.exchange(&f.route(), "submit", "direct")).reply,
        contract::project(&owner).unwrap()
    );
    assert_eq!(
        std::fs::metadata(f.output()).unwrap().modified().unwrap(),
        before
    );
}
