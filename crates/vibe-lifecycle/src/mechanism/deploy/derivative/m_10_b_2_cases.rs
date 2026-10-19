use super::*;
use crate::mechanism::deploy::binding::TrustedLocalDeployFacts;
use crate::mechanism::deploy::support::{Fixture, selection};
use vibe_core::manifest::derivation::{DerivationBinding, DerivationRecipe};
use vibe_wire::generated::shared::{
    EnvironmentOperationBindingTargetMode as Mode, EnvironmentOperationBindingTargetOs as Os,
    EnvironmentOperationBindingTargetRole as Role,
};
use vibe_workspace::derivatives::compatibility::{CreationState, ExportInterface};
use vibe_workspace::{
    derivatives::{SourceInput, prepare_binding},
    source_query::CapturedSourceFile,
};

pub(super) fn result(fixture: &Fixture, base_resource: &str) -> BoundDerivative {
    let upstream = Fixture::new("sealed original bytes");
    let source = |fixture: &Fixture, name: &str| {
        SourceInput::new(
        vibe_core::PackageRef::parse(&format!("tool:org.example/{name}@=1.0.0")).unwrap(),
        toml::from_str("file = 'target/debug/helper.exe'\nmedia = 'application/octet-stream'\ninterpretation = 'opaque'\nrole = 'resource'\nloading = 'explicit'").unwrap(),
        vec![CapturedSourceFile::new("target/debug/helper.exe".into(), vibe_safefs::Project::open(fixture.project.path()).unwrap().read_file_snapshot_bounded("target/debug/helper.exe", 4096).unwrap().unwrap()).unwrap()],
    ).unwrap()
    };
    let base = source(&upstream, "upstream");
    let local = source(fixture, "local");
    let retained = base.retained_source(base_resource);
    let binding = DerivationBinding::new(
        retained.clone(),
        local.retained_source("helper.exe"),
        retained.observation_id().unwrap(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    let result = prepare_binding(&toml::to_string(&binding).unwrap(), Some(&base))
        .unwrap()
        .value;
    assert_eq!(
        std::fs::read(upstream.project.path().join("target/debug/helper.exe")).unwrap(),
        b"sealed original bytes"
    );
    result
}

pub(super) fn host(execution: &DeployExecution<'_>) -> AdmittedLocalDeployBinding {
    AdmittedLocalDeployBinding::issue(TrustedLocalDeployFacts {
        host_context_id: "fixture-host",
        instance_ref: "local-result",
        selected_generation_ref: Some("g1"),
        principal_ref: "author",
        admission_epoch: 1,
        binding_nonce: "fixture-nonce",
        allow_alias_resolution: false,
        target_mode: Mode::UserProfile,
        target_role: Role::ProducedToolRuntime,
        target_os: match std::env::consts::OS {
            "linux" => Os::Linux,
            "macos" => Os::Macos,
            "windows" => Os::Windows,
            other => panic!("unsupported {other}"),
        },
        target_architecture: std::env::consts::ARCH,
        target_abi: None,
        target_libc: None,
        project_root: execution.project_root,
        settings_root: execution.settings_root,
        user_home: execution.user_home,
    })
    .unwrap()
}
pub(super) fn target() -> DeployTarget {
    crate::mechanism::vibebin::support::target(
        "local-helper",
        "helper.exe",
        Some("command = 'local-helper'"),
    )
}
pub(super) fn export() -> ExportTarget {
    ExportTarget {
        name: "executable-file".into(),
        interface: ExportInterface::Files(vec!["target/debug/helper.exe".into()]),
    }
}

#[test]
fn compatible_activation_needs_own_typed_grant_and_stops_before_payload() {
    let fixture = Fixture::new("local result bytes");
    let result = result(&fixture, "upstream.exe");
    let targets = [target()];
    let selected = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state);
    let admission = host(&execution);
    let binding = admission.binding().clone();
    let passive = decide(&result, Some(&export()), None, None);
    assert_eq!(passive.activation, ActivationState::NotRequested);
    let refused = decide(
        &result,
        Some(&export()),
        Some((&execution, &targets[0], &binding, "author", Some("g1"))),
        None,
    );
    assert!(matches!(refused.activation, ActivationState::Refused(_)));
    assert_eq!(
        refused.compatibility.compatibility,
        Compatibility::Compatible
    );
    assert_eq!(refused.compatibility.creation, CreationState::Created);
    let grant = DerivativeAdmission::bind(&result, &execution, &targets[0], admission).unwrap();
    let admitted = decide(
        &result,
        Some(&export()),
        Some((&execution, &targets[0], &binding, "author", Some("g1"))),
        Some(grant),
    );
    assert_eq!(admitted.activation, ActivationState::AdmittedAtFixtureStop);
    assert!(!state.exists());
    assert_eq!(
        std::fs::read(fixture.project.path().join("target/debug/helper.exe")).unwrap(),
        b"local result bytes"
    );
    // Lock infrastructure may exist; no executable/launcher was published.
    assert!(!fixture.settings.path().join("bin/local-helper").exists());
}

#[test]
fn changed_routes_refuse_unavailable_disabled_and_native_providers_before_locks() {
    use crate::mechanism::package::support::{PLUGIN_PIN, pin, registry, world_with_plugin};
    use vibe_core::manifest::{ExtensionKey, MechanismRole, MechanismRoutes};

    let fixture = Fixture::new("result");
    let result = result(&fixture, "upstream.exe");
    let targets = [target()];
    let selected = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state);
    let mut world = world_with_plugin();
    world.installed[0].mechanisms[0].role = MechanismRole::Deploy;
    world.installed[0].mechanisms[0].name = "vibe-bin".into();
    let native = registry(&world);
    world
        .host
        .controls
        .disable
        .push(ExtensionKey::authored(PLUGIN_PIN));
    let disabled = registry(&world);
    for (current_registry, routed, reason) in [
        (
            &fixture.registry,
            "org.example/missing#deploy",
            "not installed",
        ),
        (&disabled, PLUGIN_PIN, "disabled"),
        (&native, PLUGIN_PIN, "provider selection changed"),
    ] {
        let admission = host(&execution);
        let binding = admission.binding().clone();
        let grant = DerivativeAdmission::bind(&result, &execution, &targets[0], admission).unwrap();
        let mut routes = MechanismRoutes::default();
        routes.insert(targets[0].mechanism.clone(), pin(routed));
        let mut changed = execution;
        changed.registry = current_registry;
        changed.routes = &routes;
        let decision = decide(
            &result,
            Some(&export()),
            Some((&changed, &targets[0], &binding, "author", Some("g1"))),
            Some(grant),
        );
        assert!(
            matches!(&decision.activation, ActivationState::Refused(message) if message.contains(reason)),
            "{decision:?}"
        );
        assert_eq!(
            decision.compatibility.compatibility,
            Compatibility::Compatible
        );
        assert_eq!(decision.compatibility.creation, CreationState::Created);
        assert_eq!(result.binding().local().files[0].bytes, b"result");
        assert_eq!(
            std::fs::read(fixture.project.path().join("target/debug/helper.exe")).unwrap(),
            b"result"
        );
        assert!(!state.exists());
        assert_eq!(
            std::fs::read_dir(fixture.settings.path()).unwrap().count(),
            0
        );
    }
}

#[test]
fn upstream_record_installation_and_old_result_grant_do_not_authorize_changed_bytes() {
    let fixture = Fixture::new("old result");
    let old = result(&fixture, "upstream.exe");
    let targets = [target()];
    let selected = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state);
    let admission = host(&execution);
    let binding = admission.binding().clone();
    let grant = DerivativeAdmission::bind(&old, &execution, &targets[0], admission).unwrap();
    fixture.rebuild("changed result");
    let changed = result(&fixture, "upstream.exe");
    assert!(matches!(
        decide(
            &changed,
            Some(&export()),
            Some((&execution, &targets[0], &binding, "author", Some("g1"))),
            Some(grant)
        )
        .activation,
        ActivationState::Refused(_)
    ));
    // Even a fresh host carrier cannot promote an upstream artifact identity.
    let reused = result(&fixture, "helper.exe");
    assert!(DerivativeAdmission::bind(&reused, &execution, &targets[0], host(&execution)).is_err());
    // A stale engine record cannot become a new result proof.
    std::fs::write(
        fixture.project.path().join("target/debug/helper.exe"),
        b"unrecorded changed bytes",
    )
    .unwrap();
    assert!(
        DerivativeAdmission::bind(&changed, &execution, &targets[0], host(&execution)).is_err()
    );
    assert!(!state.exists());
    assert_eq!(changed.binding().local().files[0].bytes, b"changed result");
}

#[test]
fn incompatible_unknown_descriptor_and_host_drift_refuse_without_losing_creation() {
    let fixture = Fixture::new("result");
    let result = result(&fixture, "upstream.exe");
    let targets = [target()];
    let selected = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state);
    for (principal, generation, change_descriptor, bad_export) in [
        ("foreign", "g1", false, false),
        ("author", "g2", false, false),
        ("author", "g1", true, false),
        ("author", "g1", false, true),
    ] {
        let admission = host(&execution);
        let binding = admission.binding().clone();
        let grant = DerivativeAdmission::bind(&result, &execution, &targets[0], admission).unwrap();
        let mut descriptor = targets[0].clone();
        if change_descriptor {
            descriptor.id = "other-target".into();
        }
        let mut export = export();
        if bad_export {
            export.interface = ExportInterface::Files(vec!["missing".into()]);
        }
        let decision = decide(
            &result,
            Some(&export),
            Some((
                &execution,
                &descriptor,
                &binding,
                principal,
                Some(generation),
            )),
            Some(grant),
        );
        assert!(matches!(decision.activation, ActivationState::Refused(_)));
        assert_eq!(decision.compatibility.creation, CreationState::Created);
    }
    let admission = host(&execution);
    let binding = admission.binding().clone();
    let grant = DerivativeAdmission::bind(&result, &execution, &targets[0], admission).unwrap();
    let decision = decide(
        &result,
        None,
        Some((&execution, &targets[0], &binding, "author", Some("g1"))),
        Some(grant),
    );
    assert!(matches!(
        decision.compatibility.compatibility,
        Compatibility::Unknown(_)
    ));
    assert!(matches!(decision.activation, ActivationState::Refused(_)));
    assert!(!state.exists());
    assert!(!fixture.settings.path().join("bin").exists());
}

#[test]
fn copied_binding_changed_root_and_post_grant_artifact_drift_refuse() {
    let fixture = Fixture::new("result");
    let other = Fixture::new("result");
    let result = result(&fixture, "upstream.exe");
    let targets = [target()];
    let selected = selection("local", &["local-helper"]);
    let state = fixture.state_home();
    let execution = fixture.execution(&targets, &selected, &state);
    for copied_binding in [true, false] {
        let admission = host(&execution);
        let mut binding = admission.binding().clone();
        let grant = DerivativeAdmission::bind(&result, &execution, &targets[0], admission).unwrap();
        let mut changed = execution;
        if copied_binding {
            binding.instance_ref = "foreign-instance".into();
        } else {
            changed.settings_root = other.settings.path();
        }
        let decision = decide(
            &result,
            Some(&export()),
            Some((&changed, &targets[0], &binding, "author", Some("g1"))),
            Some(grant),
        );
        assert!(matches!(decision.activation, ActivationState::Refused(_)));
        assert!(!fixture.settings.path().join("bin").exists());
        assert!(!other.settings.path().join("bin").exists());
    }
    let admission = host(&execution);
    let binding = admission.binding().clone();
    let grant = DerivativeAdmission::bind(&result, &execution, &targets[0], admission).unwrap();
    fixture.rebuild("new bytes after grant");
    let decision = decide(
        &result,
        Some(&export()),
        Some((&execution, &targets[0], &binding, "author", Some("g1"))),
        Some(grant),
    );
    assert!(matches!(decision.activation, ActivationState::Refused(_)));
    assert_eq!(result.binding().local().files[0].bytes, b"result");
    assert!(!state.exists());
    assert!(!fixture.settings.path().join("bin/local-helper").exists());
}
