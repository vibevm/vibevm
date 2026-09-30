use super::*;
use std::sync::Arc;

use crate::LifecycleStateStore;

const REQUEST: &[u8] = b"request";
const PLAN: &[u8] = b"plan";

struct Fixture {
    dir: tempfile::TempDir,
    lease: Arc<LifecycleLease>,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    provider: PathBuf,
    run_id: String,
}

impl Fixture {
    fn new(ready: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let provider = dir.path().join("provider");
        std::fs::create_dir(&provider).unwrap();
        std::fs::write(provider.join("run.sh"), b"echo one\n").unwrap();
        let lease = Arc::new(LifecycleLease::acquire(dir.path()).unwrap());
        let scope = SubmissionScope {
            context: "context".into(),
            target: "target".into(),
            principal: "principal".into(),
        };
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let run_id = "a".repeat(32);
        assert!(matches!(
            SubmissionReservations::reserve(
                &lease, &namespace, &scope, "key", REQUEST, PLAN, &run_id
            )
            .unwrap(),
            ReservationDecision::Created { .. }
        ));
        let state = LifecycleStateStore::begin(
            lease.clone(),
            "generate".into(),
            vec!["validate".into(), "install".into(), "generate".into()],
            "2026-09-30T00:00:00Z".into(),
            run_id.clone(),
            ".".into(),
            false,
        )
        .unwrap();
        drop(state);
        if ready {
            SubmissionReservations::mark_journal_ready(&lease, &namespace, &scope, "key", &run_id)
                .unwrap();
        }
        Self {
            dir,
            lease,
            namespace,
            scope,
            provider,
            run_id,
        }
    }

    fn stage(&self) -> Result<ScriptSourceStage, ScriptSourceStageError> {
        publish_script_source_stage(
            &self.lease,
            &self.namespace,
            &self.scope,
            "key",
            &self.run_id,
            REQUEST,
            PLAN,
            "generate",
            "org.example/script",
            "org.example/provider@1.1.0",
            &self.provider,
            "run.sh",
        )
    }

    fn stage_path(&self) -> PathBuf {
        self.dir
            .path()
            .join(".vibe/lifecycle-outcomes")
            .join(stage_name(&self.run_id, "generate", "org.example/script"))
    }
}

#[test]
fn script_stage_publishes_and_reads_back_exact_bound_source() {
    let fixture = Fixture::new(true);
    let stage = fixture.stage().unwrap();
    stage.recheck_before_spawn().unwrap();
    let bytes = std::fs::read(fixture.stage_path()).unwrap();
    let record: StageRecord = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(record.run_id, fixture.run_id);
    assert_eq!(record.phase, "generate");
    assert_eq!(record.script_relative, "run.sh");
    assert_eq!(record.script_size, 9);
    assert_eq!(record.stage, "invoke-may-effect");
    assert_eq!(
        record.pre_effect_plan_sha256,
        format!("sha256:{:x}", Sha256::digest(PLAN))
    );
    assert_eq!(record.reservation_ref.len(), "sha256:".len() + 64);
}

#[test]
fn script_stage_recheck_refuses_changed_bytes() {
    let fixture = Fixture::new(true);
    let stage = fixture.stage().unwrap();
    std::fs::write(fixture.provider.join("run.sh"), b"echo two\n").unwrap();
    assert!(matches!(
        stage.recheck_before_spawn(),
        Err(ScriptSourceStageError::RecoveryRequired(_))
    ));
    assert!(!fixture.dir.path().join(".vibe/lifecycle").exists());
}

#[test]
fn script_stage_same_run_and_key_requires_recovery_without_overwrite() {
    let fixture = Fixture::new(true);
    fixture.stage().unwrap();
    let original = std::fs::read(fixture.stage_path()).unwrap();
    assert!(matches!(
        fixture.stage(),
        Err(ScriptSourceStageError::RecoveryRequired(_))
    ));
    assert_eq!(std::fs::read(fixture.stage_path()).unwrap(), original);
}

#[test]
fn script_stage_foreign_scope_and_not_ready_leave_no_stage() {
    let pending = Fixture::new(false);
    assert!(matches!(
        pending.stage(),
        Err(ScriptSourceStageError::RecoveryRequired(_))
    ));
    assert!(!pending.stage_path().exists());

    let ready = Fixture::new(true);
    let mut foreign = ready.scope.clone();
    foreign.principal = "someone-else".into();
    assert!(matches!(
        publish_script_source_stage(
            &ready.lease,
            &ready.namespace,
            &foreign,
            "key",
            &ready.run_id,
            REQUEST,
            PLAN,
            "generate",
            "org.example/script",
            "org.example/provider@1.1.0",
            &ready.provider,
            "run.sh",
        ),
        Err(ScriptSourceStageError::RecoveryRequired(_))
    ));
    assert!(!ready.stage_path().exists());
}

#[test]
fn script_stage_refuses_mismatched_canonical_plan() {
    let fixture = Fixture::new(true);
    assert!(matches!(
        publish_script_source_stage(
            &fixture.lease,
            &fixture.namespace,
            &fixture.scope,
            "key",
            &fixture.run_id,
            REQUEST,
            b"different-plan",
            "generate",
            "org.example/script",
            "org.example/provider@1.1.0",
            &fixture.provider,
            "run.sh",
        ),
        Err(ScriptSourceStageError::RecoveryRequired(_))
    ));
    assert!(!fixture.stage_path().exists());
}

#[test]
fn script_stage_refuses_phase_outside_reserved_lifecycle_chain() {
    let fixture = Fixture::new(true);
    assert!(matches!(
        publish_script_source_stage(
            &fixture.lease,
            &fixture.namespace,
            &fixture.scope,
            "key",
            &fixture.run_id,
            REQUEST,
            PLAN,
            "deploy",
            "org.example/script",
            "org.example/provider@1.1.0",
            &fixture.provider,
            "run.sh",
        ),
        Err(ScriptSourceStageError::RecoveryRequired(_))
    ));
    assert!(!fixture.stage_path().exists());
}

#[test]
fn script_stage_refuses_oversize_source_without_publication() {
    let fixture = Fixture::new(true);
    std::fs::write(fixture.provider.join("run.sh"), vec![b'x'; SCRIPT_CAP + 1]).unwrap();
    assert!(matches!(
        fixture.stage(),
        Err(ScriptSourceStageError::SourceRefused(_))
    ));
    assert!(!fixture.stage_path().exists());
}

#[cfg(unix)]
#[test]
fn script_stage_refuses_symlink_source_without_publication() {
    let fixture = Fixture::new(true);
    std::fs::rename(
        fixture.provider.join("run.sh"),
        fixture.provider.join("actual.sh"),
    )
    .unwrap();
    std::os::unix::fs::symlink("actual.sh", fixture.provider.join("run.sh")).unwrap();
    assert!(matches!(
        fixture.stage(),
        Err(ScriptSourceStageError::SourceRefused(_))
    ));
    assert!(!fixture.stage_path().exists());
}
