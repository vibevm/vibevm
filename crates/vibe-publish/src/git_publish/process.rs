//! Shared Git subprocess construction, failure classification, and secret
//! redaction for the publish-side workflows.

use std::path::Path;
use std::process::{Command, Output};

use crate::PublishError;

/// Like [`git_command`] but cwd-less — used by network-only operations.
pub(super) fn git_command_in_temp(args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(args);
    cmd.env("LC_ALL", "C").env("LANG", "C");
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

pub(super) fn staged_tree_changed(cwd: &Path) -> Result<bool, PublishError> {
    let args = ["diff", "--cached", "--quiet", "--exit-code", "HEAD", "--"];
    let output = git_command(cwd, &args).output().map_err(|e| {
        PublishError::Git(format!("spawning git {}: {e}", join_args_redacted(&args)))
    })?;
    match output.status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => {
            let stderr = redact_credentials(String::from_utf8_lossy(&output.stderr).trim());
            Err(PublishError::Git(format!(
                "git {} failed: {stderr}",
                join_args_redacted(&args)
            )))
        }
    }
}

pub(super) fn run_remote_git_in(
    cwd: &Path,
    args: &[&str],
    clone_url: &str,
) -> Result<Output, PublishError> {
    let output = git_command(cwd, args).output().map_err(|e| {
        PublishError::Git(format!("spawning git {}: {e}", join_args_redacted(args)))
    })?;
    classify_remote_output(
        &output,
        clone_url,
        args.first().copied().unwrap_or("operation"),
    )?;
    Ok(output)
}

pub(super) fn classify_remote_output(
    output: &Output,
    clone_url: &str,
    operation: &str,
) -> Result<(), PublishError> {
    if output.status.success() {
        return Ok(());
    }
    let stderr_lower = String::from_utf8_lossy(&output.stderr).to_lowercase();
    let safe_repo = redact_credentials(clone_url);
    if stderr_lower.contains("permission denied")
        || stderr_lower.contains("publickey")
        || stderr_lower.contains("authentication failed")
        || stderr_lower.contains("403")
        || stderr_lower.contains("401")
    {
        return Err(PublishError::PushDenied { repo: safe_repo });
    }
    if stderr_lower.contains("could not resolve host")
        || stderr_lower.contains("network is unreachable")
        || stderr_lower.contains("could not read from remote repository")
    {
        return Err(PublishError::HostUnreachable { host: safe_repo });
    }
    let safe_stderr = redact_credentials(String::from_utf8_lossy(&output.stderr).trim());
    Err(PublishError::Git(format!(
        "git {operation} {safe_repo} failed: {safe_stderr}"
    )))
}

pub(super) fn run_git_in(cwd: &Path, args: &[&str]) -> Result<Output, PublishError> {
    let output = git_command(cwd, args).output().map_err(|e| {
        PublishError::Git(format!("spawning git {}: {e}", join_args_redacted(args)))
    })?;
    if !output.status.success() {
        let stderr = redact_credentials(String::from_utf8_lossy(&output.stderr).trim());
        return Err(PublishError::Git(format!(
            "git {} failed: {stderr}",
            join_args_redacted(args)
        )));
    }
    Ok(output)
}

/// Like [`run_git_in`] but maps `git push` failures onto the structured
/// publish error variants the operator sees.
pub(super) fn push_with_classification(
    cwd: &Path,
    args: &[&str],
    clone_url: &str,
) -> Result<(), PublishError> {
    let output = git_command(cwd, args).output().map_err(|e| {
        PublishError::Git(format!("spawning git {}: {e}", join_args_redacted(args)))
    })?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    let safe_repo = redact_credentials(clone_url);
    if stderr.contains("permission denied")
        || stderr.contains("publickey")
        || stderr.contains("authentication failed")
        || stderr.contains("403")
    {
        return Err(PublishError::PushDenied { repo: safe_repo });
    }
    if is_concurrent_ref_rejection(&stderr) {
        return Err(PublishError::ConcurrentUpdate { repo: safe_repo });
    }
    if stderr.contains("could not resolve host")
        || stderr.contains("network is unreachable")
        || stderr.contains("could not read from remote repository")
    {
        return Err(PublishError::HostUnreachable { host: safe_repo });
    }
    let safe_stderr = redact_credentials(String::from_utf8_lossy(&output.stderr).trim());
    Err(PublishError::Git(format!(
        "git {} failed: {}",
        join_args_redacted(args),
        safe_stderr
    )))
}

pub(super) fn is_concurrent_ref_rejection(stderr_lowercase: &str) -> bool {
    // Git's exact-force-with-lease rejection is `(stale info)`. Do not
    // broaden this to `atomic push failed` or `cannot lock ref`: those
    // strings also cover a host that lacks atomic-push support or rejects
    // mutable tags by policy, and neither condition is a publication race.
    stderr_lowercase.contains("stale info")
}

/// Replace `userinfo` (everything between `://` and `@`) in URL-looking
/// substrings with `***` before any Git diagnostic reaches a user surface.
pub fn redact_credentials(s: impl AsRef<str>) -> String {
    let s = s.as_ref();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if let Some(rel) = s[i..].find("://") {
            let scheme_end = i + rel;
            let mut start = scheme_end;
            while start > 0 {
                let b = bytes[start - 1];
                let valid = b.is_ascii_alphanumeric() || b == b'+' || b == b'-' || b == b'.';
                if !valid {
                    break;
                }
                start -= 1;
            }
            if start < scheme_end && bytes[start].is_ascii_alphabetic() {
                out.push_str(&s[i..start]);
                let after_scheme = scheme_end + 3;
                let mut at_pos = None;
                let mut bound = bytes.len();
                let stops = [b'/', b'?', b'#', b' ', b'\t', b'\n', b'\r', b'"', b'\''];
                for (j, b) in bytes.iter().enumerate().skip(after_scheme) {
                    if *b == b'@' {
                        at_pos = Some(j);
                        bound = j + 1;
                        break;
                    }
                    if stops.contains(b) {
                        bound = j;
                        break;
                    }
                }
                if let Some(at) = at_pos {
                    out.push_str(&s[start..after_scheme]);
                    out.push_str("***");
                    out.push('@');
                    i = at + 1;
                } else {
                    out.push_str(&s[start..bound]);
                    i = bound;
                }
                continue;
            }
        }
        let Some(ch) = s[i..].chars().next() else {
            break;
        };
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn join_args_redacted(args: &[&str]) -> String {
    let parts: Vec<String> = args.iter().map(|a| redact_credentials(*a)).collect();
    parts.join(" ")
}

fn git_command(cwd: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(args);
    cmd.current_dir(cwd);
    cmd.env("LC_ALL", "C").env("LANG", "C");
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}
