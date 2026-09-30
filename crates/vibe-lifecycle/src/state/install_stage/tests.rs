//! Focused install-entered owner-stage tests.

use std::sync::Arc;

use super::*;
use crate::LifecycleStateStore;
use crate::process::capture_selected_manifest_lock;

const RUN: &str = "00112233445566778899aabbccddeeff";
const OTHER: &str = "ffeeddccbbaa99887766554433221100";
const REQUEST: &[u8] = b"protected-generate-request";
const PLAN: &[u8] = b"protected-generate-preplan";

struct Fixture {
    dir: tempfile::TempDir,
    lease: Arc<LifecycleLease>,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    witness: SelectedManifestLockWitness,
}

impl Fixture {
    fn new(ready: bool, header_run: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vibe.toml"),
            "[project]\nname = 'sample'\nversion = '1.1.0'\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("vibe.lock"), b"lock").unwrap();
        let lease = Arc::new(LifecycleLease::acquire(dir.path()).unwrap());
        let scope = SubmissionScope {
            context: "context".into(),
            target: "root".into(),
            principal: "owner".into(),
        };
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let decision =
            SubmissionReservations::reserve(&lease, &namespace, &scope, "one", REQUEST, PLAN, RUN)
                .unwrap();
        assert!(matches!(decision, ReservationDecision::Created { .. }));
        let state = LifecycleStateStore::begin(
            lease.clone(),
            "generate".into(),
            vec!["validate".into(), "install".into(), "generate".into()],
            "2026-09-30T00:00:00Z".into(),
            header_run.into(),
            ".".into(),
            false,
        )
        .unwrap();
        drop(state);
        if ready {
            SubmissionReservations::mark_journal_ready(&lease, &namespace, &scope, "one", RUN)
                .unwrap();
        }
        let witness = capture_selected_manifest_lock(dir.path()).unwrap();
        Self {
            dir,
            lease,
            namespace,
            scope,
            witness,
        }
    }

    fn publish(&self, plan: &[u8]) -> Result<InstallEnteredStage, InstallStageError> {
        publish_install_entered_stage(
            &self.lease,
            &self.namespace,
            &self.scope,
            "one",
            RUN,
            REQUEST,
            plan,
            self.dir.path(),
            &self.witness,
            self.dir.path(),
            &self.witness,
        )
    }

    fn stage_path(&self) -> std::path::PathBuf {
        self.dir
            .path()
            .join(".vibe/lifecycle-outcomes")
            .join(format!("{RUN}.install-entered.json"))
    }

    fn assert_no_stage_or_scratch(&self) {
        assert!(!self.stage_path().exists());
        assert!(!self.dir.path().join(".vibe/lifecycle").exists());
    }
}

#[test]
fn ready_install_stage_is_immutable_read_back_and_never_allocates_scratch() {
    let fixture = Fixture::new(true, RUN);
    let stage = fixture.publish(PLAN).unwrap();
    assert_eq!(stage.run_id, RUN);
    assert_eq!(
        stage.pre_effect_plan_sha256,
        format!("sha256:{:x}", Sha256::digest(PLAN))
    );
    assert!(stage.reservation_ref.starts_with("sha256:"));
    let bytes = std::fs::read(fixture.stage_path()).unwrap();
    let record: StageRecord = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(record.run_id, RUN);
    assert_eq!(record.reservation_ref, stage.reservation_ref);
    assert_eq!(record.effect_truth, "may-effect-uncertain-after-entry");
    assert!(!fixture.dir.path().join(".vibe/lifecycle").exists());
    assert!(matches!(
        fixture.publish(PLAN),
        Err(InstallStageError::RecoveryRequired(_))
    ));
    assert_eq!(std::fs::read(fixture.stage_path()).unwrap(), bytes);
}

#[test]
fn wrong_plan_or_unready_reservation_refuses_without_stage() {
    let ready = Fixture::new(true, RUN);
    assert!(matches!(
        ready.publish(b"different-plan"),
        Err(InstallStageError::RecoveryRequired(_))
    ));
    ready.assert_no_stage_or_scratch();
    let pending = Fixture::new(false, RUN);
    assert!(matches!(
        pending.publish(PLAN),
        Err(InstallStageError::RecoveryRequired(_))
    ));
    pending.assert_no_stage_or_scratch();
}

#[test]
fn foreign_header_or_source_drift_refuses_without_stage() {
    let foreign = Fixture::new(true, OTHER);
    assert!(matches!(
        foreign.publish(PLAN),
        Err(InstallStageError::RecoveryRequired(_))
    ));
    foreign.assert_no_stage_or_scratch();

    let drift = Fixture::new(true, RUN);
    std::fs::write(
        drift.dir.path().join("vibe.toml"),
        "[project]\nname = 'changed'\nversion = '1.1.0'\n",
    )
    .unwrap();
    assert!(matches!(
        drift.publish(PLAN),
        Err(InstallStageError::SourceRefused(_))
    ));
    drift.assert_no_stage_or_scratch();

    let lock_drift = Fixture::new(true, RUN);
    std::fs::write(lock_drift.dir.path().join("vibe.lock"), b"different lock").unwrap();
    assert!(matches!(
        lock_drift.publish(PLAN),
        Err(InstallStageError::SourceRefused(_))
    ));
    lock_drift.assert_no_stage_or_scratch();
}
