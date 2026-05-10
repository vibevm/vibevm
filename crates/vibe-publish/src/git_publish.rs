//! Git operations for the publish flow.
//!
//! Initialise a temp working tree, copy the package contents, set up
//! `origin`, commit, tag, push the tag. Wraps `git` shell-out the same
//! way `vibe-registry`'s `ShellGit` does for consume-side ops, but
//! kept inline here because the publish-side commands aren't on
//! [`vibe_registry::GitBackend`] — that trait is intentionally narrow.
//!
//! Error classification matches PROP-002 §2.10 — push-denied, tag-already-
//! exists, and host-unreachable each produce a distinct
//! [`crate::PublishError`] variant.

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

use crate::PublishError;

/// Initialise a temp git repo, copy the source dir contents into it,
/// commit on `main`, add `origin`, push (creates the branch upstream),
/// then create the tag and push it.
///
/// Path expectations: `source_dir` contains the package payload at its
/// root. We copy verbatim — no `.git` filtering needed since a freshly
/// authored package is unlikely to carry one, and the bare-init in our
/// staging dir would shadow it anyway.
pub fn push_release(
    source_dir: &Path,
    clone_url: &str,
    tag: &str,
    package_name: &str,
    version: &semver::Version,
) -> Result<(), PublishError> {
    let staging = TempDir::new().map_err(|e| PublishError::Io {
        path: std::env::temp_dir(),
        message: format!("creating publish staging dir: {e}"),
    })?;
    let staging_path = staging.path();

    // Copy package contents into staging.
    copy_dir(source_dir, staging_path)?;

    // git init main + identity (use repo-local config so we don't
    // mutate the user's global git config).
    run_git_in(staging_path, &["init", "--initial-branch=main"])?;
    run_git_in(
        staging_path,
        &["config", "user.email", "publish@vibevm.local"],
    )?;
    run_git_in(staging_path, &["config", "user.name", "vibevm publisher"])?;

    // Stage + commit.
    run_git_in(staging_path, &["add", "-A"])?;
    let commit_msg = format!("Release {package_name}@{version}");
    run_git_in(staging_path, &["commit", "-m", &commit_msg])?;

    // Tag the commit. `-a` annotated so the registry's `ls-remote
    // --tags` peeled-form dedup is exercised on the consumer side.
    let tag_msg = format!("{package_name}@{version}");
    run_git_in(
        staging_path,
        &["tag", "-a", tag, "-m", &tag_msg],
    )?;

    // Wire up origin and push the branch first, then the tag. Two
    // separate pushes because `--mirror` would imply we own every
    // ref on the remote — we don't, and a freshly-created repo has
    // none anyway.
    run_git_in(staging_path, &["remote", "add", "origin", clone_url])?;

    push_with_classification(staging_path, &["push", "-u", "origin", "main"], clone_url)?;
    push_with_classification(staging_path, &["push", "origin", tag], clone_url)?;

    Ok(())
}

/// Initialise a temp git repo, copy `source_dir` contents into it, commit
/// on `main`, and push to `clone_url`. No tag — used by redirect-stub
/// creation where the stub repo starts tag-less and tags accrete later
/// via [`push_tag_only`] (`vibe registry redirect-sync`).
pub fn push_initial(
    source_dir: &Path,
    clone_url: &str,
    commit_msg: &str,
) -> Result<(), PublishError> {
    let staging = TempDir::new().map_err(|e| PublishError::Io {
        path: std::env::temp_dir(),
        message: format!("creating publish staging dir: {e}"),
    })?;
    let staging_path = staging.path();

    copy_dir(source_dir, staging_path)?;

    run_git_in(staging_path, &["init", "--initial-branch=main"])?;
    run_git_in(
        staging_path,
        &["config", "user.email", "publish@vibevm.local"],
    )?;
    run_git_in(staging_path, &["config", "user.name", "vibevm publisher"])?;

    run_git_in(staging_path, &["add", "-A"])?;
    run_git_in(staging_path, &["commit", "-m", commit_msg])?;

    run_git_in(staging_path, &["remote", "add", "origin", clone_url])?;
    push_with_classification(staging_path, &["push", "-u", "origin", "main"], clone_url)?;

    Ok(())
}

/// List remote tags via `git ls-remote --tags <url>`. Returned tags are
/// stripped of the `refs/tags/` prefix and any `^{}` peeled-form suffix;
/// duplicates are de-duplicated. Used by the redirect-sync flow to
/// enumerate the target's tag list before mirroring missing tags into
/// the stub.
///
/// `url` may carry embedded credentials (`https://x-access-token:T@…`)
/// — this function never prints the URL, only the structured stderr
/// classification of failures (with credentials redacted).
pub fn ls_remote_tags(url: &str) -> Result<Vec<String>, PublishError> {
    let output = git_command_in_temp(&["ls-remote", "--tags", "--", url])
        .output()
        .map_err(|e| {
            PublishError::Git(format!(
                "spawning git ls-remote {}: {e}",
                redact_credentials(url)
            ))
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
        let safe = redact_credentials(url);
        if stderr.contains("could not resolve host") || stderr.contains("network is unreachable") {
            return Err(PublishError::HostUnreachable { host: safe });
        }
        if stderr.contains("authentication failed")
            || stderr.contains("403")
            || stderr.contains("401")
            || stderr.contains("permission denied")
        {
            return Err(PublishError::PushDenied { repo: safe });
        }
        let safe_stderr = redact_credentials(String::from_utf8_lossy(&output.stderr).trim());
        return Err(PublishError::Git(format!(
            "git ls-remote {safe} failed: {safe_stderr}"
        )));
    }
    let mut tags: Vec<String> = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let refname = parts[1];
        let stripped = refname.strip_prefix("refs/tags/").unwrap_or(refname);
        // ls-remote returns both the tag itself and the peeled-form
        // (`refs/tags/v0.1.0^{}`) for annotated tags. Strip the suffix
        // and de-dup.
        let cleaned = stripped.trim_end_matches("^{}");
        if !cleaned.is_empty() && !tags.iter().any(|t| t == cleaned) {
            tags.push(cleaned.to_string());
        }
    }
    Ok(tags)
}

/// Push a single tag pointing at `target_commit_sha` to `clone_url` —
/// no working tree, no checkout, no fetch of objects. Used by
/// redirect-sync: the stub repo only needs the tag ref pointing at the
/// existing initial commit (the `main` branch HEAD), since stub content
/// is just the marker file regardless of which tag a consumer probes.
///
/// `staging_path` must be an existing local clone of the stub remote
/// (so `git push origin <tag>` can resolve `<tag>` to a known commit).
pub fn push_tag_only(
    staging_path: &Path,
    clone_url: &str,
    tag: &str,
) -> Result<(), PublishError> {
    let tag_msg = format!("redirect stub: surface target ref {tag}");
    run_git_in(
        staging_path,
        &["tag", "-a", tag, "-m", &tag_msg],
    )?;
    push_with_classification(staging_path, &["push", "origin", tag], clone_url)?;
    Ok(())
}

/// Like [`git_command`] but cwd-less — used by network-only ops
/// (`ls-remote`) that don't need a working tree.
fn git_command_in_temp(args: &[&str]) -> Command {
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

/// Clone `clone_url` into a temp working tree on the current branch and
/// return the path. Used by redirect-sync to obtain a local clone of an
/// existing stub before tagging missing target tags into it. The clone
/// is shallow (`--depth=1`) — we only need the `main` commit to anchor
/// new tags onto.
pub fn shallow_clone(clone_url: &str) -> Result<TempDir, PublishError> {
    let staging = TempDir::new().map_err(|e| PublishError::Io {
        path: std::env::temp_dir(),
        message: format!("creating clone staging dir: {e}"),
    })?;
    let dest_str = staging.path().to_string_lossy().into_owned();
    let mut cmd = git_command_in_temp(&[
        "clone",
        "--depth=1",
        "--single-branch",
        "--branch=main",
        clone_url,
        &dest_str,
    ]);
    let output = cmd.output().map_err(|e| {
        PublishError::Git(format!(
            "spawning git clone {}: {e}",
            redact_credentials(clone_url)
        ))
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
        let safe = redact_credentials(clone_url);
        if stderr.contains("could not resolve host") || stderr.contains("network is unreachable") {
            return Err(PublishError::HostUnreachable { host: safe });
        }
        if stderr.contains("authentication failed")
            || stderr.contains("403")
            || stderr.contains("401")
            || stderr.contains("permission denied")
        {
            return Err(PublishError::PushDenied { repo: safe });
        }
        let safe_stderr = redact_credentials(String::from_utf8_lossy(&output.stderr).trim());
        return Err(PublishError::Git(format!(
            "git clone {safe} failed: {safe_stderr}"
        )));
    }
    // Re-write `origin` to the unredacted (credentialed) URL so subsequent
    // `git push origin <tag>` runs reuse the credentials. Equivalent of
    // `git remote set-url origin <clone_url>` on the cloned working tree.
    run_git_in(staging.path(), &["remote", "set-url", "origin", clone_url])?;
    // Set local identity (parallel to push_release / push_initial) so
    // tag annotation does not require a global git config.
    run_git_in(staging.path(), &["config", "user.email", "publish@vibevm.local"])?;
    run_git_in(staging.path(), &["config", "user.name", "vibevm publisher"])?;
    Ok(staging)
}

/// Recursively copy `src` → `dst`. Skips any `.git/` subtree (defensive;
/// unusual to find one in a publish source dir).
fn copy_dir(src: &Path, dst: &Path) -> Result<(), PublishError> {
    std::fs::create_dir_all(dst).map_err(|e| PublishError::Io {
        path: dst.to_path_buf(),
        message: format!("create_dir_all: {e}"),
    })?;
    for entry in walk(src)? {
        let path = entry;
        let rel = path
            .strip_prefix(src)
            .expect("walk yields paths under src")
            .to_path_buf();
        if rel
            .components()
            .any(|c| c.as_os_str() == std::ffi::OsStr::new(".git"))
        {
            continue;
        }
        let target = dst.join(&rel);
        if path.is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| PublishError::Io {
                path: target.clone(),
                message: format!("create_dir_all: {e}"),
            })?;
        } else if path.is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| PublishError::Io {
                    path: parent.to_path_buf(),
                    message: format!("create_dir_all: {e}"),
                })?;
            }
            std::fs::copy(&path, &target).map_err(|e| PublishError::Io {
                path: target.clone(),
                message: format!("copy: {e}"),
            })?;
        }
    }
    Ok(())
}

/// Manual recursive walk; avoids pulling `walkdir` into this crate's
/// runtime deps for one helper.
fn walk(root: &Path) -> Result<Vec<std::path::PathBuf>, PublishError> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(p) = stack.pop() {
        if p.is_dir() {
            let entries = std::fs::read_dir(&p).map_err(|e| PublishError::Io {
                path: p.clone(),
                message: format!("read_dir: {e}"),
            })?;
            for entry in entries {
                let entry = entry.map_err(|e| PublishError::Io {
                    path: p.clone(),
                    message: format!("read_dir entry: {e}"),
                })?;
                let path = entry.path();
                stack.push(path.clone());
                if path.is_file() {
                    out.push(path);
                }
            }
        } else if p.is_file() {
            out.push(p);
        }
    }
    Ok(out)
}

fn run_git_in(cwd: &Path, args: &[&str]) -> Result<Output, PublishError> {
    let output = git_command(cwd, args).output().map_err(|e| PublishError::Git(format!(
        "spawning git {}: {e}",
        join_args_redacted(args)
    )))?;
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
/// PROP-002 error variants the operator sees.
fn push_with_classification(
    cwd: &Path,
    args: &[&str],
    clone_url: &str,
) -> Result<(), PublishError> {
    let output = git_command(cwd, args).output().map_err(|e| PublishError::Git(format!(
        "spawning git {}: {e}",
        join_args_redacted(args)
    )))?;
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
    if stderr.contains("could not resolve host")
        || stderr.contains("network is unreachable")
        || stderr.contains("could not read from remote repository")
    {
        return Err(PublishError::HostUnreachable { host: safe_repo });
    }
    if stderr.contains("already exists") && (stderr.contains("tag") || stderr.contains("ref")) {
        // Pull the tag out of args for a useful message.
        let tag = args
            .iter()
            .rev()
            .find(|a| a.starts_with('v') || a.contains('.'))
            .copied()
            .unwrap_or("<unknown>")
            .to_string();
        return Err(PublishError::TagCollision {
            repo: safe_repo,
            tag,
        });
    }
    let safe_stderr = redact_credentials(String::from_utf8_lossy(&output.stderr).trim());
    Err(PublishError::Git(format!(
        "git {} failed: {}",
        join_args_redacted(args),
        safe_stderr
    )))
}

/// Replace `userinfo` (everything between `://` and `@`) in any URL-looking
/// substring with `***`. Modern git already does this on its own diagnostic
/// output (≥ 2.31), but `vibe-publish` cannot rely on the version of git
/// the operator has installed and MUST scrub anything that could end up in
/// a `PublishError` message rendered to the user. Per
/// [PROP-000 §20](../../../spec/common/PROP-000.md#token-secrecy) the
/// publish token never appears in any vibevm-produced output.
pub(crate) fn redact_credentials(s: impl AsRef<str>) -> String {
    let s = s.as_ref();
    // Walk the string and replace any "<scheme>://<user[:pass]>@" with
    // "<scheme>://***@". The set of schemes is anything before "://"
    // matching `[a-zA-Z][a-zA-Z0-9+.-]*` (per RFC 3986 §3.1).
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if let Some(rel) = s[i..].find("://") {
            let scheme_end = i + rel;
            // Walk back to find the start of the scheme.
            let mut start = scheme_end;
            while start > 0 {
                let b = bytes[start - 1];
                let valid = b.is_ascii_alphanumeric() || b == b'+' || b == b'-' || b == b'.';
                if !valid {
                    break;
                }
                start -= 1;
            }
            // Scheme must start with an ASCII alpha.
            if start < scheme_end && bytes[start].is_ascii_alphabetic() {
                // Copy everything before scheme.
                out.push_str(&s[i..start]);
                // Search for the next '@', '/', '?', or '#' boundary
                // after the "://". An '@' before any path-delimiter
                // means user-info is present.
                let after_scheme = scheme_end + 3; // past "://"
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
                    // No userinfo — copy through the end of the host segment.
                    out.push_str(&s[start..bound]);
                    i = bound;
                }
                continue;
            }
        }
        // Default: copy one byte and advance.
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Render an argv slice for human display, with any embedded credentials
/// redacted. Used in error messages — never on a fast path.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn copy_dir_skips_dot_git_subtrees() {
        let src = tempdir().unwrap();
        let dst = tempdir().unwrap();
        fs::create_dir_all(src.path().join(".git/objects")).unwrap();
        fs::write(src.path().join(".git/HEAD"), "ref: refs/heads/main").unwrap();
        fs::write(src.path().join("README.md"), "hi").unwrap();
        fs::write(
            src.path().join("vibe-package.toml"),
            "[package]\nname = \"x\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        copy_dir(src.path(), dst.path()).unwrap();

        assert!(dst.path().join("README.md").exists());
        assert!(dst.path().join("vibe-package.toml").exists());
        assert!(!dst.path().join(".git").exists());
        assert!(!dst.path().join(".git/HEAD").exists());
    }

    #[test]
    fn push_release_against_local_bare_origin() {
        if !git_available() {
            eprintln!("skipping: git not on PATH");
            return;
        }

        // Build a bare origin we can push to.
        let outer = tempdir().unwrap();
        let bare = outer.path().join("origin.git");
        let init_status = Command::new("git")
            .args(["init", "--bare", bare.to_str().unwrap()])
            .env("LC_ALL", "C")
            .status()
            .unwrap();
        assert!(init_status.success());

        // Build a fake source dir with a manifest + spec file.
        let src = tempdir().unwrap();
        fs::write(
            src.path().join("vibe-package.toml"),
            "[package]\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::create_dir_all(src.path().join("spec")).unwrap();
        fs::write(src.path().join("spec/PROTOCOL.md"), "...").unwrap();

        let url = bare.to_string_lossy().into_owned();
        let v = semver::Version::parse("0.1.0").unwrap();
        push_release(src.path(), &url, "v0.1.0", "wal", &v).expect("push ok");

        // Inspect the bare repo: tag and main branch should both be there.
        let tags = Command::new("git")
            .args(["-C", bare.to_str().unwrap(), "tag", "--list"])
            .env("LC_ALL", "C")
            .output()
            .unwrap();
        let tag_list = String::from_utf8_lossy(&tags.stdout);
        assert!(
            tag_list.contains("v0.1.0"),
            "expected v0.1.0 in tags, got: {tag_list}"
        );

        let branches = Command::new("git")
            .args(["-C", bare.to_str().unwrap(), "branch", "--list"])
            .env("LC_ALL", "C")
            .output()
            .unwrap();
        assert!(
            String::from_utf8_lossy(&branches.stdout).contains("main"),
            "expected main branch in bare origin"
        );
    }

    // Tag-collision classification is exercised via the substring
    // matcher in `push_with_classification`. End-to-end testing of the
    // collision case is awkward because publishing two distinct package
    // trees to the same bare origin fails on the `main` branch
    // non-fast-forward before reaching the tag push. The collision
    // path is best validated against a real registry; that's part of
    // the live-migration smoke-test in the next commit.

    #[test]
    fn redact_credentials_hides_user_info() {
        let url = "https://x-access-token:abcd1234@github.com/vibespecs/flow-wal.git";
        let scrubbed = redact_credentials(url);
        assert_eq!(
            scrubbed,
            "https://***@github.com/vibespecs/flow-wal.git",
            "credentials must be replaced with `***`"
        );
        assert!(!scrubbed.contains("abcd1234"));
    }

    #[test]
    fn redact_credentials_passthrough_when_no_credentials() {
        let url = "https://github.com/vibespecs/flow-wal.git";
        assert_eq!(redact_credentials(url), url);
    }

    #[test]
    fn redact_credentials_handles_ssh_no_scheme() {
        // SSH shorthand `git@host:path` does not match the userinfo
        // pattern (no `://`); pass-through is correct here because
        // this form has no embedded password to hide.
        let url = "git@github.com:vibespecs/flow-wal.git";
        assert_eq!(redact_credentials(url), url);
    }

    #[test]
    fn redact_credentials_handles_ssh_scheme() {
        let url = "ssh://git@github.com/vibespecs/flow-wal.git";
        // `ssh://git@github.com/...` has user `git` but no password —
        // the helper still scrubs it to be safe (consistent with the
        // PROP-000 §20 "never any credential-like token in output"
        // posture). Operators that genuinely needed to see "git" can
        // read the registry URL from `vibe.toml`.
        assert_eq!(
            redact_credentials(url),
            "ssh://***@github.com/vibespecs/flow-wal.git"
        );
    }

    #[test]
    fn redact_credentials_within_message() {
        let msg = "git remote add origin https://x-access-token:secret@github.com/foo/bar.git failed: oops";
        let scrubbed = redact_credentials(msg);
        assert!(!scrubbed.contains("secret"));
        assert!(scrubbed.contains("https://***@github.com/foo/bar.git"));
        assert!(scrubbed.contains("failed: oops"));
    }

    #[test]
    fn redact_credentials_multiple_urls_in_message() {
        let msg = "trying https://user:pw1@host.example/a then https://user:pw2@other.example/b done";
        let scrubbed = redact_credentials(msg);
        assert!(!scrubbed.contains("pw1"));
        assert!(!scrubbed.contains("pw2"));
        assert!(scrubbed.contains("https://***@host.example/a"));
        assert!(scrubbed.contains("https://***@other.example/b"));
    }
}
