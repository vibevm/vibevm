//! Closed generated values and bounded projection of the fixed Cargo owner.
use serde::{Serialize, de::DeserializeOwned};
use std::io::Write;
use vibe_lifecycle::retained_mechanism::{
    BuildInspection as Owner, ObservedFile as File, ObservedPresence as Presence,
};
pub(super) use vibe_wire::generated::machine::e1::{
    fixed_cargo_action_error as err, fixed_cargo_action_reply as rep,
    fixed_cargo_action_request as req,
};
pub const REQUEST_CAP: usize = 128 * 1024;
pub const PUBLICATION_CAP: usize = 256 * 1024;

pub(super) fn key(s: &str) -> bool {
    !s.is_empty() && s.len() <= 256 && s.trim() == s && !s.chars().any(char::is_control)
}
pub(super) fn valid_run(s: &str) -> bool {
    vibe_lifecycle::process::is_valid_run_id(s)
}
fn sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
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
                .filter(|n| *n <= self.cap)
                .ok_or_else(|| std::io::Error::other("serialized frame exceeds cap"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter { count: 0, cap };
    serde_json::to_writer(&mut counter, value).map_err(|e| e.to_string())?;
    let text = serde_json::to_string(value).map_err(|e| e.to_string())?;
    let recovered: T = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if recovered != *value {
        return Err("generated publication roundtrip mismatch".into());
    }
    Ok(text)
}
fn presence(p: &Presence) -> Result<rep::ObservedPresence, String> {
    Ok(match p {
        Presence::Absent => rep::ObservedPresence::Absent(Box::new(rep::ObservedPresenceAbsent {})),
        Presence::Present => {
            rep::ObservedPresence::Present(Box::new(rep::ObservedPresencePresent {}))
        }
        Presence::Unavailable(reason) if !reason.trim().is_empty() => {
            rep::ObservedPresence::Unavailable(Box::new(rep::ObservedPresenceUnavailable {
                reason: reason.clone(),
            }))
        }
        _ => return Err("invalid owner presence reason".into()),
    })
}
fn file(f: &File, cap: u64) -> Result<rep::ObservedFile, String> {
    Ok(match f {
        File::Absent => rep::ObservedFile::Absent(Box::new(rep::ObservedFileAbsent {})),
        File::Present { sha256, bytes }
            if sha256.strip_prefix("sha256:").is_some_and(sha) && *bytes <= cap =>
        {
            rep::ObservedFile::Present(Box::new(rep::ObservedFilePresent {
                sha256: sha256.clone(),
                bytes: *bytes,
            }))
        }
        File::Unavailable(reason) if !reason.trim().is_empty() => {
            rep::ObservedFile::Unavailable(Box::new(rep::ObservedFileUnavailable {
                reason: reason.clone(),
            }))
        }
        _ => return Err("invalid owner file hash, count or reason".into()),
    })
}
pub(super) fn project(o: &Owner, key: &str) -> Result<rep::Reply, String> {
    use rep::*;
    let reply = match o {
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
                target_root: presence(&partial.target_root)?,
                cargo_home: presence(&partial.cargo_home)?,
                cargo_lock: file(&partial.cargo_lock, 64 * 1024)?,
                artifact_record: file(&partial.artifact_record, 64 * 1024)?,
            },
        })),
        Owner::Completed { run_id, result } => {
            if !sha(&result.output_sha256)
                || result.artifact_record != ".vibe/state/artifacts/fixture-tool.bin.json"
                || result.output_relative.is_empty()
                || result.output_relative.starts_with('/')
                || result
                    .output_relative
                    .split('/')
                    .any(|part| part == ".." || part.is_empty())
                || result.warning.trim().is_empty()
            {
                return Err("invalid fixed Cargo completion identity or warning".into());
            }
            Reply::Completed(Box::new(ReplyCompleted {
                run_id: run_id.clone(),
                output_relative: result.output_relative.clone(),
                output_sha256: result.output_sha256.clone(),
                artifact_record: result.artifact_record.clone(),
                warning: result.warning.clone(),
            }))
        }
    };
    if run(&reply).is_some_and(|id| !valid_run(id)) {
        return Err("invalid owner run identity".into());
    }
    Ok(reply)
}
pub(super) fn run(r: &rep::Reply) -> Option<&str> {
    match r {
        rep::Reply::Completed(v) => Some(&v.run_id),
        rep::Reply::Uncertain(v) => Some(&v.run_id),
        rep::Reply::RecoveryRequired(v) => Some(&v.run_id),
        rep::Reply::Retired(v) => v.run_id.as_deref(),
        rep::Reply::Absent(_) => None,
    }
}
pub(super) fn owner_run(o: &Owner) -> Option<&str> {
    match o {
        Owner::Completed { run_id, .. }
        | Owner::Uncertain { run_id, .. }
        | Owner::RecoveryRequired { run_id } => Some(run_id),
        Owner::Retired { run_id } => run_id.as_deref(),
        Owner::Absent => None,
    }
}

fn error_presence(value: &rep::ObservedPresence) -> err::ObservedPresence {
    match value {
        rep::ObservedPresence::Absent(_) => {
            err::ObservedPresence::Absent(Box::new(err::ObservedPresenceAbsent {}))
        }
        rep::ObservedPresence::Present(_) => {
            err::ObservedPresence::Present(Box::new(err::ObservedPresencePresent {}))
        }
        rep::ObservedPresence::Unavailable(row) => {
            err::ObservedPresence::Unavailable(Box::new(err::ObservedPresenceUnavailable {
                reason: row.reason.clone(),
            }))
        }
    }
}
fn error_file(value: &rep::ObservedFile) -> err::ObservedFile {
    match value {
        rep::ObservedFile::Absent(_) => {
            err::ObservedFile::Absent(Box::new(err::ObservedFileAbsent {}))
        }
        rep::ObservedFile::Present(row) => {
            err::ObservedFile::Present(Box::new(err::ObservedFilePresent {
                sha256: row.sha256.clone(),
                bytes: row.bytes,
            }))
        }
        rep::ObservedFile::Unavailable(row) => {
            err::ObservedFile::Unavailable(Box::new(err::ObservedFileUnavailable {
                reason: row.reason.clone(),
            }))
        }
    }
}
/// A typed projection across the two independently generated reply families.
pub(super) fn error_view(value: &rep::Reply) -> err::Reply {
    match value {
        rep::Reply::Completed(row) => err::Reply::Completed(Box::new(err::ReplyCompleted {
            run_id: row.run_id.clone(),
            output_relative: row.output_relative.clone(),
            output_sha256: row.output_sha256.clone(),
            artifact_record: row.artifact_record.clone(),
            warning: row.warning.clone(),
        })),
        rep::Reply::Uncertain(row) => err::Reply::Uncertain(Box::new(err::ReplyUncertain {
            run_id: row.run_id.clone(),
            partial: err::Partial {
                target_root: error_presence(&row.partial.target_root),
                cargo_home: error_presence(&row.partial.cargo_home),
                cargo_lock: error_file(&row.partial.cargo_lock),
                artifact_record: error_file(&row.partial.artifact_record),
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
