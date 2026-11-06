//! A fresh byte-state recovery must not claim the lost target-object history.
use super::*;

#[test]
#[cfg(unix)]
fn inspect_unknown_fifo_stage_needs_no_peer_and_preserves_all_evidence() {
    use std::os::unix::fs::{FileTypeExt as _, OpenOptionsExt as _};
    let f = Fixture::new();
    crash(&f, "after_prepared_journal", "apply");
    let journal = f.journal().unwrap();
    let stage = identity::transaction_stage_name(&journal.intent_id, identity::StageAction::Apply);
    let stage_path = f.project.join(&stage);
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&stage_path)
            .status()
            .unwrap()
            .success()
    );
    let before = f.target();
    let journal_bytes = std::fs::read(f.directory().join("journal.json")).unwrap();
    let image = std::fs::read(f.directory().join("before.bin")).unwrap();
    let project = f.project.clone();
    let store = f.store.clone();
    let request = f.inspect();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let join = std::thread::spawn(move || {
        let grant = OwnedManifestMigrationFile::new(&project, "vibe.toml", &store).unwrap();
        entered_tx.send(()).unwrap();
        done_tx
            .send(execute_manifest_migration_json(&request, &grant).unwrap())
            .unwrap();
    });
    entered_rx.recv().unwrap();
    let settled = done_rx.recv_timeout(std::time::Duration::from_secs(2));
    let without_peer = settled.is_ok();
    let unblock = if without_peer {
        None
    } else {
        Some(
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&stage_path)
                .unwrap(),
        )
    };
    let report = settled.unwrap_or_else(|_| done_rx.recv().unwrap());
    join.join().unwrap();
    drop(unblock);
    assert_eq!(report.outcome, OperationReportOutcome::Pending);
    assert!(
        report
            .residue
            .iter()
            .any(|r| r.relative_path == stage && r.ownership == ResidueOwnership::Unproven)
    );
    assert!(
        std::fs::symlink_metadata(&stage_path)
            .unwrap()
            .file_type()
            .is_fifo()
    );
    assert_eq!(f.target(), before);
    assert_eq!(
        std::fs::read(f.directory().join("journal.json")).unwrap(),
        journal_bytes
    );
    assert_eq!(
        std::fs::read(f.directory().join("before.bin")).unwrap(),
        image
    );
    assert!(
        without_peer,
        "inspection tried to read a FIFO stage before reporting residue"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#OWNED-MIGRATION-STAGING"
)]
fn unavailable_final_observation_retains_uncertain_publication() {
    let f = Fixture::new();
    let path = f.project.join("vibe.toml");
    on_boundary(
        "after_target_publish",
        Box::new(move || {
            std::fs::rename(&path, path.with_file_name("published-before-replacement")).unwrap();
            std::fs::create_dir(path).unwrap();
        }),
    );
    let report = f.call(&f.apply());
    assert_eq!(report.outcome, OperationReportOutcome::Uncertain);
    assert_eq!(report.target_state, OperationReportTargetState::Unavailable);
    assert_eq!(
        report.error.as_ref().unwrap().code,
        ApplicationErrorCode::PublicationIo
    );
    assert!(
        report
            .effects
            .iter()
            .any(|e| e.kind == EffectKind::TargetPublication && e.state == EffectState::Uncertain)
    );
    assert_eq!(
        report.guarantees.durability,
        GuaranteesDurability::Unconfirmed
    );
    assert!(f.project.join("vibe.toml").is_dir());
    assert_eq!(
        std::fs::read(f.project.join("published-before-replacement")).unwrap(),
        BEFORE.replace("'snapshot'", "'copy'").as_bytes()
    );
    assert_eq!(
        f.journal().unwrap().phase,
        ApplicationJournalPhase::Publishing
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#OWNED-MIGRATION-STAGING"
)]
fn durable_intent_reuses_exact_stage_without_rewriting_readonly_mode() {
    let f = Fixture::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(
            f.project.join("vibe.toml"),
            std::fs::Permissions::from_mode(0o400),
        )
        .unwrap();
    }
    crash(&f, "after_publishing_journal", "apply");
    let journal = f.journal().unwrap();
    let stage = identity::transaction_stage_name(&journal.intent_id, identity::StageAction::Apply);
    let path = f.project.join(&stage);
    std::fs::write(&path, BEFORE.replace("'snapshot'", "'copy'")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400)).unwrap();
    }
    let report = f.call(&f.recover(&journal.intent_id, "complete"));
    assert_eq!(report.outcome, OperationReportOutcome::Completed);
    assert!(!path.exists());
    assert_eq!(
        f.target(),
        BEFORE.replace("'snapshot'", "'copy'").as_bytes()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(f.project.join("vibe.toml"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o400
        );
    }
    assert_eq!(f.call(&f.apply()).outcome, OperationReportOutcome::NoOp);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#OWNED-MIGRATION-STAGING"
)]
fn live_receipt_survives_helper_return_until_domain_final_observation() {
    let f = Fixture::new();
    let path = f.project.join("vibe.toml");
    on_boundary(
        "after_target_publish",
        Box::new(move || {
            let metadata = std::fs::metadata(&path).unwrap();
            let bytes = std::fs::read(&path).unwrap();
            let replacement = path.with_file_name("same-byte-after-helper");
            std::fs::write(&replacement, bytes).unwrap();
            std::fs::set_permissions(&replacement, metadata.permissions()).unwrap();
            std::fs::rename(replacement, path).unwrap();
        }),
    );
    let observed = f.call(&f.apply());
    assert_eq!(
        observed.outcome,
        OperationReportOutcome::Uncertain,
        "same bytes on a new object after helper return must not be Completed"
    );
    assert_eq!(
        observed.error.as_ref().unwrap().code,
        ApplicationErrorCode::PublicationIo
    );
    assert!(
        observed
            .effects
            .iter()
            .any(|e| e.kind == EffectKind::TargetPublication && e.state == EffectState::Uncertain)
    );
    assert_eq!(
        observed.guarantees.durability,
        GuaranteesDurability::Unconfirmed
    );
    assert_eq!(
        f.journal().unwrap().phase,
        ApplicationJournalPhase::Publishing
    );
    assert_eq!(
        f.target(),
        BEFORE.replace("'snapshot'", "'copy'").as_bytes()
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MIGRATION-RECOVERY-OBJECT-LIMIT"
)]
fn observed_receipt_mismatch_then_fresh_recovery_retains_explicit_limits() {
    let f = Fixture::new();
    crash(&f, "after_publishing_journal", "apply");
    let intent = f.journal().unwrap().intent_id;
    // Recovery reaches target publication next: the publishing journal and
    // retained image already exist, so this hook cannot target a store write.
    vibe_safefs::arm_before_publish_verify(Some(Box::new(|parent, name| {
        assert_eq!(name, "vibe.toml");
        let path = parent.path().join(name);
        let metadata = std::fs::metadata(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let replacement = parent.path().join("same-byte-replacement");
        std::fs::write(&replacement, bytes).unwrap();
        std::fs::set_permissions(&replacement, metadata.permissions()).unwrap();
        std::fs::rename(replacement, path).unwrap();
    })));
    let observed = f.call(&f.recover(&intent, "complete"));
    vibe_safefs::arm_before_publish_verify(None);
    assert_eq!(observed.outcome, OperationReportOutcome::Uncertain);
    assert_eq!(
        observed.error.as_ref().unwrap().code,
        ApplicationErrorCode::PublicationIo
    );
    assert!(
        observed
            .effects
            .iter()
            .any(|e| e.kind == EffectKind::TargetPublication && e.state == EffectState::Uncertain)
    );
    assert_eq!(
        f.journal().unwrap().phase,
        ApplicationJournalPhase::Publishing
    );

    let grant = f.grant();
    let replacement = grant
        .project()
        .read_file_snapshot_bounded("vibe.toml", usize::MAX - 1)
        .unwrap()
        .unwrap();
    let before_inspect = tree(f._temp.path());
    let inspected = f.call(&f.inspect());
    assert_eq!(inspected.outcome, OperationReportOutcome::Pending);
    assert_eq!(tree(f._temp.path()), before_inspect);
    let completed = f.call(&f.recover(&intent, "complete"));
    assert_eq!(completed.outcome, OperationReportOutcome::Completed);
    assert_eq!(
        grant
            .project()
            .read_file_snapshot_bounded("vibe.toml", usize::MAX - 1)
            .unwrap()
            .unwrap(),
        replacement,
        "fresh completion recognizes byte state; it must not rewrite that target"
    );
    for report in [inspected, completed] {
        assert_eq!(
            report.target_state,
            OperationReportTargetState::ValidatedAfter
        );
        assert_eq!(
            report.guarantees.durability,
            GuaranteesDurability::Unconfirmed
        );
        assert!(!report.guarantees.uncooperating_writers_excluded);
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.code == WarningCode::TargetObjectHistoryUnproven
                    && w.action == WarningAction::Inspect)
        );
    }
}
