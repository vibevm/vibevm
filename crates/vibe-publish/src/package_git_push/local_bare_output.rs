//! Bounded local Git process output and strict two-ref porcelain evidence.
use super::*;
use std::io::Read;
use std::process::ExitStatus;

pub(super) fn bounded_output(
    mut command: Command,
    cap: usize,
) -> Result<(ExitStatus, Vec<u8>, Vec<u8>), String> {
    fn drain(mut reader: impl Read, cap: usize) -> Result<(Vec<u8>, bool), String> {
        let mut kept = Vec::new();
        let mut extra = false;
        let mut chunk = [0u8; 8192];
        loop {
            let count = reader.read(&mut chunk).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            let remaining = cap.saturating_sub(kept.len());
            kept.extend_from_slice(&chunk[..count.min(remaining)]);
            extra |= count > remaining;
        }
        Ok((kept, extra))
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or("Git stdout pipe absent")?;
    let stderr = child.stderr.take();
    let out = std::thread::spawn(move || drain(stdout, cap));
    let err = stderr.map(|pipe| std::thread::spawn(move || drain(pipe, cap)));
    let status = child.wait().map_err(|e| e.to_string())?;
    let (stdout, out_extra) = out.join().map_err(|_| "Git stdout reader panicked")??;
    let (stderr, err_extra) = match err {
        Some(reader) => reader.join().map_err(|_| "Git stderr reader panicked")??,
        None => (Vec::new(), false), // LocalGit routes stderr to null.
    };
    if out_extra || err_extra {
        return Err("Git response exceeds bounded output cap".into());
    }
    Ok((status, stdout, stderr))
}

pub(super) fn porcelain_receipt(
    bytes: &[u8],
    main: &str,
    tag: &str,
    commit_oid: &str,
    tag_oid: &str,
) -> Result<PushReceipt, String> {
    if !bytes.ends_with(b"\n") {
        return Err("atomic push porcelain lacks final newline".into());
    }
    let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let mut main_update = None;
    let mut tag_update = None;
    for line in text.lines() {
        if line.starts_with("To ") || line == "Done" {
            continue;
        }
        let parts: Vec<_> = line.split('\t').collect();
        if parts.len() != 3 || parts[0].len() != 1 || parts[2].is_empty() {
            return Err("malformed atomic push porcelain row".into());
        }
        let kind = match parts[0].as_bytes()[0] {
            b'*' => RefUpdateKind::New,
            b'+' => RefUpdateKind::Forced,
            b' ' => RefUpdateKind::FastForward,
            b'=' => return Err("Git reported up-to-date/no-push ref".into()),
            _ => return Err("Git reported ambiguous/non-updating ref".into()),
        };
        let (source, destination) = parts[1]
            .split_once(':')
            .ok_or("missing porcelain destination")?;
        if destination == main {
            if source != commit_oid {
                return Err("foreign main source OID".into());
            }
            if main_update.replace(kind).is_some() {
                return Err("duplicate main row".into());
            }
        } else if destination == tag {
            if source != tag_oid {
                return Err("foreign tag source OID".into());
            }
            if tag_update.replace(kind).is_some() {
                return Err("duplicate tag row".into());
            }
        } else {
            return Err("foreign porcelain destination".into());
        }
    }
    Ok(PushReceipt {
        main_update: main_update.ok_or("missing main update row")?,
        tag_update: tag_update.ok_or("missing tag update row")?,
    })
}
