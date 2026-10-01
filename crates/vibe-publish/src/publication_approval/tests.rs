use std::fs;

use super::*;
use crate::publication_authority::PublicationAuthorityScope;
use crate::publication_subject::PublicationSubjectExpectations;

const SUBJECT: &str = include_str!(
    "../../../../formats/corpora/publication_subject/e1/valid/complete-five-targets.json"
);
const PLAN: &str = include_str!(
    "../../../../formats/corpora/publication_effect_plan/e1/valid/new-draft-complete.json"
);

fn expected() -> PublicationEffectPlanExpectations<'static> {
    PublicationEffectPlanExpectations {
        subject: PublicationSubjectExpectations {
            version: "1.1.0",
            repository: "vibevm/vibevm",
            smoke_policy_id: "distribution-smoke/e1",
            smoke_policy_epoch: 1,
        },
        host: "github.com",
        owner: "vibevm",
        repository: "vibevm",
        publish: false,
        allow_replace_published: false,
    }
}

struct World {
    settings: tempfile::TempDir,
    _workspace: tempfile::TempDir,
    home: ApprovalHome,
    lease: LifecycleLease,
    scope: SubmissionScope,
    subject: PublicationSubject,
    plan: PublicationEffectPlan,
}

impl World {
    fn new() -> Self {
        let settings = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let home = ApprovalHome::open_selected(settings.path()).unwrap();
        let lease = LifecycleLease::acquire(workspace.path()).unwrap();
        let scope = SubmissionScope {
            context: "fixture-context".into(),
            target: "github.com/vibevm/vibevm".into(),
            principal: "fixture-maintainer".into(),
        };
        let subject = serde_json::from_str(SUBJECT).unwrap();
        let plan = serde_json::from_str(PLAN).unwrap();
        let world = Self {
            settings,
            _workspace: workspace,
            home,
            lease,
            scope,
            subject,
            plan,
        };
        world.write_record(&world.record());
        world
    }

    fn record(&self) -> ApprovalRecord {
        let digest =
            publication_effect_plan_digest(&self.plan, &self.subject, &expected()).unwrap();
        ApprovalRecord {
            format: FORMAT.into(),
            schema: 1,
            context: self.scope.context.clone(),
            root_correlation_token: self
                .lease
                .publication_journal_project()
                .root_correlation_token()
                .unwrap(),
            receipt: PublicationApprovalReceipt {
                approval_id: "approval-1".into(),
                scope: PublicationAuthorityScope {
                    plan_sha256: digest,
                    subject_sha256: self.plan.subject_sha256.clone(),
                    host: self.plan.target.host.clone(),
                    owner: self.plan.target.owner.clone(),
                    repository: self.plan.target.repository.clone(),
                    principal: self.scope.principal.clone(),
                    remote_generation_ref: self.plan.remote_generation_ref.clone(),
                    smoke_policy_id: self.plan.smoke_policy_id.clone(),
                    smoke_policy_epoch: 1,
                    publish: false,
                    allow_replace_published: false,
                },
                issued_at_unix: 1,
                expires_at_unix: 20,
            },
        }
    }

    fn path(&self) -> std::path::PathBuf {
        self.settings
            .path()
            .join(HOME)
            .join(plan_name(&self.record().receipt.scope.plan_sha256).unwrap())
    }
    fn write_record(&self, record: &ApprovalRecord) {
        let path = self.path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec(record).unwrap()).unwrap();
    }
    fn load(&self) -> Result<VerifiedApproval, ApprovalError> {
        self.home.load_at(
            &self.lease,
            &self.scope,
            &self.subject,
            &self.plan,
            &expected(),
            10,
        )
    }
    fn reserve(&self, namespace: &SubmissionNamespace, key: &str) -> String {
        let run_id = vibe_lifecycle::process::propose_run_id(self.lease.root()).unwrap();
        let plan_bytes = serde_json::to_vec(&self.plan).unwrap();
        assert!(matches!(
            SubmissionReservations::reserve(
                &self.lease,
                namespace,
                &self.scope,
                key,
                b"exact-request",
                &plan_bytes,
                &run_id,
            )
            .unwrap(),
            vibe_lifecycle::ReservationDecision::Created { .. }
        ));
        run_id
    }
}

#[test]
fn exact_owner_record_loads_and_redeems_only_one_pending_run() {
    let world = World::new();
    let verified = world.load().unwrap();
    assert_eq!(
        verified.plan_sha256,
        world.record().receipt.scope.plan_sha256
    );
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
    assert_eq!(redeemed.run_id, run_id);
    assert_eq!(redeemed.plan_sha256, verified.plan_sha256);
    assert!(redeemed.marker_sha256.starts_with("sha256:"));
    let row = SubmissionReservations::inspect_existing_key(
        &world.lease,
        &namespace,
        &world.scope,
        "key-a",
    )
    .unwrap()
    .unwrap();
    assert_eq!(row.journal_admission, JournalAdmission::Pending);
    let second =
        SubmissionReservations::issue_namespace(&world.lease, &world.scope, 60_000).unwrap();
    let second_run = world.reserve(&second, "key-b");
    assert!(matches!(
        world.home.redeem_inner(
            &world.lease,
            &second,
            &world.scope,
            "key-b",
            &second_run,
            b"exact-request",
            &verified,
            10,
            false,
        ),
        Err(ApprovalError::RecoveryRequired(_))
    ));
}

#[test]
fn malformed_survived_marker_is_recovery_required_without_reissue() {
    let world = World::new();
    let verified = world.load().unwrap();
    let namespace =
        SubmissionReservations::issue_namespace(&world.lease, &world.scope, 60_000).unwrap();
    let run_id = world.reserve(&namespace, "key-a");
    world
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
    let marker = world
        .settings
        .path()
        .join(HOME)
        .join(REDEEMED)
        .join(marker_name(&verified.approval_id));
    fs::write(&marker, b"malformed survived marker").unwrap();
    assert!(matches!(
        world.home.redeem_inner(
            &world.lease,
            &namespace,
            &world.scope,
            "key-a",
            &run_id,
            b"exact-request",
            &verified,
            10,
            false,
        ),
        Err(ApprovalError::RecoveryRequired(_))
    ));
    assert_eq!(fs::read(marker).unwrap(), b"malformed survived marker");
}

#[test]
fn foreign_expired_malformed_noncanonical_and_wrong_root_records_refuse() {
    let world = World::new();
    let original = world.record();
    let mut foreign = original.clone();
    foreign.context = "foreign".into();
    world.write_record(&foreign);
    assert!(matches!(world.load(), Err(ApprovalError::Refused(_))));
    let mut wrong_scope = original.clone();
    wrong_scope.receipt.scope.principal = "foreign".into();
    world.write_record(&wrong_scope);
    assert!(matches!(world.load(), Err(ApprovalError::Refused(_))));
    let mut expired = original.clone();
    expired.receipt.expires_at_unix = 10;
    world.write_record(&expired);
    assert!(matches!(world.load(), Err(ApprovalError::Refused(_))));
    let mut root = original.clone();
    root.root_correlation_token = sha(b"foreign-root");
    world.write_record(&root);
    assert!(matches!(world.load(), Err(ApprovalError::Refused(_))));
    fs::write(world.path(), b"{").unwrap();
    assert!(matches!(world.load(), Err(ApprovalError::Refused(_))));
    let mut noncanonical = serde_json::to_vec(&original).unwrap();
    noncanonical.push(b' ');
    fs::write(world.path(), noncanonical).unwrap();
    assert!(matches!(world.load(), Err(ApprovalError::Refused(_))));
    world.write_record(&original);
    assert!(world.load().is_ok());
}

#[test]
fn lost_redemption_response_retains_marker_and_never_reissues() {
    let world = World::new();
    let verified = world.load().unwrap();
    let namespace =
        SubmissionReservations::issue_namespace(&world.lease, &world.scope, 60_000).unwrap();
    let run_id = world.reserve(&namespace, "key-a");
    assert!(matches!(
        world.home.redeem_inner(
            &world.lease,
            &namespace,
            &world.scope,
            "key-a",
            &run_id,
            b"exact-request",
            &verified,
            10,
            true,
        ),
        Err(ApprovalError::RecoveryRequired(_))
    ));
    let marker = world
        .settings
        .path()
        .join(HOME)
        .join(REDEEMED)
        .join(marker_name(&verified.approval_id));
    assert!(marker.is_file());
    assert!(matches!(
        world.home.redeem_inner(
            &world.lease,
            &namespace,
            &world.scope,
            "key-a",
            &run_id,
            b"exact-request",
            &verified,
            10,
            false,
        ),
        Err(ApprovalError::RecoveryRequired(_))
    ));
}

#[test]
fn approval_expiring_between_load_and_redemption_creates_no_marker() {
    let world = World::new();
    let verified = world.load().unwrap(); // selected at t=10, expires at t=20
    let namespace =
        SubmissionReservations::issue_namespace(&world.lease, &world.scope, 60_000).unwrap();
    let run_id = world.reserve(&namespace, "key-a");
    assert!(matches!(
        world.home.redeem_inner(
            &world.lease,
            &namespace,
            &world.scope,
            "key-a",
            &run_id,
            b"exact-request",
            &verified,
            20,
            false,
        ),
        Err(ApprovalError::Refused(_))
    ));
    let marker = world
        .settings
        .path()
        .join(HOME)
        .join(REDEEMED)
        .join(marker_name(&verified.approval_id));
    assert!(!marker.exists());
}

#[test]
fn redemption_refuses_a_different_c1_request_before_creating_marker() {
    let world = World::new();
    let verified = world.load().unwrap();
    let namespace =
        SubmissionReservations::issue_namespace(&world.lease, &world.scope, 60_000).unwrap();
    let run_id = world.reserve(&namespace, "key-a");
    assert!(matches!(
        world.home.redeem_inner(
            &world.lease,
            &namespace,
            &world.scope,
            "key-a",
            &run_id,
            b"different-request",
            &verified,
            10,
            false,
        ),
        Err(ApprovalError::Refused(_))
    ));
    let marker = world
        .settings
        .path()
        .join(HOME)
        .join(REDEEMED)
        .join(marker_name(&verified.approval_id));
    assert!(!marker.exists());
}

#[cfg(unix)]
#[test]
fn symlinked_approval_is_not_owner_controlled() {
    use std::os::unix::fs::symlink;
    let world = World::new();
    let outside = tempfile::tempdir().unwrap();
    fs::write(
        outside.path().join("approval.json"),
        serde_json::to_vec(&world.record()).unwrap(),
    )
    .unwrap();
    fs::remove_file(world.path()).unwrap();
    symlink(outside.path().join("approval.json"), world.path()).unwrap();
    assert!(matches!(world.load(), Err(ApprovalError::Refused(_))));
}
