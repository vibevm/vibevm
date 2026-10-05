use super::*;
use crate::machine_operation::input_decision::BodyApprovalAnswer;
use crate::machine_submission::{HostPolicyFacts, TrustedPolicySource};
use crate::ports::{PackageSource, PackageSourceBuild, PackageSourceFactory};
use anyhow::{Result, bail};
use vibe_core::PackageRef;
use vibe_lifecycle::LifecycleLease;
use vibe_lifecycle::process::{
    PackageTreeWitness, capture_package_tree_witness, recheck_package_tree_witness,
};

struct BoundFixture(PackageTreeWitness);
impl PackageSourceFactory for BoundFixture {
    fn build(&self, _: PackageSourceBuild<'_>) -> Result<Box<dyn PackageSource>> {
        bail!("decision preparation must never build a package source")
    }
}
impl BoundOnePackageSource for BoundFixture {
    fn source_witness(&self) -> &PackageTreeWitness {
        &self.0
    }
    fn recheck_source_binding(&self, witness: &PackageTreeWitness) -> Result<()> {
        anyhow::ensure!(witness == &self.0, "foreign source witness");
        Ok(recheck_package_tree_witness(witness)?)
    }
}

fn facts(install: bool) -> HostPolicyFacts {
    HostPolicyFacts {
        approve_captured_script: false,
        approve_exact_local_install: install,
        allow_detach: false,
        allow_cancel_pre_dispatch: false,
        allow_cancel_deferred_boundary: false,
    }
}
fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "local-install-context".into(),
        target: "root".into(),
        principal: "local-install-principal".into(),
    }
}
fn setup() -> (
    tempfile::TempDir,
    PathBuf,
    Arc<PolicyOwner>,
    Arc<dyn BoundOnePackageSource>,
) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("consumer");
    let source = dir.path().join("registry/org.example/fixture/v1.1.0");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(root.join("vibe.toml"),
        "[project]\nname = 'consumer'\nversion = '1.1.0'\n[requires.packages]\n'flow:org.example/fixture' = '=1.1.0'\n").unwrap();
    std::fs::write(
        source.join("vibe.toml"),
        "[package]\ngroup = 'org.example'\nname = 'fixture'\nkind = 'flow'\nversion = '1.1.0'\n",
    )
    .unwrap();
    std::fs::write(source.join("README.md"), b"# Fixture\n").unwrap();
    let pin = PackageRef::parse("flow:org.example/fixture@=1.1.0").unwrap();
    let witness = capture_package_tree_witness(&source, &pin, &["vibe.toml", "README.md"]).unwrap();
    let lease = LifecycleLease::acquire(&root).unwrap();
    let source_id =
        TrustedPolicySource::from_host_identifier("exact-local-test-policy".into()).unwrap();
    PolicyOwner::initialize_under_lease(&lease, &scope(), source_id, facts(true)).unwrap();
    drop(lease);
    let owner = Arc::new(
        PolicyOwner::attach_readonly(Arc::new(Project::open(&root).unwrap()), scope()).unwrap(),
    );
    (dir, root, owner, Arc::new(BoundFixture(witness)))
}
fn host(
    root: PathBuf,
    policy: Arc<PolicyOwner>,
    source: Arc<dyn BoundOnePackageSource>,
) -> ExactLocalInstallDecisionHost {
    ExactLocalInstallDecisionHost::from_host(root, policy, source, TargetOs::current().unwrap())
        .unwrap()
}
fn answer(question: &InputDecisionMessage, approved: bool) -> InputDecisionMessage {
    let mut answer = question.clone();
    answer.body = Body::ApprovalAnswer(Box::new(BodyApprovalAnswer { approved }));
    answer
}
fn no_options() -> RequestedOptions {
    RequestedOptions {
        force: false,
        trace: false,
        deploy_profile: None,
        control: RequestedOptionsControl::None,
    }
}

#[test]
fn exact_local_install_approval_is_typed_and_retained_without_submit() {
    let (_dir, root, policy, source) = setup();
    let host = host(root.clone(), policy, source);
    let question = host.prepare_requested_options(no_options()).unwrap();
    assert_eq!(question.operation_id.as_deref(), Some(OPERATION));
    assert_eq!(question.policy_generation.is_some(), true);
    assert!(matches!(question.body, Body::ApprovalQuestion(_)));
    let refusal = host.answer_typed(&answer(&question, false)).err().unwrap();
    assert_eq!(refusal, ExactLocalInstallDecisionError::PolicyDenied);
    let bytes = serde_json::to_vec(&answer(&question, true)).unwrap();
    let prepared = host.answer_bytes(&bytes).unwrap();
    assert_eq!(prepared.identity(), question.plan_id.as_deref().unwrap());
    assert_eq!(prepared.options(), &no_options());
    assert!(!root.join("vibe.lock").exists());
    assert!(!root.join(".vibe/lifecycle-outcomes").exists());
    let mut foreign = answer(&question, true);
    foreign.context_id = Some("foreign-context".into());
    assert_eq!(
        host.answer_typed(&foreign).err().unwrap(),
        ExactLocalInstallDecisionError::ForeignScope
    );
    for unsupported in [
        RequestedOptions {
            force: true,
            ..no_options()
        },
        RequestedOptions {
            trace: true,
            ..no_options()
        },
        RequestedOptions {
            deploy_profile: Some("local".into()),
            ..no_options()
        },
        RequestedOptions {
            control: RequestedOptionsControl::Detach,
            ..no_options()
        },
    ] {
        assert_eq!(
            host.prepare_requested_options(unsupported).err().unwrap(),
            ExactLocalInstallDecisionError::UnsupportedOptions
        );
    }
    let mut changed_plan = answer(&question, true);
    changed_plan.plan_id = Some("forged-plan".into());
    assert_eq!(
        host.answer_typed(&changed_plan).err().unwrap(),
        ExactLocalInstallDecisionError::InvalidData
    );
    let mut extra: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    extra
        .as_object_mut()
        .unwrap()
        .insert("unexpected".into(), serde_json::json!(true));
    assert_eq!(
        host.answer_bytes(&serde_json::to_vec(&extra).unwrap())
            .err()
            .unwrap(),
        ExactLocalInstallDecisionError::InvalidData
    );
}

#[test]
fn exact_local_install_answer_refuses_stale_policy_and_source() {
    let (_dir, root, policy, source) = setup();
    let host = host(root.clone(), Arc::clone(&policy), source);
    let question = host.prepare_requested_options(no_options()).unwrap();
    let lease = LifecycleLease::acquire(&root).unwrap();
    let generation = policy
        .current_under_lease(&lease)
        .unwrap()
        .generation()
        .to_owned();
    policy
        .update_under_lease(
            &lease,
            &generation,
            TrustedPolicySource::from_host_identifier("exact-local-test-policy".into()).unwrap(),
            facts(true),
        )
        .unwrap();
    drop(lease);
    assert_eq!(
        host.answer_typed(&answer(&question, true)).err().unwrap(),
        ExactLocalInstallDecisionError::StalePolicy
    );

    let fresh = host.prepare_requested_options(no_options()).unwrap();
    std::fs::write(
        root.parent()
            .unwrap()
            .join("registry/org.example/fixture/v1.1.0/README.md"),
        b"changed\n",
    )
    .unwrap();
    assert!(matches!(
        host.answer_typed(&answer(&fresh, true)).err().unwrap(),
        ExactLocalInstallDecisionError::SourceUnavailable(_)
    ));
    assert!(!root.join("vibe.lock").exists());
    assert!(!root.join(".vibe/lifecycle-outcomes").exists());
}

#[test]
fn exact_local_install_policy_bit_is_independent_from_approval() {
    let (_dir, root, policy, source) = setup();
    let lease = LifecycleLease::acquire(&root).unwrap();
    let generation = policy
        .current_under_lease(&lease)
        .unwrap()
        .generation()
        .to_owned();
    policy
        .update_under_lease(
            &lease,
            &generation,
            TrustedPolicySource::from_host_identifier("exact-local-test-policy".into()).unwrap(),
            facts(false),
        )
        .unwrap();
    drop(lease);
    let host = host(root, policy, source);
    assert_eq!(
        host.prepare_requested_options(no_options()).err().unwrap(),
        ExactLocalInstallDecisionError::PolicyDenied
    );
}
