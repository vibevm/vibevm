//! The deploy journal pair's scalar laws — the hand-written
//! validation cell beside the two generated §7.2 records: the intent
//! journal written BEFORE apply ([`DeployIntent`]) and the receipt
//! written after ([`DeployReceipt`]) (R8A2, the packages-2026-09
//! architecture §7.2). One cell for both, because they are one
//! exchange: the journal's planned resources and the receipt's owned
//! resources join by the same digests, and recovery reads them as a
//! pair — two cells would split one spelling rule across the seam a
//! crash lands on.
//!
//! JTD owns the FORM (the closed scope and status vocabularies, the
//! optional members, the typed RFC 3339 timestamps); this cell owns
//! what a form cannot say about itself: the epoch constants, the one
//! frozen id grammar on profile and target names, the bare 64-hex
//! digest law, the ExtensionKey provider shape, the free-text safety
//! of every member a reader prints, and the receipt's status/
//! finalisation matrix — receipt finalisation is LAST, so a terminal
//! status without `finalized_at` is a receipt that claims a state it
//! never reached, and `applied` with one is a mid-flight receipt
//! wearing a terminal timestamp. Every predicate that is not this
//! pair's own is REUSED from [`crate::behaviour::scalars`] and the
//! trace-index cell — one grammar, every wire; each record keeps its
//! OWN typed refusals on top of them, exactly as the evidence and
//! requirements cells do.
//!
//! Every value it reads is untrusted — a journal and a receipt are
//! files on disk — so no refusal clones the offending scalar: errors
//! carry a bounded [`ScalarPreview`] and the true byte length.

use crate::behaviour::compiler_trace_index::ScalarPreview;
use crate::behaviour::scalars::{
    has_control_bytes, is_lowercase_hex, is_portable_token, is_sha256, provider_key_defect,
};
use crate::generated::deploy_intent::DeployIntent;
use crate::generated::deploy_receipt::{DeployReceipt, ReceiptStatus};

mod errors;

pub use errors::{DeployIntentError, DeployReceiptError};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// The deploy-intent epoch this validator speaks.
pub const INTENT_EPOCH: u32 = 1;
/// The deploy-receipt epoch this validator speaks.
pub const RECEIPT_EPOCH: u32 = 1;

/// Validate one deploy intent against every scalar law. Pure: the
/// value in, the first broken law out.
pub fn validate_intent(intent: &DeployIntent) -> Result<(), DeployIntentError> {
    if intent.schema != INTENT_EPOCH {
        return Err(DeployIntentError::SchemaEpoch {
            found: intent.schema,
        });
    }
    if !is_lowercase_hex(&intent.plan_hash, 64) {
        return Err(DeployIntentError::PlanHashNotHex {
            plan_hash: preview(&intent.plan_hash),
        });
    }
    let target = &intent.target;
    intent_text_gate("target.project", Some(&target.project))?;
    intent_text_gate("target.package", target.package.as_deref())?;
    if !is_portable_token(&target.profile) {
        return Err(DeployIntentError::ProfileNotPortableToken {
            profile: preview(&target.profile),
        });
    }
    if !is_portable_token(&target.target) {
        return Err(DeployIntentError::TargetNotPortableToken {
            target: preview(&target.target),
        });
    }
    for (row, resource) in intent.resources.iter().enumerate() {
        if is_unsafe_text(&resource.resource) {
            return Err(DeployIntentError::UnsafeResource {
                row,
                value: preview(&resource.resource),
            });
        }
        if !is_lowercase_hex(&resource.desired_digest, 64) {
            return Err(DeployIntentError::DesiredDigestNotHex {
                row,
                desired_digest: preview(&resource.desired_digest),
            });
        }
        if let Some(prior) = &resource.prior_digest
            && !is_lowercase_hex(prior, 64)
        {
            return Err(DeployIntentError::PriorDigestNotHex {
                row,
                prior_digest: preview(prior),
            });
        }
    }
    Ok(())
}

/// Validate one deploy receipt against every scalar law and its
/// status/finalisation matrix. Pure: the value in, the first broken
/// law out.
pub fn validate_receipt(receipt: &DeployReceipt) -> Result<(), DeployReceiptError> {
    if receipt.schema != RECEIPT_EPOCH {
        return Err(DeployReceiptError::SchemaEpoch {
            found: receipt.schema,
        });
    }
    receipt_text_gate("identity.project", Some(&receipt.identity.project))?;
    receipt_text_gate("identity.package", receipt.identity.package.as_deref())?;
    if !is_portable_token(&receipt.profile) {
        return Err(DeployReceiptError::ProfileNotPortableToken {
            profile: preview(&receipt.profile),
        });
    }
    if !is_portable_token(&receipt.target) {
        return Err(DeployReceiptError::TargetNotPortableToken {
            target: preview(&receipt.target),
        });
    }
    for (member, value) in [
        ("artifact_digest", &receipt.artifact_digest),
        ("desired_config_digest", &receipt.desired_config_digest),
    ] {
        if !is_lowercase_hex(value, 64) {
            return Err(DeployReceiptError::DigestNotHex {
                member,
                value: preview(value),
            });
        }
    }
    if let Some(defect) = provider_key_defect(&receipt.provider.key) {
        return Err(DeployReceiptError::BadProviderKey {
            key: preview(&receipt.provider.key),
            defect,
        });
    }
    receipt_text_gate("provider.version", receipt.provider.version.as_deref())?;
    if let Some(hash) = &receipt.provider.content_hash
        && !is_sha256(hash)
    {
        return Err(DeployReceiptError::BadContentHash {
            content_hash: preview(hash),
        });
    }
    for (row, resource) in receipt.resources.iter().enumerate() {
        if is_unsafe_text(&resource.resource) {
            return Err(DeployReceiptError::UnsafeResource {
                row,
                value: preview(&resource.resource),
            });
        }
        if !is_lowercase_hex(&resource.post_digest, 64) {
            return Err(DeployReceiptError::PostDigestNotHex {
                row,
                post_digest: preview(&resource.post_digest),
            });
        }
    }
    receipt_text_gate("evidence", receipt.evidence.as_deref())?;
    receipt_text_gate("prior_state_handle", receipt.prior_state_handle.as_deref())?;
    finalisation_gate(receipt)
}

/// The status/finalisation matrix: `finalized_at` is present exactly
/// for the terminal statuses. Receipt finalisation is last — after
/// independent verify — so `applied` (mid-flight) carries none, and a
/// terminal status without one is a receipt that never reached the
/// state its status claims.
fn finalisation_gate(receipt: &DeployReceipt) -> Result<(), DeployReceiptError> {
    let terminal = matches!(
        receipt.status,
        ReceiptStatus::Verified | ReceiptStatus::Failed | ReceiptStatus::RolledBack
    );
    match (terminal, receipt.finalized_at.is_some()) {
        (true, false) => Err(DeployReceiptError::TerminalNotFinalised {
            status: receipt.status.clone(),
        }),
        (false, true) => Err(DeployReceiptError::AppliedFinalised),
        _ => Ok(()),
    }
}

/// One bounded preview — the same refusal discipline the trace cells
/// use, applied through their shared type.
fn preview(value: &str) -> ScalarPreview {
    ScalarPreview::of(value)
}

/// The shared free-text predicate: non-blank once trimmed and free of
/// the three bytes that break a log line or a C string. The RULE is
/// shared; each record keeps its own typed refusal on top of it.
fn is_unsafe_text(value: &str) -> bool {
    value.trim().is_empty() || has_control_bytes(value)
}

/// The intent's free-text gate: absent is absent, a present value is
/// held to the shared predicate.
fn intent_text_gate(field: &'static str, value: Option<&str>) -> Result<(), DeployIntentError> {
    if let Some(text) = value
        && is_unsafe_text(text)
    {
        return Err(DeployIntentError::UnsafeScalar {
            field,
            value: preview(text),
        });
    }
    Ok(())
}

/// The receipt's twin of [`intent_text_gate`].
fn receipt_text_gate(field: &'static str, value: Option<&str>) -> Result<(), DeployReceiptError> {
    if let Some(text) = value
        && is_unsafe_text(text)
    {
        return Err(DeployReceiptError::UnsafeScalar {
            field,
            value: preview(text),
        });
    }
    Ok(())
}
