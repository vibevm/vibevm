use super::m_10_b_2_cases::{host, result, target};
use super::*;
use crate::mechanism::deploy::{
    state::DeploymentHome,
    support::{Fixture, selection},
    view::DeployStateView,
};
use vibe_safefs::Project;
use vibe_workspace::derivatives::MaterializedDerivative;

struct Sources {
    _roots: Vec<tempfile::TempDir>,
    base: Project,
    local: Project,
    entry: Project,
}
impl Sources {
    fn new(result: &BoundDerivative) -> Self {
        let roots: Vec<_> = (0..3).map(|_| tempfile::tempdir().unwrap()).collect();
        let base = Project::open(roots[0].path()).unwrap();
        let local = Project::open(roots[1].path()).unwrap();
        let output = Project::open(roots[2].path()).unwrap();
        for (root, source) in [
            (&base, result.binding().base()),
            (&local, result.binding().local()),
        ] {
            for file in &source.files {
                root.write_atomic(&file.path, &file.bytes).unwrap();
            }
        }
        let materialized = MaterializedDerivative::derive(
            &result.record().binding_toml,
            &base,
            &local,
            &output,
            "result",
        )
        .unwrap();
        assert_eq!(materialized.result().record(), result.record());
        let entry = output.subproject(&["result"]).unwrap();
        Self {
            _roots: roots,
            base,
            local,
            entry,
        }
    }
}

#[test]
fn exact_result_artifact_grant_effect_and_independent_receipt_join() {
    // Code is published by the real builtin; it is never launched by this test.
    let bytes = "#!/bin/sh\nprintf DO_NOT_EXECUTE\\n\n";
    let fixture = Fixture::new(bytes);
    let result = result(&fixture, "upstream.exe");
    let sources = Sources::new(&result);
    let targets = [target()];
    let selected = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state);
    let admission = host(&execution);
    let binding = admission.binding().clone();
    let grant = DerivativeAdmission::bind(&result, &execution, &targets[0], admission).unwrap();
    let outcome = grant
        .execute(
            &sources.entry,
            &sources.base,
            &sources.local,
            &execution,
            &binding,
            "author",
            &mut || Ok(Some("g1".into())),
        )
        .unwrap();
    assert_eq!(outcome.len(), 1);
    let home = DeploymentHome::new_bound(
        execution.state_home,
        execution.project,
        execution.package,
        "local-helper",
        &binding,
    )
    .unwrap();
    let settings = Project::open(execution.settings_root).unwrap();
    let receipt = DeployStateView::open_bound(&settings)
        .unwrap()
        .read_receipt(&home)
        .unwrap()
        .unwrap();
    assert_eq!(receipt.generation, outcome[0].generation);
    assert_eq!(receipt.operation_binding.as_ref(), Some(&binding));
    assert_eq!(receipt.resources.len(), outcome[0].resources.len());
    for resource in &receipt.resources {
        assert!(
            outcome[0]
                .resources
                .iter()
                .any(|r| r.resource == resource.resource && r.post_digest == resource.post_digest)
        );
    }
    let (artifact, _) = super::super::artifact::resolve_bound_project_artifact(
        &Project::open(execution.project_root).unwrap(),
        &targets[0],
    )
    .unwrap();
    eprintln!(
        "M10D2_ACTIVATION_JOIN {}",
        serde_json::json!({
            "result": result.record(),
            "artifact": {"id": artifact.id, "digest": artifact.digest, "bytes": artifact.bytes, "relative": artifact.relative},
            "grant_binding": binding,
            "effect": outcome.iter().map(|row| serde_json::json!({
                "target": row.target, "generation": row.generation,
                "settlement": row.settlement,
                "resources": row.resources.iter().map(|resource| serde_json::json!({
                    "resource": resource.resource, "post_digest": resource.post_digest,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "receipt": receipt,
            "payload_execution": false,
        })
    );
    // Read actual content-addressed storage independently of the outcome DTO.
    fn find(root: &std::path::Path, bytes: &[u8]) -> bool {
        std::fs::read_dir(root).unwrap().any(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                find(&path, bytes)
            } else {
                std::fs::read(path).unwrap() == bytes
            }
        })
    }
    assert!(find(
        &fixture.settings.path().join("store"),
        bytes.as_bytes()
    ));
    assert_eq!(
        result.record().result_identity,
        MaterializedDerivative::read(&sources.entry, &sources.base, &sources.local, 1 << 20,)
            .unwrap()
            .result()
            .record()
            .result_identity
    );
}

#[test]
fn base_and_local_changes_between_plan_and_effect_refuse() {
    for change_base in [true, false] {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let result = result(&fixture, "upstream.exe");
        let sources = Sources::new(&result);
        let targets = [target()];
        let selected = selection("local", &["local-helper"]);
        let state = fixture.state_home();
        let execution = fixture.execution(&targets, &selected, &state);
        let admission = host(&execution);
        let binding = admission.binding().clone();
        let grant = DerivativeAdmission::bind(&result, &execution, &targets[0], admission).unwrap();
        let mut calls = 0;
        let error = grant
            .execute(
                &sources.entry,
                &sources.base,
                &sources.local,
                &execution,
                &binding,
                "author",
                &mut || {
                    calls += 1;
                    if calls == 2 {
                        let source = if change_base {
                            &sources.base
                        } else {
                            &sources.local
                        };
                        source
                            .write_atomic("target/debug/helper.exe", b"changed after plan")
                            .unwrap();
                    }
                    Ok(Some("g1".into()))
                },
            )
            .unwrap_err();
        assert_eq!(calls, 2);
        assert!(matches!(
            error,
            super::super::DeployError::BoundGrantChanged { .. }
        ));
        assert!(!fixture.settings.path().join("store").exists());
        assert!(!state.exists());
    }
}

#[test]
fn changed_result_principal_descriptor_and_host_refuse_before_effect() {
    for change in 0..4 {
        let fixture = Fixture::new("old");
        let old = result(&fixture, "upstream.exe");
        let sources = Sources::new(&old);
        let targets = [target()];
        let selected = selection("local", &["local-helper"]);
        let state = fixture.state_home();
        let execution = fixture.execution(&targets, &selected, &state);
        let admission = host(&execution);
        let mut binding = admission.binding().clone();
        let grant = DerivativeAdmission::bind(&old, &execution, &targets[0], admission).unwrap();
        let mut changed_targets = targets.clone();
        let mut execution = execution;
        let principal = if change == 1 { "foreign" } else { "author" };
        match change {
            0 => fixture.rebuild("changed result"),
            2 => {
                changed_targets[0].config = Some(crate::mechanism::package::support::config(
                    "command = 'foreign'",
                ));
                execution.targets = &changed_targets;
            }
            3 => binding.instance_ref = "foreign-host".into(),
            _ => {}
        }
        assert!(
            grant
                .execute(
                    &sources.entry,
                    &sources.base,
                    &sources.local,
                    &execution,
                    &binding,
                    principal,
                    &mut || Ok(Some("g1".into())),
                )
                .is_err()
        );
        assert!(!fixture.settings.path().join("store").exists());
    }
}

#[test]
fn opencode_existing_consumer_publishes_inert_derivative_skill_and_removes_only_owned_members() {
    use crate::mechanism::deploy::plugin::{
        PluginClient,
        fixture::{World, selection as plugin_selection, target as plugin_target, write},
    };
    use crate::mechanism::deploy::{execute_deploy_targets, undeploy_targets};
    mod source {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/next/derivatives/service_fixture.rs"
        ));
        pub(super) fn selected() -> (Vec<u8>, Vec<u8>) {
            let f = Fixture::new("nested-tree");
            let result = vibe_workspace::derivatives::MaterializedDerivative::derive(
                &f.text, &f.base, &f.local, &f.output, "result",
            )
            .unwrap();
            let files = &result.result().binding().local().files;
            let get = |path| {
                files
                    .iter()
                    .find(|file| file.path == path)
                    .unwrap()
                    .bytes
                    .clone()
            };
            (get("payload/SKILL.md"), get("payload/scripts/helper.sh"))
        }
    }
    let (skill, script) = source::selected();
    let world = World::new();
    let files = [
        (
            "skills/local-skill/SKILL.md",
            std::str::from_utf8(&skill).unwrap(),
        ),
        (
            "skills/local-skill/scripts/helper.sh",
            std::str::from_utf8(&script).unwrap(),
        ),
        (
            "opencode.json",
            "{\"mcp\":{\"local\":{\"type\":\"local\",\"command\":[\"DO_NOT_EXECUTE\"],\"enabled\":true}}}",
        ),
    ];
    world.record_projection("opencode-local-result", &files);
    write(
        world.home.path(),
        ".config/opencode/opencode.json",
        "{\"theme\":\"dark\",\"mcp\":{\"foreign\":{\"type\":\"remote\",\"url\":\"https://example.test\",\"enabled\":true}}}",
    );
    let targets = [plugin_target(
        PluginClient::OpenCode,
        "local-result",
        "opencode-local-result",
        "local-result",
    )];
    let selected = plugin_selection("local-result");
    let execution = world.execution(&targets, &selected);
    execute_deploy_targets(&execution).unwrap();
    assert_eq!(
        std::fs::read(world.at(".config/opencode/skills/local-skill/scripts/helper.sh")).unwrap(),
        script
    );
    execute_deploy_targets(&execution).unwrap();
    undeploy_targets(&execution).unwrap();
    assert!(
        !world
            .at(".config/opencode/skills/local-skill/SKILL.md")
            .exists()
    );
    let config: serde_json::Value =
        serde_json::from_slice(&std::fs::read(world.at(".config/opencode/opencode.json")).unwrap())
            .unwrap();
    assert_eq!(config["theme"], "dark");
    assert_eq!(config["mcp"]["foreign"]["url"], "https://example.test");
    assert!(config["mcp"].get("local").is_none());
    assert!(
        world
            .trace()
            .iter()
            .all(|line| line.contains("|--version|"))
    );
}

#[test]
fn upstream_artifact_identity_cannot_enter_active_consumer() {
    let fixture = Fixture::new("same bytes do not confer authority");
    let reused = result(&fixture, "helper.exe");
    let targets = [target()];
    let selected = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state);
    assert!(DerivativeAdmission::bind(&reused, &execution, &targets[0], host(&execution)).is_err());
    assert!(!state.exists());
    assert!(!fixture.settings.path().join("store").exists());
}

#[test]
fn source_drift_after_publication_retains_bound_intent_and_uncertain_effect() {
    let fixture = Fixture::new("#!/bin/sh\nprintf DO_NOT_EXECUTE\n");
    let result = result(&fixture, "upstream.exe");
    let sources = Sources::new(&result);
    let targets = [target()];
    let selected = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state_home);
    let admission = host(&execution);
    let binding = admission.binding().clone();
    let grant = DerivativeAdmission::bind(&result, &execution, &targets[0], admission).unwrap();
    let mut observations = 0;
    let effect = grant.execute(
        &sources.entry,
        &sources.base,
        &sources.local,
        &execution,
        &binding,
        "author",
        &mut || {
            observations += 1;
            if observations == 6 {
                sources
                    .local
                    .write_atomic("target/debug/helper.exe", b"changed after publication")
                    .unwrap();
            }
            Ok(Some("g1".into()))
        },
    );
    assert!(
        matches!(
            effect,
            Err(super::super::DeployError::BoundPublicationUncertain { .. })
        ),
        "{effect:?}"
    );
    let settings = Project::open(execution.settings_root).unwrap();
    let view = DeployStateView::open_bound(&settings).unwrap();
    let home = DeploymentHome::new_bound(
        execution.state_home,
        execution.project,
        execution.package,
        "local-helper",
        &binding,
    )
    .unwrap();
    assert_eq!(
        view.read_intent(&home)
            .unwrap()
            .unwrap()
            .operation_binding
            .as_ref(),
        Some(&binding)
    );
    assert!(view.read_receipt(&home).unwrap().is_none());
    assert!(fixture.settings.path().join("store").exists());
}

#[test]
fn missing_provider_after_admission_refuses_forward_without_erasing_result() {
    let fixture = Fixture::new("local code bytes");
    let result = result(&fixture, "upstream.exe");
    let sources = Sources::new(&result);
    let targets = [target()];
    let selected = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state);
    let admission = host(&execution);
    let binding = admission.binding().clone();
    let grant = DerivativeAdmission::bind(&result, &execution, &targets[0], admission).unwrap();
    let mut routes = vibe_core::manifest::MechanismRoutes::default();
    routes.insert(
        targets[0].mechanism.clone(),
        crate::mechanism::package::support::pin("org.example/missing#deploy"),
    );
    let mut changed = execution;
    changed.routes = &routes;
    assert!(
        grant
            .execute(
                &sources.entry,
                &sources.base,
                &sources.local,
                &changed,
                &binding,
                "author",
                &mut || Ok(Some("g1".into()))
            )
            .is_err()
    );
    assert!(!state.exists());
    assert!(!fixture.settings.path().join("store").exists());
    assert_eq!(
        MaterializedDerivative::read(&sources.entry, &sources.base, &sources.local, 1 << 20)
            .unwrap()
            .result()
            .record(),
        result.record()
    );
}

fn authored(fixture: &Fixture) -> Vec<DeployTarget> {
    let text = concat!(
        "[project]\nname = 'demo'\nversion = '1.1.0'\n",
        "[[artifacts.build]]\nid = 'helper'\nmechanism = 'build:cargo'\n",
        "outputs = [{ id = 'helper.exe', kind = 'executable' }]\n",
        "[[deploy.target]]\nid = 'local-helper'\nartifact = 'helper.exe'\n",
        "mechanism = 'deploy:vibe-bin'\ndepends_on = []\nconfig = { command = 'local-helper' }\n",
        "[deploy.profiles.local]\ntargets = ['local-helper']\n",
    );
    std::fs::write(fixture.project.path().join("vibe.toml"), text).unwrap();
    vibe_core::manifest::Manifest::parse_str(text)
        .unwrap()
        .deploy
        .unwrap()
        .targets
}

fn public_facts<'a>(
    execution: &DeployExecution<'a>,
) -> crate::retained_mechanism::DeployColdHostFacts<'a> {
    crate::retained_mechanism::DeployColdHostFacts {
        host_context_id: "fixture-host",
        instance_ref: "local-result",
        selected_generation_ref: Some("g1"),
        principal_ref: "author",
        admission_epoch: 1,
        binding_nonce: "fixture-nonce",
        allow_alias_resolution: false,
        target_abi: None,
        target_libc: None,
        project_root: execution.project_root,
        settings_root: execution.settings_root,
        user_home: execution.user_home,
    }
}

fn ordinary_join(
    execution: &DeployExecution<'_>,
    run_id: &str,
    result: &BoundDerivative,
    receipt: &vibe_wire::generated::deploy_receipt::DeployReceipt,
) {
    use crate::mechanism::vibebin::{launcher, store};
    let project = Project::open(execution.project_root).unwrap();
    let settings = Project::open(execution.settings_root).unwrap();
    let target = execution
        .targets
        .iter()
        .find(|target| target.id == receipt.target)
        .unwrap();
    let (artifact, _) =
        super::super::artifact::resolve_bound_project_artifact(&project, target).unwrap();
    assert_eq!(artifact.id, result.binding().local().resource);
    assert_eq!(artifact.digest, receipt.artifact_digest);
    let retained: serde_json::Value = serde_json::from_slice(
        &project
            .read_file(&format!(
                ".vibe/deploy-operations/{run_id}.bound-vibe-bin-bridge.json"
            ))
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    let finalized: serde_json::Value = serde_json::from_slice(
        &project
            .read_file(&format!(
                ".vibe/deploy-operations/{run_id}.bound-vibe-bin-result.json"
            ))
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    let plan = &retained["plan"];
    assert_eq!(plan["derivative_result"], result.record().result_identity);
    assert_eq!(plan["artifact_digest"], receipt.artifact_digest);
    assert_eq!(
        plan["binding_id"],
        receipt.operation_binding.as_ref().unwrap().binding_id
    );
    assert_eq!(plan["generation"], receipt.generation);
    for owned in &receipt.resources {
        assert_eq!(
            settings
                .stable_file_state(&owned.resource)
                .unwrap()
                .unwrap()
                .sha256,
            owned.post_digest
        );
    }
    let payload = store::payload_relative(launcher::LauncherFlavour::NATIVE, &artifact.digest);
    assert_eq!(
        settings.read_file(&payload).unwrap().unwrap(),
        result.binding().local().files[0].bytes
    );
    eprintln!(
        "M10D2_ORDINARY_JOIN {}",
        serde_json::json!({
            "result": result.record().result_identity, "artifact_id": artifact.id,
            "artifact_digest": artifact.digest, "binding_id": plan["binding_id"],
            "plan_sha256": retained["plan_sha256"], "receipt_sha256": finalized["receipt_sha256"],
            "run_id": run_id, "generation": receipt.generation,
            "observed_resources": receipt.resources, "payload_executed": false,
        })
    );
}

#[test]
fn ordinary_host_derivative_submission_updates_the_same_bound_owner() {
    use crate::retained_mechanism::{
        DeployInspection, FixedDeployHostAdmission, submit_fixed_bound_vibe_bin,
    };
    use crate::{LifecycleLease, SubmissionReservations, SubmissionScope};
    let fixture = Fixture::new("#!/bin/sh\nprintf FIRST_DO_NOT_EXECUTE\n");
    let first = result(&fixture, "upstream.exe");
    let sources = Sources::new(&first);
    let targets = authored(&fixture);
    let selected = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state);
    let lease = LifecycleLease::acquire(execution.project_root).unwrap();
    let scope = SubmissionScope {
        context: "fixture-host".into(),
        target: "local-result".into(),
        principal: "author".into(),
    };
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let mut current = || Ok(Some("g1".into()));
    let permit = FixedDeployHostAdmission::issue_from_trusted_host(
        &lease,
        &namespace,
        &scope,
        "first",
        &execution,
        public_facts(&execution),
        &mut current,
    )
    .unwrap()
    .with_derivative(&sources.entry, &sources.base, &sources.local)
    .unwrap();
    let observed = submit_fixed_bound_vibe_bin(permit, "first", &mut current).unwrap();
    let DeployInspection::Completed {
        run_id,
        result: completed,
    } = observed
    else {
        panic!("{observed:?}")
    };
    ordinary_join(&execution, &run_id, &first, &completed.receipt);
    assert_eq!(completed.receipt.generation, 0);
    let first_receipt = completed.receipt;
    std::fs::write(execution.settings_root.join("bin/FOREIGN"), "neighbor").unwrap();

    fixture.rebuild("#!/bin/sh\nprintf SECOND_DO_NOT_EXECUTE\n");
    let second = result(&fixture, "upstream.exe");
    let next = Sources::new(&second);
    assert_ne!(
        first.record().result_identity,
        second.record().result_identity
    );
    let permit = FixedDeployHostAdmission::issue_from_trusted_host(
        &lease,
        &namespace,
        &scope,
        "second",
        &execution,
        public_facts(&execution),
        &mut current,
    )
    .unwrap()
    .with_derivative(&next.entry, &next.base, &next.local)
    .unwrap();
    let observed = submit_fixed_bound_vibe_bin(permit, "second", &mut current).unwrap();
    let DeployInspection::Completed {
        run_id,
        result: completed,
    } = observed
    else {
        panic!("{observed:?}")
    };
    ordinary_join(&execution, &run_id, &second, &completed.receipt);
    assert_eq!(completed.receipt.generation, 1);
    assert_ne!(
        completed.receipt.artifact_digest,
        first_receipt.artifact_digest
    );
    assert_eq!(
        completed.receipt.operation_binding,
        first_receipt.operation_binding
    );
    assert_eq!(
        std::fs::read(execution.settings_root.join("bin/FOREIGN")).unwrap(),
        b"neighbor"
    );
    let binding = completed.receipt.operation_binding.as_ref().unwrap();
    let home = DeploymentHome::new_bound(
        execution.state_home,
        execution.project,
        None,
        "local-helper",
        binding,
    )
    .unwrap();
    let settings = Project::open(execution.settings_root).unwrap();
    let view = DeployStateView::open_bound(&settings).unwrap();
    assert_eq!(
        view.read_receipt(&home).unwrap().unwrap(),
        completed.receipt
    );
    assert_eq!(
        view.read_bound_generation(&home, 0).unwrap().unwrap(),
        first_receipt
    );
    assert!(view.read_intent(&home).unwrap().is_none());
    use crate::retained_mechanism::{
        FixedDeployCleanupPlan, ReadOnlyDeployAdmission, cleanup_fixed_bound_vibe_bin,
        inspect_bound_vibe_bin_deploy,
    };
    // Missing authored descriptor AND selected provider after the effect.
    std::fs::remove_file(execution.project_root.join("vibe.toml")).unwrap();
    std::fs::remove_file(execution.project_root.join("target/debug/helper.exe")).unwrap();
    let read = ReadOnlyDeployAdmission::reissue(public_facts(&execution)).unwrap();
    let retained =
        inspect_bound_vibe_bin_deploy(&lease, namespace.id(), &scope, "second", &read).unwrap();
    assert!(matches!(retained, DeployInspection::Completed { .. }));
    let history =
        inspect_bound_vibe_bin_deploy(&lease, namespace.id(), &scope, "first", &read).unwrap();
    assert!(
        matches!(history, DeployInspection::Completed { result, .. } if result.receipt == first_receipt)
    );
    assert!(
        FixedDeployHostAdmission::issue_from_trusted_host(
            &lease,
            &namespace,
            &scope,
            "revoked-forward",
            &execution,
            public_facts(&execution),
            &mut current,
        )
        .is_err()
    );
    let cleanup = FixedDeployCleanupPlan::prepare_from_trusted_host(
        &lease,
        &namespace,
        &scope,
        "second",
        public_facts(&execution),
    )
    .unwrap();
    assert_eq!(cleanup.receipt(), &completed.receipt);
    let mut foreign = public_facts(&execution);
    foreign.principal_ref = "foreign";
    assert!(
        cleanup
            .approve_from_trusted_host(&lease, foreign, &mut current)
            .is_err()
    );
    let cleanup = FixedDeployCleanupPlan::prepare_from_trusted_host(
        &lease,
        &namespace,
        &scope,
        "second",
        public_facts(&execution),
    )
    .unwrap()
    .approve_from_trusted_host(&lease, public_facts(&execution), &mut current)
    .unwrap();
    let removed = cleanup_fixed_bound_vibe_bin(cleanup, &mut current).unwrap();
    assert_eq!(removed.len(), 2);
    assert!(
        !execution
            .settings_root
            .join("bin/local-helper.current")
            .exists()
    );
    assert_eq!(
        std::fs::read(execution.settings_root.join("bin/FOREIGN")).unwrap(),
        b"neighbor"
    );
    let current = view.read_receipt(&home).unwrap().unwrap();
    assert!(current.resources.is_empty());
    assert_eq!(
        current.status,
        vibe_wire::generated::deploy_receipt::ReceiptStatus::RolledBack
    );
    assert_eq!(
        view.read_bound_generation(&home, 1).unwrap().unwrap(),
        completed.receipt
    );
    let final_observation =
        inspect_bound_vibe_bin_deploy(&lease, namespace.id(), &scope, "second", &read).unwrap();
    assert!(
        matches!(final_observation, DeployInspection::Completed { result, .. }
        if result.receipt == current)
    );
}

#[test]
fn ordinary_host_interruption_recovers_same_generation_and_refuses_third_digest() {
    use crate::retained_mechanism::{
        DeployInspection, FixedDeployHostAdmission, submit_fixed_bound_vibe_bin,
    };
    use crate::{LifecycleLease, SubmissionReservations, SubmissionScope};
    // Actual destination publication, receipt publication and sidecar promotion
    // are three separate retained crash windows, all driven through the host.
    for window in [6, 8, 9, 10] {
        let fixture = Fixture::new("#!/bin/sh\nprintf RECOVER_DO_NOT_EXECUTE\n");
        let result = result(&fixture, "upstream.exe");
        let sources = Sources::new(&result);
        let targets = authored(&fixture);
        let selected = selection("local", &["local-helper"]);
        let state = fixture.state_home();
        let execution = fixture.execution(&targets, &selected, &state);
        let lease = LifecycleLease::acquire(execution.project_root).unwrap();
        let scope = SubmissionScope {
            context: "fixture-host".into(),
            target: "local-result".into(),
            principal: "author".into(),
        };
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let mut current = || Ok(Some("g1".into()));
        let permit = FixedDeployHostAdmission::issue_from_trusted_host(
            &lease,
            &namespace,
            &scope,
            "interrupted",
            &execution,
            public_facts(&execution),
            &mut current,
        )
        .unwrap()
        .with_derivative(&sources.entry, &sources.base, &sources.local)
        .unwrap();
        let mut observations = 0;
        let observed = submit_fixed_bound_vibe_bin(permit, "interrupted", &mut || {
            observations += 1;
            if observations == window {
                sources
                    .local
                    .write_atomic(
                        "target/debug/helper.exe",
                        b"source changed during publication",
                    )
                    .unwrap();
            }
            Ok(Some("g1".into()))
        })
        .unwrap();
        assert!(
            matches!(observed, DeployInspection::Uncertain { .. }),
            "window {window}: {observed:?}"
        );
        assert!(
            execution
                .settings_root
                .join("bin/local-helper.current")
                .exists()
        );
        sources
            .local
            .write_atomic(
                "target/debug/helper.exe",
                &result.binding().local().files[0].bytes,
            )
            .unwrap();
        if window == 6 {
            let pointer = execution.settings_root.join("bin/local-helper.current");
            let owned = std::fs::read(&pointer).unwrap();
            std::fs::write(&pointer, "foreign third digest").unwrap();
            let permit = FixedDeployHostAdmission::issue_from_trusted_host(
                &lease,
                &namespace,
                &scope,
                "third",
                &execution,
                public_facts(&execution),
                &mut current,
            )
            .unwrap()
            .with_derivative(&sources.entry, &sources.base, &sources.local)
            .unwrap();
            assert!(submit_fixed_bound_vibe_bin(permit, "third", &mut current).is_err());
            assert_eq!(std::fs::read(&pointer).unwrap(), b"foreign third digest");
            std::fs::write(&pointer, owned).unwrap();
        }
        let mut recovery_execution = fixture.execution(&targets, &selected, &state);
        recovery_execution.created_at = "2026-10-19T00:00:00Z";
        let permit = FixedDeployHostAdmission::issue_from_trusted_host(
            &lease,
            &namespace,
            &scope,
            "recovery",
            &recovery_execution,
            public_facts(&recovery_execution),
            &mut current,
        )
        .unwrap()
        .with_derivative(&sources.entry, &sources.base, &sources.local)
        .unwrap();
        let observed = submit_fixed_bound_vibe_bin(permit, "recovery", &mut current).unwrap();
        let DeployInspection::Completed {
            result: completed, ..
        } = observed
        else {
            panic!("window {window}: {observed:?}")
        };
        assert_eq!(completed.receipt.generation, 0);
        let binding = completed.receipt.operation_binding.as_ref().unwrap();
        let home = DeploymentHome::new_bound(
            execution.state_home,
            execution.project,
            None,
            "local-helper",
            binding,
        )
        .unwrap();
        let settings = Project::open(execution.settings_root).unwrap();
        assert!(
            DeployStateView::open_bound(&settings)
                .unwrap()
                .read_intent(&home)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn ordinary_host_retained_cleanup_interruption_and_drift_keep_exact_ownership() {
    use crate::retained_mechanism::{
        DeployInspection, FixedDeployCleanupPlan, FixedDeployHostAdmission,
        cleanup_fixed_bound_vibe_bin, submit_fixed_bound_vibe_bin,
    };
    use crate::{LifecycleLease, SubmissionReservations, SubmissionScope};
    for window in [4, 5, 6, 8] {
        let fixture = Fixture::new("#!/bin/sh\nprintf CLEANUP_DO_NOT_EXECUTE\n");
        let result = result(&fixture, "upstream.exe");
        let sources = Sources::new(&result);
        let targets = authored(&fixture);
        let selected = selection("local", &["local-helper"]);
        let state_home = fixture.state_home();
        let execution = fixture.execution(&targets, &selected, &state_home);
        let lease = LifecycleLease::acquire(execution.project_root).unwrap();
        let scope = SubmissionScope {
            context: "fixture-host".into(),
            target: "local-result".into(),
            principal: "author".into(),
        };
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let mut current = || Ok(Some("g1".into()));
        let permit = FixedDeployHostAdmission::issue_from_trusted_host(
            &lease,
            &namespace,
            &scope,
            "installed",
            &execution,
            public_facts(&execution),
            &mut current,
        )
        .unwrap()
        .with_derivative(&sources.entry, &sources.base, &sources.local)
        .unwrap();
        let observed = submit_fixed_bound_vibe_bin(permit, "installed", &mut current).unwrap();
        let DeployInspection::Completed {
            result: installed, ..
        } = observed
        else {
            panic!("{observed:?}")
        };
        let binding = installed.receipt.operation_binding.as_ref().unwrap();
        let home = DeploymentHome::new_bound(
            execution.state_home,
            execution.project,
            None,
            "local-helper",
            binding,
        )
        .unwrap();
        let settings = Project::open(execution.settings_root).unwrap();
        let state = crate::mechanism::deploy::state::DeployState::open_bound(&settings).unwrap();
        std::fs::write(execution.settings_root.join("bin/FOREIGN"), "neighbor").unwrap();
        std::fs::remove_file(execution.project_root.join("vibe.toml")).unwrap();
        let cleanup = FixedDeployCleanupPlan::prepare_from_trusted_host(
            &lease,
            &namespace,
            &scope,
            "installed",
            public_facts(&execution),
        )
        .unwrap()
        .approve_from_trusted_host(&lease, public_facts(&execution), &mut current)
        .unwrap();
        let mut observations = 0;
        assert!(
            cleanup_fixed_bound_vibe_bin(cleanup, &mut || {
                observations += 1;
                Ok(Some(
                    if observations == window {
                        "revoked-generation"
                    } else {
                        "g1"
                    }
                    .into(),
                ))
            })
            .is_err()
        );
        assert_eq!(
            state.read_inverse(&home).unwrap().is_some(),
            window != 8,
            "window {window}"
        );
        let read =
            crate::retained_mechanism::ReadOnlyDeployAdmission::reissue(public_facts(&execution))
                .unwrap();
        let observed = crate::retained_mechanism::inspect_bound_vibe_bin_deploy(
            &lease,
            namespace.id(),
            &scope,
            "installed",
            &read,
        )
        .unwrap();
        assert!(
            matches!(observed, DeployInspection::Uncertain { ref partial, .. }
            if if window == 8 { partial.intent == "absent" && partial.sidecar == "absent" }
                else { partial.intent.contains("retained inverse present") }),
            "window {window}: {observed:?}"
        );
        assert!(
            !execution
                .settings_root
                .join("bin/local-helper.current")
                .exists()
        );
        if window == 4 {
            let pointer = execution.settings_root.join("bin/local-helper.current");
            std::fs::write(&pointer, b"foreign third digest").unwrap();
            let cleanup = FixedDeployCleanupPlan::prepare_from_trusted_host(
                &lease,
                &namespace,
                &scope,
                "installed",
                public_facts(&execution),
            )
            .unwrap()
            .approve_from_trusted_host(&lease, public_facts(&execution), &mut current)
            .unwrap();
            assert!(cleanup_fixed_bound_vibe_bin(cleanup, &mut current).is_err());
            assert_eq!(std::fs::read(&pointer).unwrap(), b"foreign third digest");
            // The fixture owner removes its injected foreign edit; the engine
            // never guesses a rollback or overwrites a third digest.
            std::fs::remove_file(pointer).unwrap();
        }
        let cleanup = FixedDeployCleanupPlan::prepare_from_trusted_host(
            &lease,
            &namespace,
            &scope,
            "installed",
            public_facts(&execution),
        )
        .unwrap()
        .approve_from_trusted_host(&lease, public_facts(&execution), &mut current)
        .unwrap();
        cleanup_fixed_bound_vibe_bin(cleanup, &mut current).unwrap();
        assert!(state.read_inverse(&home).unwrap().is_none());
        assert!(state.read_lock_resources(&home).unwrap().is_none());
        let observed = crate::retained_mechanism::inspect_bound_vibe_bin_deploy(
            &lease,
            namespace.id(),
            &scope,
            "installed",
            &read,
        )
        .unwrap();
        assert!(
            matches!(observed, DeployInspection::Completed { result, .. }
            if result.receipt.status == vibe_wire::generated::deploy_receipt::ReceiptStatus::RolledBack
                && result.receipt.resources.is_empty())
        );
        assert!(
            state
                .read_receipt(&home)
                .unwrap()
                .unwrap()
                .resources
                .is_empty()
        );
        assert_eq!(
            std::fs::read(execution.settings_root.join("bin/FOREIGN")).unwrap(),
            b"neighbor"
        );
    }
}

#[test]
fn ordinary_host_changed_receipt_refuses_forward_and_prepared_cleanup_without_effect() {
    use crate::retained_mechanism::{
        DeployInspection, FixedDeployCleanupPlan, FixedDeployHostAdmission,
        cleanup_fixed_bound_vibe_bin, submit_fixed_bound_vibe_bin,
    };
    use crate::{LifecycleLease, SubmissionReservations, SubmissionScope};
    let fixture = Fixture::new("#!/bin/sh\nprintf RECEIPT_DO_NOT_EXECUTE\n");
    let result = result(&fixture, "upstream.exe");
    let sources = Sources::new(&result);
    let targets = authored(&fixture);
    let selected = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state_home);
    let lease = LifecycleLease::acquire(execution.project_root).unwrap();
    let scope = SubmissionScope {
        context: "fixture-host".into(),
        target: "local-result".into(),
        principal: "author".into(),
    };
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let mut current = || Ok(Some("g1".into()));
    let permit = FixedDeployHostAdmission::issue_from_trusted_host(
        &lease,
        &namespace,
        &scope,
        "installed",
        &execution,
        public_facts(&execution),
        &mut current,
    )
    .unwrap()
    .with_derivative(&sources.entry, &sources.base, &sources.local)
    .unwrap();
    let observed = submit_fixed_bound_vibe_bin(permit, "installed", &mut current).unwrap();
    let DeployInspection::Completed {
        result: installed, ..
    } = observed
    else {
        panic!("{observed:?}")
    };
    let cleanup = FixedDeployCleanupPlan::prepare_from_trusted_host(
        &lease,
        &namespace,
        &scope,
        "installed",
        public_facts(&execution),
    )
    .unwrap();
    let binding = installed.receipt.operation_binding.as_ref().unwrap();
    let home = DeploymentHome::new_bound(
        execution.state_home,
        execution.project,
        None,
        "local-helper",
        binding,
    )
    .unwrap();
    let settings = Project::open(execution.settings_root).unwrap();
    let state = crate::mechanism::deploy::state::DeployState::open_bound(&settings).unwrap();
    // Mutate an actual engine-produced receipt through its incumbent writer;
    // no handwritten receipt or approval stands in for successful execution.
    let mut changed = installed.receipt.clone();
    changed.desired_config_digest = "e".repeat(64);
    state.write_receipt(&home, &changed).unwrap();
    let pointer = execution.settings_root.join("bin/local-helper.current");
    let before = std::fs::read(&pointer).unwrap();
    let permit = FixedDeployHostAdmission::issue_from_trusted_host(
        &lease,
        &namespace,
        &scope,
        "changed",
        &execution,
        public_facts(&execution),
        &mut current,
    )
    .unwrap()
    .with_derivative(&sources.entry, &sources.base, &sources.local)
    .unwrap();
    assert!(submit_fixed_bound_vibe_bin(permit, "changed", &mut current).is_err());
    let cleanup = cleanup
        .approve_from_trusted_host(&lease, public_facts(&execution), &mut current)
        .unwrap();
    assert!(cleanup_fixed_bound_vibe_bin(cleanup, &mut current).is_err());
    assert_eq!(std::fs::read(&pointer).unwrap(), before);
    assert_eq!(state.read_receipt(&home).unwrap().unwrap(), changed);
    assert!(state.read_intent(&home).unwrap().is_none());
    assert!(state.read_inverse(&home).unwrap().is_none());
}
