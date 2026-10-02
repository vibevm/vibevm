//! Closed generated values and bounded publication of exact owner observations.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-PACKAGE-WIRE-DECISION"
);
use serde::{Serialize, de::DeserializeOwned};
use std::io::Write;
use vibe_lifecycle::StaticFileJournalInspection as Owner;
use vibe_lifecycle::{StaticFileObservedFile as File, StaticFileObservedPresence as Presence};
pub(super) use vibe_wire::generated::machine::e1::{
    fixed_package_action_error as err, fixed_package_action_reply as rep,
    fixed_package_action_request as req,
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
pub(super) fn encode<T: Serialize>(v: &T, cap: usize) -> Result<String, String> {
    struct Counter {
        n: usize,
        cap: usize,
    }
    impl Write for Counter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.n = self
                .n
                .checked_add(b.len())
                .filter(|n| *n <= self.cap)
                .ok_or_else(|| std::io::Error::other("serialized frame exceeds cap"))?;
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut c = Counter { n: 0, cap };
    serde_json::to_writer(&mut c, v).map_err(|e| e.to_string())?;
    let mut bytes = Vec::with_capacity(c.n);
    serde_json::to_writer(&mut bytes, v).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}
pub(super) fn roundtrip<T: Serialize + DeserializeOwned + PartialEq>(
    v: &T,
    cap: usize,
) -> Result<String, String> {
    let bytes = encode(v, cap)?;
    let read: T = serde_json::from_str(&bytes).map_err(|e| e.to_string())?;
    if read != *v {
        return Err("generated publication roundtrip mismatch".into());
    }
    Ok(bytes)
}
fn file(f: &File, cap: u64, prefixed: bool) -> Result<rep::ObservedFile, String> {
    Ok(match f {
        File::Absent => rep::ObservedFile::Absent(Box::new(rep::ObservedFileAbsent {})),
        File::Present { sha256, bytes }
            if (if prefixed {
                sha256.strip_prefix("sha256:").is_some_and(sha)
            } else {
                sha(sha256)
            }) && *bytes <= cap =>
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
pub(super) fn project(o: &Owner) -> Result<rep::Reply, String> {
    use rep::*;
    let r = match o {
        Owner::Completed {
            run_id,
            output_sha256,
            artifact_record,
        } => {
            if !sha(output_sha256) || artifact_record != ".vibe/state/artifacts/launcher.bin.json" {
                return Err("invalid fixed output hash or artifact path".into());
            }
            Reply::Completed(Box::new(ReplyCompleted {
                run_id: run_id.clone(),
                output_sha256: output_sha256.clone(),
                artifact_record: artifact_record.clone(),
            }))
        }
        Owner::IntentReady { run_id } => Reply::IntentReady(Box::new(ReplyIntentReady {
            run_id: run_id.clone(),
        })),
        Owner::RecoveryRequired { run_id } => {
            Reply::RecoveryRequired(Box::new(ReplyRecoveryRequired {
                run_id: run_id.clone(),
            }))
        }
        Owner::Retired { run_id } => Reply::Retired(Box::new(ReplyRetired {
            run_id: run_id.clone(),
        })),
        Owner::Uncertain { run_id, partial } => {
            let output_directory = match &partial.output_directory {
                Presence::Absent => ObservedPresence::Absent(Box::new(ObservedPresenceAbsent {})),
                Presence::Present => {
                    ObservedPresence::Present(Box::new(ObservedPresencePresent {}))
                }
                Presence::Unavailable(reason) if !reason.trim().is_empty() => {
                    ObservedPresence::Unavailable(Box::new(ObservedPresenceUnavailable {
                        reason: reason.clone(),
                    }))
                }
                _ => return Err("invalid owner directory reason".into()),
            };
            Reply::Uncertain(Box::new(ReplyUncertain {
                run_id: run_id.clone(),
                partial: Partial {
                    output_directory,
                    output_file: file(&partial.output_file, 2 * 1024 * 1024, false)?,
                    artifact_record: file(&partial.artifact_record, 64 * 1024, true)?,
                },
            }))
        }
    };
    if run(&r).is_some_and(|s| !valid_run(s)) {
        return Err("invalid owner run identity".into());
    }
    Ok(r)
}
pub(super) fn run(r: &rep::Reply) -> Option<&str> {
    match r {
        rep::Reply::Completed(v) => Some(&v.run_id),
        rep::Reply::Uncertain(v) => Some(&v.run_id),
        rep::Reply::IntentReady(v) => Some(&v.run_id),
        rep::Reply::RecoveryRequired(v) => Some(&v.run_id),
        rep::Reply::Retired(v) => v.run_id.as_deref(),
        rep::Reply::Absent(_) => None,
    }
}
pub(super) fn owner_run(o: &Owner) -> Option<&str> {
    match o {
        Owner::Completed { run_id, .. }
        | Owner::Uncertain { run_id, .. }
        | Owner::IntentReady { run_id }
        | Owner::RecoveryRequired { run_id } => Some(run_id),
        Owner::Retired { run_id } => run_id.as_deref(),
    }
}
