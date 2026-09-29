//! Source-frozen application hashes; planner hashes stay with the planner.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE"
);

use sha2::{Digest, Sha256};
use vibe_wire::generated::migration::e1::application_journal::ApplicationJournal;

fn framed(parts: &[&[u8]]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part);
    }
    format!("{:x}", digest.finalize())
}

pub fn raw_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn operation_storage_key(project_binding: &str, operation_id: &str) -> String {
    framed(&[
        b"vibe-manifest-migration-operation-r1",
        project_binding.as_bytes(),
        operation_id.as_bytes(),
    ])
}

/// Phase deliberately excluded: checkpoint updates cannot rewrite intent.
pub fn intent_identity(journal: &ApplicationJournal) -> String {
    let mode = journal
        .unix_mode
        .map_or_else(|| "absent".to_owned(), |m| m.to_string());
    let a = &journal.authority;
    let fields = [
        "vibe-manifest-migration-intent-r1",
        &journal.operation_id,
        &journal.project_binding,
        &journal.store_binding,
        &a.project_root,
        &a.recovery_store_root,
        &a.target_relative,
        "owned_authored_file",
        &journal.approved_request_json,
        &journal.before_content_id,
        &journal.after_content_id,
        &journal.diff_id,
        &journal.before_raw_sha256,
        &journal.after_raw_sha256,
        &journal.before_length,
        &journal.after_length,
        &mode,
    ];
    framed(&fields.map(str::as_bytes))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageAction {
    Apply,
    Restore,
}

/// Deterministic reserved spelling is not stage ownership evidence.
pub fn transaction_stage_name(intent_id: &str, action: StageAction) -> String {
    let action = match action {
        StageAction::Apply => "apply",
        StageAction::Restore => "restore",
    };
    let digest = framed(&[
        b"vibe-manifest-migration-stage-r1",
        intent_id.as_bytes(),
        action.as_bytes(),
    ]);
    format!(".vibe-stage-tx-{}", &digest[..32])
}
