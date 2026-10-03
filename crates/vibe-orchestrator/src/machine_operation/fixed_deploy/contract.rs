//! Closed generated fixed Deploy values and owner receipt/partial projection.
use serde::{Serialize, de::DeserializeOwned};
use std::io::Write;
use vibe_lifecycle::retained_mechanism::DeployInspection as Owner;
pub(super) use vibe_wire::generated::machine::e1::{
    fixed_deploy_action_error as err, fixed_deploy_action_reply as rep,
    fixed_deploy_action_request as req,
};
use vibe_wire::{
    behaviour::deploy_records::validate_receipt,
    generated::deploy_receipt::{DeployReceipt, ReceiptStatus},
};
pub const REQUEST_CAP: usize = 128 * 1024;
pub const PUBLICATION_CAP: usize = 256 * 1024;

pub(super) fn key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}
pub(super) fn valid_run(value: &str) -> bool {
    vibe_lifecycle::process::is_valid_run_id(value)
}
pub(super) fn roundtrip<T: Serialize + DeserializeOwned + PartialEq>(
    value: &T,
    cap: usize,
) -> Result<String, String> {
    struct Counter {
        count: usize,
        cap: usize,
    }
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.count = self
                .count
                .checked_add(bytes.len())
                .filter(|count| *count <= self.cap)
                .ok_or_else(|| std::io::Error::other("serialized frame exceeds cap"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter { count: 0, cap };
    serde_json::to_writer(&mut counter, value).map_err(|error| error.to_string())?;
    let text = serde_json::to_string(value).map_err(|error| error.to_string())?;
    let read: T = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    if read != *value {
        return Err("generated publication roundtrip mismatch".into());
    }
    Ok(text)
}
fn observation(value: &str) -> Result<rep::RecordObservation, String> {
    Ok(match value {
        "absent" => rep::RecordObservation::Absent(Box::new(rep::RecordObservationAbsent {})),
        "present" => rep::RecordObservation::Present(Box::new(rep::RecordObservationPresent {})),
        reason if !reason.trim().is_empty() && reason.len() <= 4096 => {
            rep::RecordObservation::Unavailable(Box::new(rep::RecordObservationUnavailable {
                reason: reason.into(),
            }))
        }
        _ => return Err("invalid owner record observation".into()),
    })
}
fn receipt_json(receipt: &DeployReceipt) -> Result<String, String> {
    validate_receipt(receipt).map_err(|error| error.to_string())?;
    if receipt.status != ReceiptStatus::Verified {
        return Err("fixed Deploy completed receipt is not verified".into());
    }
    let text = serde_json::to_string(receipt).map_err(|error| error.to_string())?;
    if text.len() > PUBLICATION_CAP {
        return Err("nested receipt exceeds cap".into());
    }
    let recovered: DeployReceipt =
        serde_json::from_str(&text).map_err(|error| error.to_string())?;
    validate_receipt(&recovered).map_err(|error| error.to_string())?;
    if recovered != *receipt {
        return Err("nested receipt differs from owner DTO".into());
    }
    Ok(text)
}
pub(super) fn project(value: &Owner, key: &str) -> Result<rep::Reply, String> {
    use rep::*;
    let reply = match value {
        Owner::Absent => Reply::Absent(Box::new(ReplyAbsent { key: key.into() })),
        Owner::Retired { run_id } => Reply::Retired(Box::new(ReplyRetired {
            run_id: run_id.clone(),
        })),
        Owner::RecoveryRequired { run_id } => {
            Reply::RecoveryRequired(Box::new(ReplyRecoveryRequired {
                run_id: run_id.clone(),
            }))
        }
        Owner::Uncertain { run_id, partial } => Reply::Uncertain(Box::new(ReplyUncertain {
            run_id: run_id.clone(),
            partial: Partial {
                intent: observation(&partial.intent)?,
                receipt: observation(&partial.receipt)?,
                sidecar: observation(&partial.sidecar)?,
            },
        })),
        Owner::Completed { run_id, result } => Reply::Completed(Box::new(ReplyCompleted {
            run_id: run_id.clone(),
            receipt_format_id: ReplyCompletedReceiptFormatId::DeployReceipt,
            receipt_json: receipt_json(&result.receipt)?,
        })),
    };
    if run(&reply).is_some_and(|id| !valid_run(id)) {
        return Err("invalid owner run identity".into());
    }
    Ok(reply)
}
pub(super) fn run(value: &rep::Reply) -> Option<&str> {
    match value {
        rep::Reply::Completed(row) => Some(&row.run_id),
        rep::Reply::Uncertain(row) => Some(&row.run_id),
        rep::Reply::RecoveryRequired(row) => Some(&row.run_id),
        rep::Reply::Retired(row) => row.run_id.as_deref(),
        rep::Reply::Absent(_) => None,
    }
}
pub(super) fn owner_run(value: &Owner) -> Option<&str> {
    match value {
        Owner::Completed { run_id, .. }
        | Owner::Uncertain { run_id, .. }
        | Owner::RecoveryRequired { run_id } => Some(run_id),
        Owner::Retired { run_id } => run_id.as_deref(),
        Owner::Absent => None,
    }
}
fn error_observation(value: &rep::RecordObservation) -> err::RecordObservation {
    match value {
        rep::RecordObservation::Absent(_) => {
            err::RecordObservation::Absent(Box::new(err::RecordObservationAbsent {}))
        }
        rep::RecordObservation::Present(_) => {
            err::RecordObservation::Present(Box::new(err::RecordObservationPresent {}))
        }
        rep::RecordObservation::Unavailable(row) => {
            err::RecordObservation::Unavailable(Box::new(err::RecordObservationUnavailable {
                reason: row.reason.clone(),
            }))
        }
    }
}
pub(super) fn error_view(value: &rep::Reply) -> err::Reply {
    match value {
        rep::Reply::Completed(row) => err::Reply::Completed(Box::new(err::ReplyCompleted {
            run_id: row.run_id.clone(),
            receipt_format_id: err::ReplyCompletedReceiptFormatId::DeployReceipt,
            receipt_json: row.receipt_json.clone(),
        })),
        rep::Reply::Uncertain(row) => err::Reply::Uncertain(Box::new(err::ReplyUncertain {
            run_id: row.run_id.clone(),
            partial: err::Partial {
                intent: error_observation(&row.partial.intent),
                receipt: error_observation(&row.partial.receipt),
                sidecar: error_observation(&row.partial.sidecar),
            },
        })),
        rep::Reply::RecoveryRequired(row) => {
            err::Reply::RecoveryRequired(Box::new(err::ReplyRecoveryRequired {
                run_id: row.run_id.clone(),
            }))
        }
        rep::Reply::Retired(row) => err::Reply::Retired(Box::new(err::ReplyRetired {
            run_id: row.run_id.clone(),
        })),
        rep::Reply::Absent(row) => err::Reply::Absent(Box::new(err::ReplyAbsent {
            key: row.key.clone(),
        })),
    }
}
