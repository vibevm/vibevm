use super::super::state::DeployState;
use super::tests::*;
use super::*;
const CLOCK: &str = "2026-10-15T00:00:00Z";

#[test]
fn identical_standalone_receipts_cannot_substitute_donor_roots() {
    use vibe_agent_projection::{
        agents::{Agent, Scope},
        pkgskill::{SkillDonor, install_package_skill},
    };
    use vibe_core::manifest::ClientShape;
    let path = ".claude/skills/demo/SKILL.md";
    let f = Fixture::new(request(vec![binding(
        ClientScope::Project,
        ClientShape::File,
        path,
        &[],
    )]));
    let other = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let bytes = b"---\nname: demo\ndescription: fixture\n---\nbody\n";
    std::fs::write(source.path().join("SKILL.md"), bytes).unwrap();
    for root in [f.host.project.root_path(), other.path()] {
        install_package_skill(
            Agent::parse_filter("claude").unwrap()[0],
            Scope::Project,
            Some(root),
            "demo",
            source.path(),
            false,
        )
        .unwrap();
    }
    let a = SkillDonor::standalone(
        Project::open(f.host.project.root_path()).unwrap(),
        ".claude/skills",
        "demo",
    )
    .unwrap();
    let b = SkillDonor::standalone(
        Project::open(other.path()).unwrap(),
        ".claude/skills",
        "demo",
    )
    .unwrap();
    assert_eq!(a.id(), b.id());
    assert_eq!(a.receipt_bytes(), b.receipt_bytes());
    assert_ne!(a.root_identity(), b.root_identity());
    let admitted = f.admitted(vec![GenericPayload::File(bytes.to_vec())]);
    let plan = f.host.prepare_skill_adoption(&admitted, &a).unwrap();
    let grant = f.host.approve(&plan).unwrap();
    let error = f
        .host
        .adopt_skill(&plan, Some(&grant), &b, CLOCK)
        .unwrap_err();
    assert_eq!(error.effect, GenericEffect::NoEffects);
    a.validate().unwrap();
    b.validate().unwrap();
    assert!(
        DeployState::observe_bound(&f.host.settings)
            .unwrap()
            .is_none()
    );
    for root in [f.host.project.root_path(), other.path()] {
        assert!(!root.join(".vibe/generic-skill-transfer.json").exists());
        assert_eq!(std::fs::read(root.join(path)).unwrap(), bytes);
    }
    f.host.adopt_skill(&plan, Some(&grant), &a, CLOCK).unwrap();
    let inverse = f
        .host
        .prepare_retained(plan.target(), RetainedOperation::RestoreDonor)
        .unwrap();
    let inverse_grant = f.host.approve_retained(&inverse).unwrap();
    assert_eq!(
        f.host
            .execute_skill_retained(&inverse, Some(&inverse_grant), &b, CLOCK)
            .unwrap_err()
            .effect,
        GenericEffect::NoEffects
    );
    b.validate().unwrap();
    let resumed = SkillDonor::resume(Project::open(a.root()).unwrap()).unwrap();
    f.host
        .execute_skill_retained(&inverse, Some(&inverse_grant), &resumed, CLOCK)
        .unwrap();
    assert_eq!(
        std::fs::read(
            a.root()
                .join(".claude/skills/.demo.vibe-skill-receipt.toml")
        )
        .unwrap(),
        a.receipt_bytes()
    );
}

#[test]
fn incumbent_logical_deployment_identity_imports_and_restores_exact_receipt() {
    use super::super::support::{
        Fixture as Incumbent, FixtureProvider, selected, selection, target,
    };
    use super::super::{apply_selection, state::DeploymentHome};
    use vibe_core::manifest::ClientShape;
    let f = Fixture::new(request(vec![binding(
        ClientScope::Project,
        ClientShape::File,
        "legacy-file",
        &[],
    )]));
    let incumbent = Incumbent::new("artifact");
    let targets = [target("legacy-target", "helper.exe", &[])];
    let selection = selection("local", &["legacy-target"]);
    let state_home = f
        .host
        .settings
        .root_path()
        .join(super::super::model::DEPLOY_STATE_DIR);
    let mut execution = incumbent.execution(&targets, &selection, &state_home);
    execution.package = Some("legacy-package");
    let resource = f
        .host
        .project
        .root_path()
        .join("legacy-file")
        .to_str()
        .unwrap()
        .to_owned();
    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(FixtureProvider::new(
                f.host.project.root_path(),
                &[&resource],
            )),
        )],
    )
    .unwrap();
    assert_ne!(execution.project, f.host.roots[0]);
    let home = DeploymentHome::new(
        &state_home,
        execution.project,
        execution.package,
        "legacy-target",
    );
    let original = std::fs::read(home.directory().join("receipt.json")).unwrap();
    let donor = f
        .host
        .deployment_donor(execution.project, "legacy-target", execution.package)
        .unwrap();
    assert!(
        f.host
            .deployment_donor(&f.host.roots[0], "legacy-target", execution.package)
            .is_err()
    );
    assert!(
        f.host
            .deployment_donor(execution.project, "legacy-target", None)
            .is_err()
    );
    let admitted = f.admitted(vec![GenericPayload::File(
        std::fs::read(&resource).unwrap(),
    )]);
    let plan = f
        .host
        .prepare_deployment_adoption(&admitted, &donor)
        .unwrap();
    // Identical receipt bytes at the same logical triple in a second state
    // root cannot stand in for the exact approved deployment donor either.
    let other_settings = tempfile::tempdir().unwrap();
    let other_host = GenericHost::new(
        Project::open(f.host.project.root_path()).unwrap(),
        Project::open(f.host.user.root_path()).unwrap(),
        Project::open(other_settings.path()).unwrap(),
        f.authority.clone(),
        f.host.protected.clone(),
    )
    .unwrap();
    let other_home = DeploymentHome::new(
        &other_settings
            .path()
            .join(super::super::model::DEPLOY_STATE_DIR),
        execution.project,
        execution.package,
        "legacy-target",
    );
    std::fs::create_dir_all(other_home.directory()).unwrap();
    std::fs::write(other_home.directory().join("receipt.json"), &original).unwrap();
    let other_donor = other_host
        .deployment_donor(execution.project, "legacy-target", execution.package)
        .unwrap();
    let grant = f.host.approve(&plan).unwrap();
    assert_eq!(
        f.host
            .adopt_deployment(&plan, Some(&grant), &other_donor, CLOCK)
            .unwrap_err()
            .effect,
        GenericEffect::NoEffects
    );
    assert_eq!(
        std::fs::read(home.directory().join("receipt.json")).unwrap(),
        original
    );
    assert_eq!(
        std::fs::read(other_home.directory().join("receipt.json")).unwrap(),
        original
    );
    assert!(matches!(
        f.host.inspect_receipt(plan.target()).unwrap(),
        ReceiptInspection::Absent
    ));
    f.host
        .adopt_deployment(&plan, Some(&f.host.approve(&plan).unwrap()), &donor, CLOCK)
        .unwrap();
    f.authority.selected.lock().unwrap().selected = false;
    let resumed = f
        .host
        .deployment_donor(execution.project, "legacy-target", execution.package)
        .unwrap();
    let inverse = f
        .host
        .prepare_retained(plan.target(), RetainedOperation::RestoreDonor)
        .unwrap();
    assert_eq!(
        f.host
            .execute_deployment_retained(
                &inverse,
                Some(&f.host.approve_retained(&inverse).unwrap()),
                &other_donor,
                CLOCK
            )
            .unwrap_err()
            .effect,
        GenericEffect::NoEffects
    );
    f.host
        .execute_deployment_retained(
            &inverse,
            Some(&f.host.approve_retained(&inverse).unwrap()),
            &resumed,
            CLOCK,
        )
        .unwrap();
    assert_eq!(
        std::fs::read(home.directory().join("receipt.json")).unwrap(),
        original
    );
}

#[test]
fn replaced_named_arena_refuses_stale_generic_and_incumbent_hosts() {
    use super::super::{
        CoordinatedDeploy,
        support::{Fixture as Incumbent, selection},
    };
    let mut f = file_fixture();
    let arena_parent = tempfile::tempdir().unwrap();
    let arena = arena_parent.path().join("arena");
    std::fs::create_dir(&arena).unwrap();
    f.host = f
        .host
        .with_coordination(Project::open(&arena).unwrap())
        .unwrap();
    let stale = CoordinatedDeploy::new(Project::open(&arena).unwrap()).unwrap();
    let plan = f.plan(vec![GenericPayload::File(b"value".to_vec())]);
    let grant = f.host.approve(&plan).unwrap();
    std::fs::rename(&arena, arena_parent.path().join("old")).unwrap();
    std::fs::create_dir(&arena).unwrap();
    assert_eq!(
        f.host.apply(&plan, Some(&grant), CLOCK).unwrap_err().effect,
        GenericEffect::NoEffects
    );
    assert!(
        DeployState::observe_bound(&f.host.settings)
            .unwrap()
            .is_none()
    );
    assert!(
        !f.host
            .project
            .root_path()
            .join(&plan.resources[0].destination)
            .exists()
    );
    let incumbent = Incumbent::new("artifact");
    let selection = selection("local", &[]);
    let home = incumbent.state_home();
    let execution = incumbent.execution(&[], &selection, &home);
    assert!(stale.apply(&execution).is_err());
    assert!(stale.undeploy(&execution).is_err());
    assert!(!home.exists());
    let fresh = CoordinatedDeploy::new(Project::open(&arena).unwrap()).unwrap();
    assert!(fresh.apply(&execution).unwrap().is_empty());
    f.host = f
        .host
        .with_coordination(Project::open(&arena).unwrap())
        .unwrap();
    let plan = f.plan(vec![GenericPayload::File(b"value".to_vec())]);
    f.host
        .apply(&plan, Some(&f.host.approve(&plan).unwrap()), CLOCK)
        .unwrap();
    assert_eq!(
        f.bytes(ClientScope::Project, &plan.resources[0].destination),
        b"value"
    );
    assert!(!arena_parent.path().join("old/.vibe").exists());
}

#[test]
fn interrupted_member_inverse_does_not_complete_other_member() {
    use vibe_core::manifest::ClientShape;
    let f = Fixture::new(request(vec![
        binding(
            ClientScope::Project,
            ClientShape::JsonMember,
            "config.json",
            &["a"],
        ),
        binding(
            ClientScope::Project,
            ClientShape::JsonMember,
            "config.json",
            &["b"],
        ),
    ]));
    let first = f.plan(vec![
        GenericPayload::JsonMember(1.into()),
        GenericPayload::JsonMember(2.into()),
    ]);
    f.host
        .apply(&first, Some(&f.host.approve(&first).unwrap()), CLOCK)
        .unwrap();
    let cleanup = f
        .host
        .prepare_retained(first.target(), RetainedOperation::Cleanup)
        .unwrap();
    let grant = f.host.approve_retained(&cleanup).unwrap();
    vibe_safefs::fail_after_publish(Some("config.json"));
    let result = f.host.execute_retained(&cleanup, Some(&grant), CLOCK);
    vibe_safefs::fail_after_publish(None);
    assert!(result.is_err());
    let state = DeployState::observe_bound(&f.host.settings)
        .unwrap()
        .unwrap();
    let p = state
        .read_generic(&f.host.home(first.target()))
        .unwrap()
        .unwrap()
        .pending
        .unwrap();
    use vibe_wire::generated::deploy_generic_state::ResourceProgress;
    assert_eq!(p.resources[0].progress, ResourceProgress::Uncertain);
    assert_eq!(p.resources[1].progress, ResourceProgress::NotStarted);
    let recovery = f.host.prepare_recovery(first.target(), None).unwrap();
    f.host
        .execute_retained(
            &recovery,
            Some(&f.host.approve_retained(&recovery).unwrap()),
            CLOCK,
        )
        .unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&f.bytes(ClientScope::Project, "config.json")).unwrap();
    assert_eq!(value, serde_json::json!({}));
}

#[test]
fn changed_coordination_arena_refuses_retained_recovery() {
    let mut f = file_fixture();
    let first = f.plan(vec![GenericPayload::File(b"value".to_vec())]);
    f.host
        .apply(&first, Some(&f.host.approve(&first).unwrap()), CLOCK)
        .unwrap();
    let arena = tempfile::tempdir().unwrap();
    f.host = f
        .host
        .with_coordination(Project::open(arena.path()).unwrap())
        .unwrap();
    assert!(
        f.host
            .inspect_receipt(first.target())
            .unwrap_err()
            .reason
            .contains("arena identity mismatch")
    );
}

#[test]
fn c2_migration_requires_closed_match_and_never_invents_before_state() {
    let f = file_fixture();
    let first = f.plan(vec![GenericPayload::File(b"legacy".to_vec())]);
    f.host
        .apply(&first, Some(&f.host.approve(&first).unwrap()), CLOCK)
        .unwrap();
    let home = f.host.home(first.target());
    std::fs::remove_file(home.directory().join("generic-state.json")).unwrap();
    assert!(matches!(
        f.host.inspect_receipt(first.target()).unwrap(),
        ReceiptInspection::Legacy { .. }
    ));
    let admitted = f.admitted(vec![GenericPayload::File(b"legacy".to_vec())]);
    assert!(f.host.plan(&admitted).is_err());
    let wrong = f.admitted(vec![GenericPayload::File(b"different".to_vec())]);
    assert!(f.host.prepare_migration(&wrong).is_err());
    let migration = f.host.prepare_migration(&admitted).unwrap();
    f.host
        .apply(
            &migration,
            Some(&f.host.approve(&migration).unwrap()),
            CLOCK,
        )
        .unwrap();
    assert!(
        f.host
            .prepare_retained(first.target(), RetainedOperation::Inverse)
            .is_err()
    );
    f.authority.selected.lock().unwrap().selected = false;
    let cleanup = f
        .host
        .prepare_retained(first.target(), RetainedOperation::Cleanup)
        .unwrap();
    f.host
        .execute_retained(
            &cleanup,
            Some(&f.host.approve_retained(&cleanup).unwrap()),
            CLOCK,
        )
        .unwrap();
}

#[test]
fn member_inverse_preserves_current_foreign_neighbors() {
    use vibe_core::manifest::ClientShape;
    let f = Fixture::new(request(vec![binding(
        ClientScope::Project,
        ClientShape::JsonMember,
        "config.json",
        &["literal.dot"],
    )]));
    f.write(ClientScope::Project, "config.json", br#"{"foreign":1}"#);
    let first = f.plan(vec![GenericPayload::JsonMember(serde_json::json!("old"))]);
    f.host
        .apply(&first, Some(&f.host.approve(&first).unwrap()), CLOCK)
        .unwrap();
    let next = f.plan(vec![GenericPayload::JsonMember(serde_json::json!("new"))]);
    f.host
        .apply(&next, Some(&f.host.approve(&next).unwrap()), CLOCK)
        .unwrap();
    f.write(
        ClientScope::Project,
        "config.json",
        br#"{"foreign":2,"literal.dot":"new","other":true}"#,
    );
    f.authority.selected.lock().unwrap().selected = false;
    let inverse = f
        .host
        .prepare_retained(next.target(), RetainedOperation::Inverse)
        .unwrap();
    f.host
        .execute_retained(
            &inverse,
            Some(&f.host.approve_retained(&inverse).unwrap()),
            CLOCK,
        )
        .unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&f.bytes(ClientScope::Project, "config.json")).unwrap();
    assert_eq!(
        value,
        serde_json::json!({"foreign":2,"literal.dot":"old","other":true})
    );
}

#[test]
fn removed_source_inverse_restores_owned_value() {
    let f = file_fixture();
    let first = f.plan(vec![GenericPayload::File(b"old".to_vec())]);
    f.host
        .apply(&first, Some(&f.host.approve(&first).unwrap()), CLOCK)
        .unwrap();
    let next = f.plan(vec![GenericPayload::File(b"new".to_vec())]);
    f.host
        .apply(&next, Some(&f.host.approve(&next).unwrap()), CLOCK)
        .unwrap();
    f.authority.selected.lock().unwrap().selected = false;
    let inverse = f
        .host
        .prepare_retained(next.target(), RetainedOperation::Inverse)
        .unwrap();
    let grant = f.host.approve_retained(&inverse).unwrap();
    f.host
        .execute_retained(&inverse, Some(&grant), CLOCK)
        .unwrap();
    assert_eq!(
        f.bytes(ClientScope::Project, &next.resources[0].destination),
        b"old"
    );
    let state = DeployState::observe_bound(&f.host.settings)
        .unwrap()
        .unwrap();
    let receipt = state
        .read_receipt(&f.host.home(next.target()))
        .unwrap()
        .unwrap();
    assert_eq!(
        receipt.status,
        vibe_wire::generated::deploy_receipt::ReceiptStatus::Verified
    );
    assert_eq!(receipt.resources[0].post_digest, digest(b"old"));
}

#[test]
fn revocation_refuses_forward_but_allows_current_cleanup() {
    let f = file_fixture();
    let first = f.plan(vec![GenericPayload::File(b"old".to_vec())]);
    f.host
        .apply(&first, Some(&f.host.approve(&first).unwrap()), CLOCK)
        .unwrap();
    let revoke = f
        .host
        .prepare_retained(first.target(), RetainedOperation::Revocation)
        .unwrap();
    let grant = f.host.approve_retained(&revoke).unwrap();
    f.host
        .execute_retained(&revoke, Some(&grant), CLOCK)
        .unwrap();
    assert!(
        f.host
            .plan(&f.admitted(vec![GenericPayload::File(b"new".to_vec())]))
            .is_err()
    );
    f.authority.selected.lock().unwrap().enabled = false;
    let cleanup = f
        .host
        .prepare_retained(first.target(), RetainedOperation::Cleanup)
        .unwrap();
    assert!(
        f.host
            .execute_retained(&cleanup, Some(&grant), CLOCK)
            .is_err()
    );
    let cleanup_grant = f.host.approve_retained(&cleanup).unwrap();
    f.host
        .execute_retained(&cleanup, Some(&cleanup_grant), CLOCK)
        .unwrap();
    assert!(
        f.host
            .project
            .read_file(&first.resources[0].destination)
            .unwrap()
            .is_none()
    );
}

#[test]
fn durable_identity_excludes_process_capture() {
    let f = file_fixture();
    let plan = f.plan(vec![GenericPayload::File(b"retained".to_vec())]);
    let mut other = plan.clone();
    other.admitted.capture.host += 1;
    assert_eq!(plan.identity(), other.identity());
    assert!(f.host.approve(&other).is_err());
}

#[test]
fn typed_receipt_join_and_private_blob_integrity() {
    let f = file_fixture();
    let plan = f.plan(vec![GenericPayload::File(b"private-value".to_vec())]);
    let grant = f.host.approve(&plan).unwrap();
    f.host.apply(&plan, Some(&grant), CLOCK).unwrap();
    let state = DeployState::observe_bound(&f.host.settings)
        .unwrap()
        .unwrap();
    let home = f.host.home(&plan.target);
    let record = state.read_generic(&home).unwrap().unwrap();
    assert!(record.pending.is_none());
    let c = record.committed.as_ref().unwrap();
    assert_eq!(c.generation, 0);
    let receipt = state.read_receipt(&home).unwrap().unwrap();
    assert_eq!(c.receipt_digest, Some(digest(&encoded(&receipt).unwrap())));
    let json = String::from_utf8(encoded(&record).unwrap()).unwrap();
    assert!(!json.contains("private-value"));
    if let vibe_wire::generated::deploy_generic_state::Value::Present(v) = &c.resources[0].desired {
        assert_eq!(
            state.read_generic_blob(&home, &v.blob).unwrap(),
            b"private-value"
        );
        let mut changed = v.blob.clone();
        changed.bytes += 1;
        assert!(state.read_generic_blob(&home, &changed).is_err());
        std::fs::remove_file(home.directory().join("private-blobs").join(&v.blob.digest)).unwrap();
        assert!(
            f.host
                .prepare_retained(plan.target(), RetainedOperation::Cleanup)
                .unwrap_err()
                .reason
                .contains("missing retained private blob")
        );
    } else {
        panic!("present resource");
    }
    assert!(state.require_legacy_state(&home).is_err());
}

#[test]
fn interrupted_publication_retains_uncertain_state() {
    let f = file_fixture();
    let first = f.plan(vec![GenericPayload::File(b"old".to_vec())]);
    let first_grant = f.host.approve(&first).unwrap();
    f.host.apply(&first, Some(&first_grant), CLOCK).unwrap();
    let plan = f.plan(vec![GenericPayload::File(b"value".to_vec())]);
    let grant = f.host.approve(&plan).unwrap();
    vibe_safefs::fail_after_publish(Some(&plan.resources[0].destination));
    let result = f.host.apply(&plan, Some(&grant), CLOCK);
    vibe_safefs::fail_after_publish(None);
    assert!(result.is_err());
    let state = DeployState::observe_bound(&f.host.settings)
        .unwrap()
        .unwrap();
    let home = f.host.home(&plan.target);
    let record = state.read_generic(&home).unwrap().unwrap();
    let p = record.pending.unwrap();
    assert_eq!(
        p.resources[0].progress,
        vibe_wire::generated::deploy_generic_state::ResourceProgress::Uncertain
    );
    assert!(state.read_intent(&home).unwrap().is_some());
    assert_eq!(state.read_receipt(&home).unwrap().unwrap().generation, 0);
    assert!(f.host.prepare_recovery(plan.target(), None).is_err());
    let admitted = f.admitted(vec![GenericPayload::File(b"value".to_vec())]);
    let recovery = f
        .host
        .prepare_recovery(plan.target(), Some(&admitted))
        .unwrap();
    assert_eq!(recovery.operation(), RetainedOperation::Recovery);
    let grant = f.host.approve_retained(&recovery).unwrap();
    f.host
        .execute_retained(&recovery, Some(&grant), CLOCK)
        .unwrap();
    assert!(
        state
            .read_generic(&home)
            .unwrap()
            .unwrap()
            .pending
            .is_none()
    );
    assert_eq!(state.read_receipt(&home).unwrap().unwrap().generation, 1);
}

#[test]
fn upgrade_added_retained_removed_and_inverse_restore_exact_set() {
    use vibe_core::manifest::ClientShape;
    let f = Fixture::new(request(vec![
        binding(
            ClientScope::Project,
            ClientShape::JsonMember,
            "config.json",
            &["old"],
        ),
        binding(ClientScope::Project, ClientShape::File, "retained", &[]),
    ]));
    f.write(ClientScope::Project, "config.json", br#"{"foreign":7}"#);
    let first = f.plan(vec![
        GenericPayload::JsonMember(1.into()),
        GenericPayload::File(b"old".to_vec()),
    ]);
    f.host
        .apply(&first, Some(&f.host.approve(&first).unwrap()), CLOCK)
        .unwrap();
    {
        let mut selected = f.authority.selected.lock().unwrap();
        selected.request = request(vec![
            binding(
                ClientScope::Project,
                ClientShape::JsonMember,
                "config.json",
                &["new"],
            ),
            binding(ClientScope::Project, ClientShape::File, "retained", &[]),
        ]);
        selected.identity.descriptor_digest = digest(&encoded(&selected.request).unwrap());
    }
    let next = f.plan(vec![
        GenericPayload::JsonMember(2.into()),
        GenericPayload::File(b"new".to_vec()),
    ]);
    f.host
        .apply(&next, Some(&f.host.approve(&next).unwrap()), CLOCK)
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&f.bytes(ClientScope::Project, "config.json"))
            .unwrap(),
        serde_json::json!({"new":2,"foreign":7})
    );
    f.authority.selected.lock().unwrap().selected = false;
    let inverse = f
        .host
        .prepare_retained(next.target(), RetainedOperation::Inverse)
        .unwrap();
    f.host
        .execute_retained(
            &inverse,
            Some(&f.host.approve_retained(&inverse).unwrap()),
            CLOCK,
        )
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&f.bytes(ClientScope::Project, "config.json"))
            .unwrap(),
        serde_json::json!({"old":1,"foreign":7})
    );
    assert_eq!(f.bytes(ClientScope::Project, "retained"), b"old");
}

#[test]
fn initial_generation_recovers_publication_and_refuses_third_digest() {
    let f = file_fixture();
    let plan = f.plan(vec![GenericPayload::File(b"initial".to_vec())]);
    super::super::state::generic::FAIL_PHASE.with(|p| {
        *p.borrow_mut() = Some(vibe_wire::generated::deploy_generic_state::BindingPhase::Checkpoint)
    });
    let result = f
        .host
        .apply(&plan, Some(&f.host.approve(&plan).unwrap()), CLOCK);
    vibe_safefs::fail_after_publish(None);
    assert!(result.is_err());
    let admitted = f.admitted(vec![GenericPayload::File(b"initial".to_vec())]);
    f.write(
        ClientScope::Project,
        &plan.resources[0].destination,
        b"third",
    );
    assert!(
        f.host
            .prepare_recovery(plan.target(), Some(&admitted))
            .unwrap_err()
            .reason
            .contains("third digest")
    );
    f.write(
        ClientScope::Project,
        &plan.resources[0].destination,
        b"initial",
    );
    let recovery = f
        .host
        .prepare_recovery(plan.target(), Some(&admitted))
        .unwrap();
    f.host
        .execute_retained(
            &recovery,
            Some(&f.host.approve_retained(&recovery).unwrap()),
            CLOCK,
        )
        .unwrap();
    let ReceiptInspection::Retained {
        receipt: Some(receipt),
        state,
    } = f.host.inspect_receipt(plan.target()).unwrap()
    else {
        panic!("retained")
    };
    assert_eq!(receipt.generation, 0);
    assert!(state.pending.is_none());
}

#[test]
fn standalone_transfer_restores_exact_donor_without_source() {
    use vibe_agent_projection::{
        agents::{Agent, Scope},
        pkgskill::{SkillDonor, install_package_skill, uninstall_package_skill},
    };
    use vibe_core::manifest::ClientShape;
    let path = ".claude/skills/demo/SKILL.md";
    let f = Fixture::new(request(vec![binding(
        ClientScope::Project,
        ClientShape::File,
        path,
        &[],
    )]));
    let source = tempfile::tempdir().unwrap();
    let bytes = b"---\nname: demo\ndescription: fixture\n---\nbody\n";
    std::fs::write(source.path().join("SKILL.md"), bytes).unwrap();
    install_package_skill(
        Agent::parse_filter("claude").unwrap()[0],
        Scope::Project,
        Some(f.host.project.root_path()),
        "demo",
        source.path(),
        false,
    )
    .unwrap();
    let donor = SkillDonor::standalone(
        Project::open(f.host.project.root_path()).unwrap(),
        ".claude/skills",
        "demo",
    )
    .unwrap();
    let admitted = f.admitted(vec![GenericPayload::File(bytes.to_vec())]);
    assert!(f.host.plan(&admitted).is_err());
    let plan = f.host.prepare_skill_adoption(&admitted, &donor).unwrap();
    f.host
        .adopt_skill(&plan, Some(&f.host.approve(&plan).unwrap()), &donor, CLOCK)
        .unwrap();
    assert!(
        uninstall_package_skill(
            Agent::parse_filter("claude").unwrap()[0],
            Scope::Project,
            Some(f.host.project.root_path()),
            "demo",
            false
        )
        .is_err()
    );
    {
        let mut selected = f.authority.selected.lock().unwrap();
        selected.request = request(vec![binding(
            ClientScope::Project,
            ClientShape::File,
            "replacement",
            &[],
        )]);
        selected.identity.descriptor_digest = digest(&encoded(&selected.request).unwrap());
    }
    let upgraded = f.plan(vec![GenericPayload::File(b"replacement".to_vec())]);
    f.host
        .apply(&upgraded, Some(&f.host.approve(&upgraded).unwrap()), CLOCK)
        .unwrap();
    assert!(f.host.project.read_file(path).unwrap().is_none());
    drop(donor);
    let donor = SkillDonor::resume(Project::open(f.host.project.root_path()).unwrap()).unwrap();
    f.authority.selected.lock().unwrap().selected = false;
    let inverse = f
        .host
        .prepare_retained(plan.target(), RetainedOperation::RestoreDonor)
        .unwrap();
    f.host
        .execute_skill_retained(
            &inverse,
            Some(&f.host.approve_retained(&inverse).unwrap()),
            &donor,
            CLOCK,
        )
        .unwrap();
    let ReceiptInspection::Retained {
        receipt: Some(receipt),
        ..
    } = f.host.inspect_receipt(plan.target()).unwrap()
    else {
        panic!("receipt")
    };
    assert!(receipt.resources.is_empty());
    assert_eq!(f.bytes(ClientScope::Project, path), bytes);
    assert!(f.host.project.read_file("replacement").unwrap().is_none());
    uninstall_package_skill(
        Agent::parse_filter("claude").unwrap()[0],
        Scope::Project,
        Some(f.host.project.root_path()),
        "demo",
        false,
    )
    .unwrap();
}

#[test]
fn deployment_c2_transfer_and_exact_donor_restore() {
    let f = file_fixture();
    let first = f.plan(vec![GenericPayload::File(b"legacy".to_vec())]);
    f.host
        .apply(&first, Some(&f.host.approve(&first).unwrap()), CLOCK)
        .unwrap();
    let home = f.host.home(first.target());
    std::fs::remove_file(home.directory().join("generic-state.json")).unwrap();
    let store = DeployState::observe_bound(&f.host.settings)
        .unwrap()
        .unwrap();
    let original = store.read_receipt(&home).unwrap().unwrap();
    {
        let mut selected = f.authority.selected.lock().unwrap();
        selected.request.client_id = "org.example/recipient".into();
        selected.identity.descriptor_digest = digest(&encoded(&selected.request).unwrap());
    }
    let donor = f
        .host
        .deployment_donor(&f.host.roots[0], first.target(), None)
        .unwrap();
    let admitted = f.admitted(vec![GenericPayload::File(b"legacy".to_vec())]);
    let plan = f
        .host
        .prepare_deployment_adoption(&admitted, &donor)
        .unwrap();
    f.host
        .adopt_deployment(&plan, Some(&f.host.approve(&plan).unwrap()), &donor, CLOCK)
        .unwrap();
    assert!(
        store
            .read_receipt(&home)
            .unwrap()
            .unwrap()
            .resources
            .is_empty()
    );
    assert!(store.require_legacy_state(&home).is_err());
    let donor = f
        .host
        .deployment_donor(&f.host.roots[0], first.target(), None)
        .unwrap();
    f.authority.selected.lock().unwrap().selected = false;
    let inverse = f
        .host
        .prepare_retained(plan.target(), RetainedOperation::RestoreDonor)
        .unwrap();
    f.host
        .execute_deployment_retained(
            &inverse,
            Some(&f.host.approve_retained(&inverse).unwrap()),
            &donor,
            CLOCK,
        )
        .unwrap();
    assert_eq!(store.read_receipt(&home).unwrap().unwrap(), original);
    store.require_legacy_state(&home).unwrap();
}

#[test]
fn initial_phase_classes_recover_with_exact_fresh_authority() {
    use vibe_wire::generated::deploy_generic_state::BindingPhase;
    for phase in [
        BindingPhase::Prepared,
        BindingPhase::Intent,
        BindingPhase::Checkpoint,
        BindingPhase::Receipt,
        BindingPhase::Committed,
    ] {
        let f = file_fixture();
        let plan = f.plan(vec![GenericPayload::File(b"phase".to_vec())]);
        let grant = f.host.approve(&plan).unwrap();
        super::super::state::generic::FAIL_PHASE.with(|p| *p.borrow_mut() = Some(phase.clone()));
        assert!(
            f.host.apply(&plan, Some(&grant), CLOCK).is_err(),
            "{phase:?}"
        );
        let admitted = f.admitted(vec![GenericPayload::File(b"phase".to_vec())]);
        let recovery = f
            .host
            .prepare_recovery(plan.target(), Some(&admitted))
            .unwrap();
        f.host
            .execute_retained(
                &recovery,
                Some(&f.host.approve_retained(&recovery).unwrap()),
                CLOCK,
            )
            .unwrap();
        assert_eq!(
            f.bytes(ClientScope::Project, &plan.resources[0].destination),
            b"phase"
        );
        let store = DeployState::observe_bound(&f.host.settings)
            .unwrap()
            .unwrap();
        assert!(
            store
                .read_intent(&f.host.home(plan.target()))
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn automatic_donor_retirement_interruption_recovers_then_restores() {
    use vibe_agent_projection::pkgskill::SkillDonor;
    use vibe_core::manifest::ClientShape;
    use vibe_wire::generated::package_skill_receipt::*;
    let path = ".claude/skills/demo/SKILL.md";
    let f = Fixture::new(request(vec![binding(
        ClientScope::Project,
        ClientShape::File,
        path,
        &[],
    )]));
    f.write(ClientScope::Project, path, b"body");
    let key = "@vibe/package/skill/org.example/pkg/demo";
    let receipt = PackageSkillReceipt {
        schema: 2,
        applying: None,
        binding: vec![PackageSkillBinding {
            key: key.into(),
            provider: "org.example/pkg".into(),
            skill: "demo".into(),
            source_snapshot: format!("sha256:{}", digest(b"body")),
            target: vec![PackageSkillTarget {
                agent: "claude".into(),
                path: f
                    .host
                    .project
                    .root_path()
                    .join(".claude/skills/demo")
                    .to_string_lossy()
                    .into_owned(),
                file: vec![PackageSkillFile {
                    path: "SKILL.md".into(),
                    sha256: format!("sha256:{}", digest(b"body")),
                }],
            }],
        }],
    };
    f.write(
        ClientScope::Project,
        ".vibe/package-skills.toml",
        toml::to_string_pretty(&receipt).unwrap().as_bytes(),
    );
    let donor =
        SkillDonor::automatic(Project::open(f.host.project.root_path()).unwrap(), key).unwrap();
    let admitted = f.admitted(vec![GenericPayload::File(b"body".to_vec())]);
    let plan = f.host.prepare_skill_adoption(&admitted, &donor).unwrap();
    let grant = f.host.approve(&plan).unwrap();
    vibe_safefs::fail_after_publish(Some(".vibe/package-skills.toml"));
    let result = f.host.adopt_skill(&plan, Some(&grant), &donor, CLOCK);
    vibe_safefs::fail_after_publish(None);
    assert!(result.is_err());
    let donor = SkillDonor::resume(Project::open(f.host.project.root_path()).unwrap()).unwrap();
    let recovery = f
        .host
        .prepare_recovery(plan.target(), Some(&admitted))
        .unwrap();
    f.host
        .execute_skill_retained(
            &recovery,
            Some(&f.host.approve_retained(&recovery).unwrap()),
            &donor,
            CLOCK,
        )
        .unwrap();
    f.authority.selected.lock().unwrap().selected = false;
    let inverse = f
        .host
        .prepare_retained(plan.target(), RetainedOperation::RestoreDonor)
        .unwrap();
    f.host
        .execute_skill_retained(
            &inverse,
            Some(&f.host.approve_retained(&inverse).unwrap()),
            &donor,
            CLOCK,
        )
        .unwrap();
    let restored: PackageSkillReceipt = toml::from_str(
        std::str::from_utf8(&f.bytes(ClientScope::Project, ".vibe/package-skills.toml")).unwrap(),
    )
    .unwrap();
    assert_eq!(restored, receipt);
}

#[test]
fn retained_mode_create_new_restoration_and_existing_occupant_refusal() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let f = file_fixture();
        let initial = f.plan(vec![GenericPayload::File(b"mode".to_vec())]);
        f.host
            .apply(&initial, Some(&f.host.approve(&initial).unwrap()), CLOCK)
            .unwrap();
        let path = &initial.resources[0].destination;
        // Fixture changes a mode before an owned upgrade captures its exact
        // current preimage; this is an explicit local setup effect.
        std::fs::set_permissions(
            f.host.project.root_path().join(path),
            std::fs::Permissions::from_mode(0o640),
        )
        .unwrap();
        let upgraded = f.plan(vec![GenericPayload::File(b"mode2".to_vec())]);
        f.host
            .apply(&upgraded, Some(&f.host.approve(&upgraded).unwrap()), CLOCK)
            .unwrap();
        let cleanup = f
            .host
            .prepare_retained(initial.target(), RetainedOperation::Cleanup)
            .unwrap();
        f.host
            .execute_retained(
                &cleanup,
                Some(&f.host.approve_retained(&cleanup).unwrap()),
                CLOCK,
            )
            .unwrap();
        let inverse = f
            .host
            .prepare_retained(initial.target(), RetainedOperation::Inverse)
            .unwrap();
        f.host
            .execute_retained(
                &inverse,
                Some(&f.host.approve_retained(&inverse).unwrap()),
                CLOCK,
            )
            .unwrap();
        let snapshot = f
            .host
            .project
            .read_file_snapshot_bounded(path, 100)
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.unix_mode, Some(0o640));
        assert_eq!(snapshot.bytes, b"mode2");
        let root = f.host.project.root_dir().unwrap();
        f.host
            .project
            .publish_new_in_with_mode(&root, "occupied", b"same", Some(0o640))
            .unwrap();
        assert!(
            f.host
                .project
                .publish_new_in_with_mode(&root, "occupied", b"same", Some(0o600))
                .is_err()
        );
        assert_eq!(
            f.host
                .project
                .read_file_snapshot_bounded("occupied", 100)
                .unwrap()
                .unwrap()
                .unix_mode,
            Some(0o640)
        );
    }
}

#[test]
fn representable_legacy_file_and_opencode_member_transfer_preserve_neighbors() {
    use vibe_core::manifest::ClientShape;
    for member in [false, true] {
        let (scope, shape, path, key) = if member {
            (
                ClientScope::User,
                ClientShape::JsonMember,
                ".config/opencode/opencode.json",
                vec!["mcp", "server"],
            )
        } else {
            (
                ClientScope::Project,
                ClientShape::File,
                "legacy-file",
                vec![],
            )
        };
        let f = Fixture::new(request(vec![binding(scope, shape, path, &key)]));
        if member {
            f.write(scope, path, br#"{"foreign":true}"#);
        }
        let payload = if member {
            GenericPayload::JsonMember(serde_json::json!({"command":"inert-fixture"}))
        } else {
            GenericPayload::File(b"legacy".to_vec())
        };
        let first = f.plan(vec![payload.clone()]);
        f.host
            .apply(&first, Some(&f.host.approve(&first).unwrap()), CLOCK)
            .unwrap();
        let home = f.host.home(first.target());
        let state = DeployState::observe_bound(&f.host.settings)
            .unwrap()
            .unwrap();
        let mut old = state.read_receipt(&home).unwrap().unwrap();
        old.resources[0].resource = if member {
            format!("home:{path}#mcp/server")
        } else {
            f.host
                .project
                .root_path()
                .join(path)
                .to_string_lossy()
                .into_owned()
        };
        state.write_receipt(&home, &old).unwrap();
        std::fs::remove_file(home.directory().join("generic-state.json")).unwrap();
        {
            let mut current = f.authority.selected.lock().unwrap();
            current.request.client_id = "org.example/recipient".into();
            current.identity.descriptor_digest = digest(&encoded(&current.request).unwrap());
        }
        let donor = f
            .host
            .deployment_donor(&f.host.roots[0], first.target(), None)
            .unwrap();
        let admitted = f.admitted(vec![payload]);
        let plan = f
            .host
            .prepare_deployment_adoption(&admitted, &donor)
            .unwrap();
        f.host
            .adopt_deployment(&plan, Some(&f.host.approve(&plan).unwrap()), &donor, CLOCK)
            .unwrap();
        f.authority.selected.lock().unwrap().selected = false;
        let inverse = f
            .host
            .prepare_retained(plan.target(), RetainedOperation::RestoreDonor)
            .unwrap();
        f.host
            .execute_deployment_retained(
                &inverse,
                Some(&f.host.approve_retained(&inverse).unwrap()),
                &donor,
                CLOCK,
            )
            .unwrap();
        assert_eq!(state.read_receipt(&home).unwrap().unwrap(), old);
        if member {
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&f.bytes(scope, path)).unwrap()["foreign"],
                true
            );
        }
    }
}

#[test]
fn toml_literal_inverse_preserves_current_foreign_neighbors() {
    use vibe_core::manifest::ClientShape;
    let f = Fixture::new(request(vec![binding(
        ClientScope::Project,
        ClientShape::TomlMember,
        "config.toml",
        &["literal.dot"],
    )]));
    f.write(ClientScope::Project, "config.toml", b"foreign = 1\n");
    let first = f.plan(vec![GenericPayload::TomlMember(toml::Value::String(
        "old".into(),
    ))]);
    f.host
        .apply(&first, Some(&f.host.approve(&first).unwrap()), CLOCK)
        .unwrap();
    let next = f.plan(vec![GenericPayload::TomlMember(toml::Value::String(
        "new".into(),
    ))]);
    f.host
        .apply(&next, Some(&f.host.approve(&next).unwrap()), CLOCK)
        .unwrap();
    f.write(
        ClientScope::Project,
        "config.toml",
        b"foreign = 2\n\"literal.dot\" = \"new\"\n",
    );
    f.authority.selected.lock().unwrap().enabled = false;
    let inverse = f
        .host
        .prepare_retained(next.target(), RetainedOperation::Inverse)
        .unwrap();
    f.host
        .execute_retained(
            &inverse,
            Some(&f.host.approve_retained(&inverse).unwrap()),
            CLOCK,
        )
        .unwrap();
    let value: toml::Value =
        toml::from_str(std::str::from_utf8(&f.bytes(ClientScope::Project, "config.toml")).unwrap())
            .unwrap();
    assert_eq!(value["foreign"].as_integer(), Some(2));
    assert_eq!(value["literal.dot"].as_str(), Some("old"));
}

#[test]
fn receipt_publication_window_rejects_changed_record_then_recovers() {
    let f = file_fixture();
    let plan = f.plan(vec![GenericPayload::File(b"receipt-window".to_vec())]);
    let home = f.host.home(plan.target());
    let grant = f.host.approve(&plan).unwrap();
    vibe_safefs::fail_after_publish(Some(&format!("{}/receipt.json", home.id())));
    let result = f.host.apply(&plan, Some(&grant), CLOCK);
    vibe_safefs::fail_after_publish(None);
    assert!(result.is_err());
    let store = DeployState::observe_bound(&f.host.settings)
        .unwrap()
        .unwrap();
    let original = store.read_receipt(&home).unwrap().unwrap();
    let mut changed = original.clone();
    changed.evidence = Some("changed durable record".into());
    store.write_receipt(&home, &changed).unwrap();
    assert!(f.host.inspect_receipt(plan.target()).is_err());
    store.write_receipt(&home, &original).unwrap();
    let admitted = f.admitted(vec![GenericPayload::File(b"receipt-window".to_vec())]);
    let recovery = f
        .host
        .prepare_recovery(plan.target(), Some(&admitted))
        .unwrap();
    f.host
        .execute_retained(
            &recovery,
            Some(&f.host.approve_retained(&recovery).unwrap()),
            CLOCK,
        )
        .unwrap();
    assert!(
        store
            .read_generic(&home)
            .unwrap()
            .unwrap()
            .pending
            .is_none()
    );
}
