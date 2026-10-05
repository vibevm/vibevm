use super::*;
use crate::machine_operation::fixed_deploy::FixedDeployRoute;
use serde_json::json;
use std::{fs, path::Path};
use vibe_core::manifest::{DeployTarget, ExtensionConfig, ExtensionsControl, MechanismRoutes};
use vibe_extension_registry::{
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, collect_mechanisms,
};
use vibe_lifecycle::ClientExecutable;
use vibe_wire::{behaviour::artifact_record::validate, generated::artifact_record::ArtifactRecord};

fn registry() -> Arc<MechanismRegistry> {
    let world = ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("deploy-grant"),
                root: ".".into(),
                version: "1.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    };
    Arc::new(collect_mechanisms(&world).unwrap())
}
fn clients() -> ClientExecutables {
    ClientExecutables {
        claude: ClientExecutable::Missing {
            command: "claude".into(),
        },
        codex: ClientExecutable::Missing {
            command: "codex".into(),
        },
        opencode: ClientExecutable::Missing {
            command: "opencode".into(),
        },
    }
}
fn target() -> DeployTarget {
    DeployTarget {
        id: "local-helper".into(),
        artifact: "helper.exe".into(),
        mechanism: "deploy:vibe-bin".parse().unwrap(),
        provider: None,
        when: None,
        depends_on: Some(Vec::new()),
        config: Some(ExtensionConfig::from_table(
            "command = 'local-helper'".parse::<toml::Table>().unwrap(),
        )),
    }
}
fn record(root: &Path, artifact: &Path) {
    let digest = SelectedFileWitness::from_bytes(fs::read(artifact).unwrap()).sha256;
    let record: ArtifactRecord = serde_json::from_value(json!({
        "schema":1,"id":"helper.exe","kind":"executable","shape":"file",
        "path_absolute":artifact.to_string_lossy().replace('\\', "/"),
        "path_relative":{"root":"project","path":"target/debug/helper.exe"},
        "digest":{"algorithm":"sha256","value":digest.trim_start_matches("sha256:")},
        "producer":{"target":"helper","mechanism":"build:cargo",
            "provider":{"key":"org.vibevm/vibe#cargo"}},
        "freshness":{},"created_at":"2026-10-05T12:00:00Z",
        "verification":{"status":"verified","evidence":"fixture artifact"}
    }))
    .unwrap();
    validate(&record).unwrap();
    let home = root.join(".vibe/state/artifacts");
    fs::create_dir_all(&home).unwrap();
    fs::write(
        home.join("helper.exe.json"),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();
}
struct Fixture {
    project: tempfile::TempDir,
    settings: tempfile::TempDir,
    home: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let project = tempfile::tempdir().unwrap();
        let settings = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        fs::write(
            project.path().join("vibe.toml"),
            concat!(
                "[project]\nname = 'demo'\nversion = '1.1.0'\n\n",
                "[[artifacts.build]]\nid = 'helper'\nmechanism = 'build:cargo'\n",
                "outputs = [{ id = 'helper.exe', kind = 'executable' }]\n\n",
                "[[deploy.target]]\nid = 'local-helper'\nartifact = 'helper.exe'\n",
                "mechanism = 'deploy:vibe-bin'\ndepends_on = []\n",
                "config = { command = 'local-helper' }\n\n",
                "[deploy.profiles.local]\ntargets = ['local-helper']\n",
            ),
        )
        .unwrap();
        let artifact = project.path().join("target/debug/helper.exe");
        fs::create_dir_all(artifact.parent().unwrap()).unwrap();
        fs::write(&artifact, b"helper executable").unwrap();
        record(project.path(), &artifact);
        Self {
            project,
            settings,
            home,
        }
    }
    fn service(&self) -> DeployGrantService {
        DeployGrantService::from_trusted_host(
            Arc::new(Project::open(self.project.path()).unwrap()),
            Arc::new(Project::open(self.settings.path()).unwrap()),
            Arc::new(Project::open(self.home.path()).unwrap()),
            SubmissionScope {
                context: "cli-deploy-host:e1:trusted".into(),
                target: "instance-a".into(),
                principal: "owner".into(),
            },
            "local".into(),
            target(),
            registry(),
            Arc::new(MechanismRoutes::default()),
            clients(),
            "org.example/demo".into(),
            Some("generation-a".into()),
            "observer-a".into(),
            "nonce-a".into(),
            1,
            Arc::new(|| Ok(Some("generation-a".into()))),
            TrustedPolicySource::from_host_identifier("vibe-cli/fixed-deploy/e1".into()).unwrap(),
        )
        .unwrap()
    }
}

#[test]
fn old_deploy_pointer_recovers_known_run_after_manifest_drift() {
    let fixture = Fixture::new();
    let service = fixture.service();
    let host = service.issue(60_000).unwrap();
    let first = host.submit("once").unwrap();
    let DeployInspection::Completed { run_id, .. } = first else {
        panic!("first fixed Deploy did not complete");
    };
    let pointer_before = fs::read(fixture.project.path().join(&service.path)).unwrap();

    let manifest = fixture.project.path().join("vibe.toml");
    let mut source = fs::read_to_string(&manifest).unwrap();
    source.push_str("# current host manifest drift\n");
    fs::write(manifest, source).unwrap();
    assert!(matches!(
        host.submit("new-effect"),
        Err(FixedDeployHostError::Policy(
            PolicyError::StartupDisagreement
        ))
    ));
    let lease = LifecycleLease::acquire(fixture.project.path()).unwrap();
    let pointer = service.read(&lease).unwrap().unwrap();
    let namespace = service.validate_namespace(&lease, &pointer).unwrap();
    assert!(
        SubmissionReservations::inspect_existing_key(
            &lease,
            &namespace,
            &service.scope,
            "new-effect"
        )
        .unwrap()
        .is_none()
    );
    drop(lease);
    drop(host);
    drop(service);
    let restarted = fixture.service();
    let attached = restarted.reattach().unwrap();
    let DeployGrantAttachment::RecoveryOnly(host) = attached else {
        panic!("changed manifest retained effect authority");
    };
    let observed = host.inspect("once").unwrap();
    assert!(matches!(observed, DeployInspection::Completed { run_id: seen, .. } if seen == run_id));
    assert_eq!(
        fs::read(fixture.project.path().join(&restarted.path)).unwrap(),
        pointer_before
    );
    assert!(matches!(
        restarted.issue(60_000),
        Err(DeployGrantError::AlreadyIssued)
    ));
    let index: serde_json::Value = serde_json::from_slice(
        &fs::read(
            fixture
                .project
                .path()
                .join(".vibe/machine-submissions.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
}

#[test]
fn damaged_deploy_pointer_never_reissues_namespace() {
    let fixture = Fixture::new();
    let service = fixture.service();
    service.issue(60_000).unwrap();
    fs::write(fixture.project.path().join(&service.path), b"{damaged").unwrap();
    assert!(matches!(
        service.reattach(),
        Err(DeployGrantError::InvalidPointer(_))
    ));
    assert!(matches!(
        service.issue(60_000),
        Err(DeployGrantError::InvalidPointer(_))
    ));
    let index: serde_json::Value = serde_json::from_slice(
        &fs::read(
            fixture
                .project
                .path()
                .join(".vibe/machine-submissions.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
}

#[test]
fn malformed_manifest_witness_cannot_recover_or_reissue() {
    let fixture = Fixture::new();
    let service = fixture.service();
    service.issue(60_000).unwrap();
    let path = fixture.project.path().join(&service.path);
    let mut pointer: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    pointer["manifest_sha256"] = json!("sha256:bad");
    fs::write(&path, serde_json::to_vec(&pointer).unwrap()).unwrap();
    assert!(matches!(
        service.reattach(),
        Err(DeployGrantError::ForeignBinding)
    ));
    assert!(matches!(
        service.issue(60_000),
        Err(DeployGrantError::ForeignBinding)
    ));
}

#[test]
fn policy_revision_drift_revokes_effects_but_keeps_old_key_inspection() {
    let fixture = Fixture::new();
    let service = fixture.service();
    let original = service.issue(60_000).unwrap();
    let DeployGrantAttachment::Active(restarted) = fixture.service().reattach().unwrap() else {
        panic!("matching trusted facts did not retain an active host");
    };
    assert!(matches!(
        restarted.inspect("old-key").unwrap(),
        DeployInspection::Absent
    ));

    let owner = PolicyOwner::attach_readonly(
        Arc::new(Project::open(fixture.project.path()).unwrap()),
        service.scope.clone(),
    )
    .unwrap();
    let previous = owner.observe_readonly().unwrap();
    let lease = LifecycleLease::acquire(fixture.project.path()).unwrap();
    owner
        .update_under_lease(
            &lease,
            previous.generation(),
            service.policy_source.clone(),
            deploy_policy_facts(),
        )
        .unwrap();
    drop(lease);

    assert!(matches!(
        original.submit("fresh"),
        Err(FixedDeployHostError::Policy(
            PolicyError::StartupDisagreement
        ))
    ));
    assert!(matches!(
        restarted.submit("fresh"),
        Err(FixedDeployHostError::Policy(
            PolicyError::StartupDisagreement
        ))
    ));
    let DeployGrantAttachment::RecoveryOnly(recovery) = fixture.service().reattach().unwrap()
    else {
        panic!("policy generation drift retained effect authority");
    };
    assert!(matches!(
        recovery.inspect("old-key").unwrap(),
        DeployInspection::Absent
    ));
}

#[test]
fn profile_drift_keeps_only_old_key_inspection() {
    let fixture = Fixture::new();
    let service = fixture.service();
    service.issue(60_000).unwrap();

    let mut restarted = fixture.service();
    restarted.profile = "other-profile".into();
    let DeployGrantAttachment::RecoveryOnly(recovery) = restarted.reattach().unwrap() else {
        panic!("changed profile retained effect authority");
    };
    assert!(matches!(
        recovery.inspect("old-key").unwrap(),
        DeployInspection::Absent
    ));
}

#[test]
fn generation_observer_drift_keeps_only_old_key_inspection() {
    let fixture = Fixture::new();
    let service = fixture.service();
    service.issue(60_000).unwrap();

    let mut restarted = fixture.service();
    restarted.generation_observer_id = "observer-b".into();
    restarted.selected_generation_ref = Some("generation-b".into());
    restarted.generation = Arc::new(|| Ok(Some("generation-b".into())));
    let DeployGrantAttachment::RecoveryOnly(recovery) = restarted.reattach().unwrap() else {
        panic!("changed generation observer retained effect authority");
    };
    assert!(matches!(
        recovery.inspect("old-key").unwrap(),
        DeployInspection::Absent
    ));
}

#[test]
fn recovery_route_rejects_submit_with_retained_effect_truth() {
    use vibe_wire::generated::machine::e1::{
        fixed_deploy_action_error::FixedDeployActionErrorEffectObservation as Effect,
        fixed_deploy_action_reply::Reply,
    };

    let fixture = Fixture::new();
    let service = fixture.service();
    let DeployInspection::Completed { run_id, .. } =
        service.issue(60_000).unwrap().submit("once").unwrap()
    else {
        panic!("fixture Deploy did not complete");
    };
    let mut source = fs::read_to_string(fixture.project.path().join("vibe.toml")).unwrap();
    source.push_str("# changed after the retained effect\n");
    fs::write(fixture.project.path().join("vibe.toml"), source).unwrap();

    let DeployGrantAttachment::RecoveryOnly(host) = fixture.service().reattach().unwrap() else {
        panic!("drift retained effect authority");
    };
    let route = FixedDeployRoute::recovery_only(Arc::new(host));
    let request = |kind: &str, key: &str| {
        serde_json::to_vec(&json!({"schema":1,"action":{"kind":kind,"key":key}})).unwrap()
    };
    let reply = FixedDeployRoute::dispatch(
        Some(&route),
        &request("inspect", "once"),
        fixture.project.path(),
    )
    .unwrap();
    assert!(matches!(reply.reply, Reply::Completed(row) if row.run_id == run_id));

    let old = FixedDeployRoute::dispatch(
        Some(&route),
        &request("submit", "once"),
        fixture.project.path(),
    )
    .unwrap_err();
    assert!(old.effects_may_have_begun);
    assert_eq!(old.error.known_run_id.as_deref(), Some(run_id.as_str()));
    assert_eq!(old.error.effect_observation, Effect::Landed);

    let envelope = serde_json::to_vec(&json!({
        "schema": 1,
        "operation_id": crate::machine_operation::fixed_deploy::OPERATION_ID,
        "domain_owner": crate::machine_operation::fixed_deploy::DOMAIN_OWNER,
        "source_id": null,
        "evidence_id": null,
        "body": {
            "stage": "input",
            "format_id": vibe_wire::generated::format_id::FormatId::MachineFixedDeployActionRequest.id(),
            "payload_json": String::from_utf8(request("submit", "once")).unwrap()
        }
    }))
    .unwrap();
    let published =
        FixedDeployRoute::exchange_local(Some(&route), &envelope, fixture.project.path()).unwrap();
    let vibe_wire::generated::machine::e1::operation_envelope::Body::Error(error) = published.body
    else {
        panic!("recovery-only submit did not publish a typed error");
    };
    assert_eq!(
        error.effect_state,
        vibe_wire::generated::machine::e1::operation_envelope::BodyErrorEffectState::Completed
    );
    assert_eq!(error.provenance_id.as_deref(), Some(run_id.as_str()));

    let fresh = FixedDeployRoute::dispatch(
        Some(&route),
        &request("submit", "new-effect"),
        fixture.project.path(),
    )
    .unwrap_err();
    assert!(!fresh.effects_may_have_begun);
    assert_eq!(fresh.error.effect_observation, Effect::None);
    let lease = LifecycleLease::acquire(fixture.project.path()).unwrap();
    let pointer = service.read(&lease).unwrap().unwrap();
    let namespace = service.validate_namespace(&lease, &pointer).unwrap();
    assert!(
        SubmissionReservations::inspect_existing_key(
            &lease,
            &namespace,
            &service.scope,
            "new-effect"
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn recovery_only_deploy_reattach_keeps_partial_effects() {
    let fixture = Fixture::new();
    let service = fixture.service();
    let first = service.issue(60_000).unwrap().submit("once").unwrap();
    let DeployInspection::Completed { run_id, .. } = first else {
        panic!("fixture Deploy did not complete");
    };
    fs::remove_file(fixture.project.path().join(format!(
        ".vibe/deploy-operations/{run_id}.bound-vibe-bin-input.json"
    )))
    .unwrap();
    let mut source = fs::read_to_string(fixture.project.path().join("vibe.toml")).unwrap();
    source.push_str("# changed after old run\n");
    fs::write(fixture.project.path().join("vibe.toml"), source).unwrap();

    let DeployGrantAttachment::RecoveryOnly(host) = fixture.service().reattach().unwrap() else {
        panic!("changed source retained effect authority");
    };
    let DeployInspection::Uncertain {
        run_id: observed,
        partial,
    } = host.inspect("once").unwrap()
    else {
        panic!("damaged old capsule lost partial truth");
    };
    assert_eq!(observed, run_id);
    assert_eq!(partial.receipt, "present");
    assert_eq!(partial.sidecar, "present");
}

#[test]
fn foreign_settings_root_cannot_reattach_old_deploy_pointer() {
    let fixture = Fixture::new();
    let service = fixture.service();
    service.issue(60_000).unwrap();
    let foreign_settings = tempfile::tempdir().unwrap();
    let foreign = DeployGrantService::from_trusted_host(
        Arc::new(Project::open(fixture.project.path()).unwrap()),
        Arc::new(Project::open(foreign_settings.path()).unwrap()),
        Arc::new(Project::open(fixture.home.path()).unwrap()),
        service.scope.clone(),
        "local".into(),
        target(),
        registry(),
        Arc::new(MechanismRoutes::default()),
        clients(),
        "org.example/demo".into(),
        Some("generation-a".into()),
        "observer-a".into(),
        "nonce-a".into(),
        1,
        Arc::new(|| Ok(Some("generation-a".into()))),
        service.policy_source.clone(),
    )
    .unwrap();
    assert!(matches!(
        foreign.reattach(),
        Err(DeployGrantError::ForeignBinding)
    ));
}
