//! Git operations for the publish flow.
//!
//! Initialise a temporary working tree, reproduce the package contents,
//! commit on top of the observed `main`, and atomically publish `main`
//! plus the selected mutable version tag. Wraps `git` shell-out the same
//! way `vibe-registry`'s `ShellGit` does for consume-side ops, but kept
//! inline here because the publish-side commands aren't on
//! [`vibe_registry::GitBackend`] — that trait is intentionally narrow.
//!
//! Error classification matches PROP-002 §2.10 — push-denied, concurrent
//! ref movement, and host-unreachable produce distinct [`crate::PublishError`]
//! variants. Host policy and protocol capability failures retain git's
//! redacted diagnostic instead of being mislabeled as version immutability.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#publish");

use std::path::Path;

use tempfile::TempDir;

use crate::PublishError;

mod process;
#[cfg(test)]
use process::is_concurrent_ref_rejection;
pub use process::redact_credentials;
use process::{
    classify_remote_output, git_command_in_temp, push_with_classification, run_git_in,
    run_remote_git_in, staged_tree_changed,
};

mod submodules;
pub use submodules::SubmoduleProvenance;

/// Publish the source tree as the current release on `main` and `tag`.
///
/// A missing repository history gets one initial commit. An existing
/// `main` is fetched and used as the parent of a new commit whose tree is
/// an exact copy of `source_dir` (including deletions of stale files).
/// Re-publishing the same version moves that version's tag to the new
/// commit; other version tags are left untouched. Branch and tag are sent
/// in one atomic push, each guarded by the exact ref value observed before
/// preparing the release, so a concurrent publisher changes neither ref.
/// An identical retry whose tag already resolves to `main` is a no-op.
///
/// The push URL is passed directly to `git fetch` / `git push`; it is never
/// installed as a remote and therefore never persisted in `.git/config`.
pub fn push_release(
    source_dir: &Path,
    clone_url: &str,
    tag: &str,
    package_name: &str,
    version: &semver::Version,
) -> Result<Vec<SubmoduleProvenance>, PublishError> {
    let submodules = submodules::inspect(source_dir)?;
    push_release_inner(
        source_dir,
        clone_url,
        tag,
        package_name,
        version,
        |_| Ok(()),
    )?;
    Ok(submodules)
}

/// Inspect the submodules that a publish would flatten, without changing the
/// source or a remote. Used to make dry-run reporting match real publication.
pub fn inspect_submodules(source_dir: &Path) -> Result<Vec<SubmoduleProvenance>, PublishError> {
    submodules::inspect(source_dir)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ReleaseRefSnapshot {
    main: Option<String>,
    tag: Option<String>,
    tag_target: Option<String>,
}

fn push_release_inner<F>(
    source_dir: &Path,
    clone_url: &str,
    tag: &str,
    package_name: &str,
    version: &semver::Version,
    before_push: F,
) -> Result<(), PublishError>
where
    F: FnOnce(&Path) -> Result<(), PublishError>,
{
    let snapshot = release_ref_snapshot(clone_url, tag)?;
    let staging = TempDir::new().map_err(|e| PublishError::Io {
        path: std::env::temp_dir(),
        message: format!("creating publish staging dir: {e}"),
    })?;
    let staging_path = staging.path();

    // Use only repo-local identity. In particular, do not configure a
    // remote: `clone_url` can contain a short-lived publish credential.
    run_git_in(staging_path, &["init", "--initial-branch=main"])?;
    run_git_in(
        staging_path,
        &["config", "user.email", "publish@vibevm.local"],
    )?;
    run_git_in(staging_path, &["config", "user.name", "vibevm publisher"])?;

    if let Some(expected_main) = snapshot.main.as_deref() {
        // Fetch the advertised branch into a private local ref, then
        // verify it is still the exact value we observed. A move between
        // ls-remote and fetch is a concurrency conflict, not a new base
        // that this invocation is allowed to adopt silently.
        let fetched_ref = "refs/remotes/vibe-publish/main";
        let fetch_refspec = format!("refs/heads/main:{fetched_ref}");
        run_remote_git_in(
            staging_path,
            &[
                "fetch",
                "--no-tags",
                "--no-write-fetch-head",
                "--",
                clone_url,
                &fetch_refspec,
            ],
            clone_url,
        )?;
        let fetched_main = rev_parse(staging_path, fetched_ref)?;
        if fetched_main != expected_main {
            return Err(PublishError::ConcurrentUpdate {
                repo: redact_credentials(clone_url),
            });
        }
        run_git_in(staging_path, &["checkout", "-B", "main", fetched_ref])?;
    }

    // Replace the checkout payload wholesale. This is what makes a
    // republished tree exact rather than merely overlaying new files on
    // top of stale bytes from the previous release.
    clear_payload_tree(staging_path)?;
    copy_dir(source_dir, staging_path)?;
    run_git_in(staging_path, &["add", "-A"])?;

    let tree_changed = if snapshot.main.is_some() {
        staged_tree_changed(staging_path)?
    } else {
        true
    };
    if tree_changed {
        let commit_msg = format!("Release {package_name}@{version}");
        run_git_in(staging_path, &["commit", "-m", &commit_msg])?;
    }
    let head = rev_parse(staging_path, "HEAD")?;

    // A retry of bytes that already occupy main and are already selected
    // by this version is fully idempotent: do not manufacture a commit,
    // tag object, or network write.
    if !tree_changed && snapshot.tag_target.as_deref() == Some(head.as_str()) {
        return Ok(());
    }

    // `-f` intentionally moves this one mutable version tag. No other tag
    // is fetched, created, deleted, or included in the push refspec.
    let tag_msg = format!("{package_name}@{version}");
    run_git_in(staging_path, &["tag", "-f", "-a", tag, "-m", &tag_msg])?;

    before_push(staging_path)?;
    push_release_refs(staging_path, clone_url, tag, &snapshot)?;

    Ok(())
}

fn release_ref_snapshot(clone_url: &str, tag: &str) -> Result<ReleaseRefSnapshot, PublishError> {
    let main_ref = "refs/heads/main";
    let tag_ref = format!("refs/tags/{tag}");
    let peeled_tag_ref = format!("{tag_ref}^{{}}");
    let args = [
        "ls-remote",
        "--",
        clone_url,
        main_ref,
        tag_ref.as_str(),
        peeled_tag_ref.as_str(),
    ];
    let output = git_command_in_temp(&args).output().map_err(|e| {
        PublishError::Git(format!(
            "spawning git ls-remote {}: {e}",
            redact_credentials(clone_url)
        ))
    })?;
    classify_remote_output(&output, clone_url, "ls-remote")?;

    let mut snapshot = ReleaseRefSnapshot::default();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut fields = line.split_whitespace();
        let Some(oid) = fields.next() else { continue };
        let Some(reference) = fields.next() else {
            continue;
        };
        if reference == main_ref {
            snapshot.main = Some(oid.to_string());
        } else if reference == tag_ref {
            snapshot.tag = Some(oid.to_string());
        } else if reference == peeled_tag_ref {
            snapshot.tag_target = Some(oid.to_string());
        }
    }
    // A lightweight tag has no peeled line; its ref value is already the
    // commit target. Treat it the same for identical-retry detection.
    if snapshot.tag_target.is_none() {
        snapshot.tag_target.clone_from(&snapshot.tag);
    }
    Ok(snapshot)
}

fn push_release_refs(
    staging_path: &Path,
    clone_url: &str,
    tag: &str,
    snapshot: &ReleaseRefSnapshot,
) -> Result<(), PublishError> {
    let main_lease = format!(
        "--force-with-lease=refs/heads/main:{}",
        snapshot.main.as_deref().unwrap_or_default()
    );
    let tag_ref = format!("refs/tags/{tag}");
    let tag_lease = format!(
        "--force-with-lease={tag_ref}:{}",
        snapshot.tag.as_deref().unwrap_or_default()
    );
    // The explicit tag lease supplies the force. A leading `+` here
    // would be an unconditional per-ref force and would bypass the very
    // compare-and-swap guard this publish protocol relies on.
    let tag_refspec = format!("{tag_ref}:{tag_ref}");
    let args = [
        "push",
        "--atomic",
        main_lease.as_str(),
        tag_lease.as_str(),
        "--",
        clone_url,
        "HEAD:refs/heads/main",
        tag_refspec.as_str(),
    ];
    push_with_classification(staging_path, &args, clone_url)
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

    push_with_classification(
        staging_path,
        &["push", "--", clone_url, "main:refs/heads/main"],
        clone_url,
    )?;

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
/// `staging_path` must be an existing local checkout of the stub remote
/// (so the tag can resolve to a known commit). The push targets
/// `clone_url` directly; no configured remote is required.
pub fn push_tag_only(staging_path: &Path, clone_url: &str, tag: &str) -> Result<(), PublishError> {
    let tag_msg = format!("redirect stub: surface target ref {tag}");
    run_git_in(staging_path, &["tag", "-a", tag, "-m", &tag_msg])?;
    let tag_refspec = format!("refs/tags/{tag}:refs/tags/{tag}");
    push_with_classification(
        staging_path,
        &["push", "--", clone_url, &tag_refspec],
        clone_url,
    )?;
    Ok(())
}

/// Stage every change in `working_dir`, commit with `commit_msg`, push to
/// `clone_url` on `main`. Used by `vibe registry redirect-update` to land
/// a rewritten `vibe-redirect.toml` into an already-existing stub repo
/// without re-creating it. The push is a fast-forward (no `--force`) — the
/// new marker is layered on top of the stub's existing history.
///
/// `working_dir` must be an existing local checkout of the remote with at
/// least one prior commit and identity already configured (see
/// [`shallow_clone`], which sets `user.email` / `user.name` after clone).
/// If `git status --porcelain` is empty (nothing staged), the call fails
/// with [`PublishError::Git`] rather than recording an empty commit —
/// callers are expected to short-circuit "nothing changed" upstream.
pub fn commit_and_push(
    working_dir: &Path,
    clone_url: &str,
    commit_msg: &str,
) -> Result<(), PublishError> {
    run_git_in(working_dir, &["add", "-A"])?;

    // Refuse to record an empty commit. If nothing staged, the caller
    // mis-computed the diff — surface that as a hard error rather than
    // emit a no-op commit on the stub's history.
    let status = run_git_in(working_dir, &["status", "--porcelain"])?;
    if status.stdout.iter().all(u8::is_ascii_whitespace) {
        return Err(PublishError::Git(
            "commit_and_push: working tree clean; nothing to commit".to_string(),
        ));
    }

    run_git_in(working_dir, &["commit", "-m", commit_msg])?;
    push_with_classification(
        working_dir,
        &["push", "--", clone_url, "main:refs/heads/main"],
        clone_url,
    )?;
    Ok(())
}

/// Fetch `clone_url` into a temporary working tree on `main` and return
/// it. Used by redirect-sync to obtain an existing stub before tagging
/// missing target tags into it. The fetch is shallow (`--depth=1`) — we
/// only need the `main` commit to anchor new tags onto. The URL is used
/// directly for the fetch and is not persisted as a configured remote.
pub fn shallow_clone(clone_url: &str) -> Result<TempDir, PublishError> {
    let staging = TempDir::new().map_err(|e| PublishError::Io {
        path: std::env::temp_dir(),
        message: format!("creating clone staging dir: {e}"),
    })?;
    run_git_in(staging.path(), &["init", "--initial-branch=main"])?;
    let fetched_ref = "refs/remotes/vibe-publish/main";
    let fetch_refspec = format!("refs/heads/main:{fetched_ref}");
    run_remote_git_in(
        staging.path(),
        &[
            "fetch",
            "--depth=1",
            "--no-tags",
            "--no-write-fetch-head",
            "--",
            clone_url,
            &fetch_refspec,
        ],
        clone_url,
    )?;
    run_git_in(staging.path(), &["checkout", "-B", "main", fetched_ref])?;
    // Set local identity (parallel to push_release / push_initial) so
    // tag annotation does not require a global git config.
    run_git_in(
        staging.path(),
        &["config", "user.email", "publish@vibevm.local"],
    )?;
    run_git_in(staging.path(), &["config", "user.name", "vibevm publisher"])?;
    Ok(staging)
}

/// Recursively copy `src` → `dst`. Skips `.git` and `.gitmodules` at any
/// depth: populated submodule bytes become ordinary files, so shipping the
/// declarations would leave a dangling and misleading Git boundary.
fn copy_dir(src: &Path, dst: &Path) -> Result<(), PublishError> {
    std::fs::create_dir_all(dst).map_err(|e| PublishError::Io {
        path: dst.to_path_buf(),
        message: format!("create_dir_all: {e}"),
    })?;
    for entry in walk(src)? {
        let path = entry;
        let rel = path
            .strip_prefix(src)
            .map_err(|_| PublishError::Io {
                path: path.clone(),
                message: format!("walked path escaped its copy root `{}`", src.display()),
            })?
            .to_path_buf();
        if rel.components().any(|c| {
            c.as_os_str() == std::ffi::OsStr::new(".git")
                || c.as_os_str() == std::ffi::OsStr::new(".gitmodules")
        }) {
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

fn clear_payload_tree(staging_path: &Path) -> Result<(), PublishError> {
    // This helper is intentionally destructive only inside the freshly
    // initialised temp repository. Requiring its `.git` sentinel makes an
    // accidental call against a source/package directory fail closed.
    if !staging_path.join(".git").is_dir() {
        return Err(PublishError::Git(format!(
            "refusing to clear publish staging directory without `.git`: {}",
            staging_path.display()
        )));
    }
    let entries = std::fs::read_dir(staging_path).map_err(|e| PublishError::Io {
        path: staging_path.to_path_buf(),
        message: format!("reading publish staging directory: {e}"),
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| PublishError::Io {
            path: staging_path.to_path_buf(),
            message: format!("reading publish staging entry: {e}"),
        })?;
        if entry.file_name() == std::ffi::OsStr::new(".git") {
            continue;
        }
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| PublishError::Io {
            path: path.clone(),
            message: format!("reading file type: {e}"),
        })?;
        let result = if file_type.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        result.map_err(|e| PublishError::Io {
            path,
            message: format!("clearing previous release payload: {e}"),
        })?;
    }
    Ok(())
}

fn rev_parse(cwd: &Path, reference: &str) -> Result<String, PublishError> {
    let output = run_git_in(cwd, &["rev-parse", "--verify", reference])?;
    let oid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if oid.is_empty() {
        return Err(PublishError::Git(format!(
            "git rev-parse returned an empty object id for `{reference}`"
        )));
    }
    Ok(oid)
}

#[cfg(test)]
#[path = "git_publish/tests.rs"]
mod tests;
