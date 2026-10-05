use super::*;
use crate::ports::{PackageSource, PackageSourceBuild, PackageSourceFactory};
use std::fs;
use std::path::Path;
use vibe_core::PackageRef;
use vibe_lifecycle::process::{
    PackageTreeWitness, capture_package_tree_witness, recheck_package_tree_witness,
};

struct TestSource(PackageTreeWitness);
impl PackageSourceFactory for TestSource {
    fn build(&self, _: PackageSourceBuild<'_>) -> anyhow::Result<Box<dyn PackageSource>> {
        anyhow::bail!("grant tests never build an effectful source")
    }
}
impl BoundOnePackageSource for TestSource {
    fn source_witness(&self) -> &PackageTreeWitness {
        &self.0
    }
    fn recheck_source_binding(&self, witness: &PackageTreeWitness) -> anyhow::Result<()> {
        anyhow::ensure!(witness == &self.0, "foreign source witness");
        Ok(recheck_package_tree_witness(witness)?)
    }
}

fn facts(approved: bool) -> HostPolicyFacts {
    HostPolicyFacts {
        approve_captured_script: false,
        approve_exact_local_install: approved,
        allow_detach: false,
        allow_cancel_pre_dispatch: false,
        allow_cancel_deferred_boundary: false,
    }
}
fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "install-grant-fixture".into(),
        target: "physical-target".into(),
        principal: "trusted-principal".into(),
    }
}
fn source(registry: &Path) -> Arc<dyn BoundOnePackageSource> {
    let pin = PackageRef::parse("flow:org.example/fixture@=1.1.0").unwrap();
    let path = registry.join("org.example/fixture/v1.1.0");
    let witness = capture_package_tree_witness(&path, &pin, &["README.md", "vibe.toml"]).unwrap();
    Arc::new(TestSource(witness))
}
fn setup() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    let registry = dir.path().join("registry");
    let package = registry.join("org.example/fixture/v1.1.0");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&package).unwrap();
    fs::write(
        project.join("vibe.toml"),
        "[project]\nname = 'grant-fixture'\nversion = '1.1.0'\n",
    )
    .unwrap();
    fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup = 'org.example'\nname = 'fixture'\nkind = 'flow'\nversion = '1.1.0'\n",
    )
    .unwrap();
    fs::write(package.join("README.md"), b"# Fixture\n").unwrap();
    (dir, project, registry)
}
fn service(project: &Path, registry: &Path, approved: bool) -> ExactLocalInstallGrantService {
    ExactLocalInstallGrantService::from_trusted_host(
        Arc::new(Project::open(project).unwrap()),
        scope(),
        TargetOs::Linux,
        TrustedPolicySource::from_host_identifier("trusted-install-policy".into()).unwrap(),
        facts(approved),
        source(registry),
    )
    .unwrap()
}
fn index(project: &Path) -> Vec<u8> {
    fs::read(project.join(".vibe/machine-submissions.json")).unwrap()
}

#[test]
fn issue_and_new_instance_reattach_preserve_exact_binding_without_new_c1() {
    let (_dir, project, registry) = setup();
    let first = service(&project, &registry, true);
    let grant = first.issue(60_000).unwrap();
    assert_eq!(grant.host().readonly_scope(), &scope());
    let namespace = grant.namespace().id().to_owned();
    let pointer = fs::read(project.join(&first.path)).unwrap();
    let before = index(&project);
    drop(grant);
    drop(first);
    let restarted = service(&project, &registry, true);
    let attached = restarted.reattach().unwrap();
    assert_eq!(attached.namespace().id(), namespace);
    assert_eq!(attached.host().readonly_scope(), &scope());
    assert_eq!(fs::read(project.join(&restarted.path)).unwrap(), pointer);
    assert_eq!(index(&project), before);
    assert!(matches!(
        restarted.issue(60_000),
        Err(ExactLocalInstallGrantError::AlreadyIssued)
    ));
}

#[test]
fn changed_source_or_physical_project_root_refuses_without_new_c1() {
    let (_dir, project, registry) = setup();
    let original = service(&project, &registry, true);
    original.issue(60_000).unwrap();
    let before = index(&project);
    fs::write(
        registry.join("org.example/fixture/v1.1.0/README.md"),
        b"# Changed\n",
    )
    .unwrap();
    let changed = service(&project, &registry, true);
    assert!(matches!(
        changed.reattach(),
        Err(ExactLocalInstallGrantError::ForeignBinding)
    ));
    assert!(matches!(
        changed.issue(60_000),
        Err(ExactLocalInstallGrantError::ForeignBinding)
    ));
    assert_eq!(index(&project), before);

    let foreign = tempfile::tempdir().unwrap();
    fs::write(
        foreign.path().join("vibe.toml"),
        "[project]\nname = 'other'\nversion = '1.1.0'\n",
    )
    .unwrap();
    let new_root = service(foreign.path(), &registry, true);
    fs::create_dir_all(foreign.path().join(".vibe/exact-local-install-grants")).unwrap();
    fs::copy(
        project.join(&original.path),
        foreign.path().join(&new_root.path),
    )
    .unwrap();
    assert!(matches!(
        new_root.reattach(),
        Err(ExactLocalInstallGrantError::ForeignBinding)
    ));
    assert!(
        !foreign
            .path()
            .join(".vibe/machine-submissions.json")
            .exists()
    );
}

#[test]
fn retained_source_port_can_reattach_for_cold_inspection_after_content_drift() {
    let (_dir, project, registry) = setup();
    let retained = service(&project, &registry, true);
    let first = retained.issue(60_000).unwrap();
    let namespace = first.namespace().id().to_owned();
    let before = index(&project);
    fs::write(
        registry.join("org.example/fixture/v1.1.0/README.md"),
        b"# Drifted\n",
    )
    .unwrap();
    let attached = retained.reattach().unwrap();
    assert_eq!(attached.namespace().id(), namespace);
    assert_eq!(index(&project), before);
    assert!(matches!(
        retained.check_source(),
        Err(ExactLocalInstallGrantError::SourceUnavailable(_))
    ));
}

#[test]
fn changed_policy_generation_source_or_install_bit_refuses_reconnect() {
    let (_dir, project, registry) = setup();
    let original = service(&project, &registry, true);
    original.issue(60_000).unwrap();
    let before = index(&project);
    let owner =
        PolicyOwner::attach_readonly(Arc::new(Project::open(&project).unwrap()), scope()).unwrap();
    let lease = LifecycleLease::acquire(&project).unwrap();
    let current = owner.current_under_lease(&lease).unwrap();
    let next = owner
        .update_under_lease(
            &lease,
            current.generation(),
            TrustedPolicySource::from_host_identifier("trusted-install-policy".into()).unwrap(),
            facts(true),
        )
        .unwrap();
    assert_ne!(current.generation(), next.generation());
    drop(lease);
    assert!(matches!(
        original.reattach(),
        Err(ExactLocalInstallGrantError::Policy(
            PolicyError::StartupDisagreement
        ))
    ));
    let lease = LifecycleLease::acquire(&project).unwrap();
    let newer = owner
        .update_under_lease(
            &lease,
            next.generation(),
            TrustedPolicySource::from_host_identifier("different-policy".into()).unwrap(),
            facts(false),
        )
        .unwrap();
    assert!(!newer.facts().approve_exact_local_install);
    drop(lease);
    assert!(matches!(
        original.reattach(),
        Err(ExactLocalInstallGrantError::Policy(
            PolicyError::StartupDisagreement
        ))
    ));
    assert_eq!(index(&project), before);
}

#[test]
fn default_denied_bit_and_foreign_fact_claim_never_grant_approval() {
    let (_dir, project, registry) = setup();
    let denied = service(&project, &registry, false);
    let grant = denied.issue(60_000).unwrap();
    let options = crate::machine_operation::input_decision::RequestedOptions {
        force: false,
        trace: false,
        deploy_profile: None,
        control: crate::machine_operation::input_decision::RequestedOptionsControl::None,
    };
    assert!(matches!(
        grant.host().prepare_requested_options(options),
        Err(ExactLocalInstallDecisionError::PolicyDenied)
    ));
    assert!(matches!(
        service(&project, &registry, true).reattach(),
        Err(ExactLocalInstallGrantError::Policy(
            PolicyError::StartupDisagreement
        ))
    ));
}

#[test]
fn incomplete_fence_and_missing_pointer_do_not_reissue() {
    let (_dir, project, registry) = setup();
    let grant = service(&project, &registry, true);
    let lease = grant.lease().unwrap();
    grant
        .publish(&lease, &grant.pointer(GrantState::Issuing))
        .unwrap();
    drop(lease);
    assert!(matches!(
        grant.issue(60_000),
        Err(ExactLocalInstallGrantError::IssuanceIncomplete)
    ));
    assert!(matches!(
        grant.reattach(),
        Err(ExactLocalInstallGrantError::IssuanceIncomplete)
    ));
    assert!(!project.join(".vibe/machine-submissions.json").exists());
}
