//! [`GitBackend`] implementation that shells out to the system `git`.
//!
//! Design decisions pinned in
//! [`spec/modules/vibe-registry/PROP-001-git-backend.md`][prop]:
//!
//! - Every spawn is wrapped with `LC_ALL=C` / `LANG=C` so stderr parsing
//!   is locale-invariant.
//! - On Windows, `CREATE_NO_WINDOW` is set on every spawn so a
//!   hostless parent process never flashes a console window.
//! - Error classification is substring-based against the stable
//!   C-locale stderr.
//!
//! [prop]: ../../../../../spec/modules/vibe-registry/PROP-001-git-backend.md

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-001#backend");

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use super::{GitBackend, GitError};

/// `git` subprocess backend.
///
/// Safe to share across threads: every operation constructs a fresh
/// [`Command`] and blocks on its output. The only shared state is the
/// immutable path to the `git` binary.
#[derive(Debug)]
pub struct ShellGit {
    binary: PathBuf,
    // Cached preflight result — populated on first `preflight()` call
    // for this instance. Kept per-instance (not global) so tests with
    // bogus binaries do not poison the cache for instances pointing at
    // a real `git`.
    preflight_cache: OnceLock<bool>,
}

impl Default for ShellGit {
    fn default() -> Self {
        ShellGit::new()
    }
}

impl ShellGit {
    /// Construct a [`ShellGit`] bound to `git` on `PATH`.
    ///
    /// Does **not** preflight — the cost is deferred to the first real
    /// operation, which fails fast with [`GitError::NotInstalled`] if
    /// git cannot be spawned. Call [`ShellGit::preflight`] once per
    /// `vibe` invocation to turn that into an up-front error.
    pub fn new() -> Self {
        // Allow `VIBE_GIT_BINARY` to override PATH lookup. See PROP-001 §6
        // for the rationale on env-var rather than CLI-flag.
        let binary = std::env::var_os("VIBE_GIT_BINARY")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("git"));
        ShellGit {
            binary,
            preflight_cache: OnceLock::new(),
        }
    }

    /// Verify that the configured git binary responds to `--version`.
    ///
    /// Caches the result on this instance. Subsequent calls are free.
    pub fn preflight(&self) -> Result<(), GitError> {
        let ok = *self.preflight_cache.get_or_init(|| {
            let mut cmd = Command::new(&self.binary);
            apply_common_env(&mut cmd);
            cmd.arg("--version");
            cmd.output().map(|o| o.status.success()).unwrap_or(false)
        });
        if ok {
            Ok(())
        } else {
            Err(GitError::NotInstalled)
        }
    }

    fn run(&self, args: &[&str], cwd: Option<&Path>) -> Result<Output, GitError> {
        let mut cmd = Command::new(&self.binary);
        // apply_common_env may push `-c <key>=<value>` flags that must
        // come BEFORE the subcommand — push the env / global-flag
        // layer first, then the actual git subcommand args.
        apply_common_env(&mut cmd);
        cmd.args(args);
        if let Some(d) = cwd {
            cmd.current_dir(d);
        }

        tracing::debug!(target: "vibe_registry::git", argv = ?render_argv(&self.binary, args), cwd = ?cwd, "running git");

        let output = cmd.output().map_err(|e| GitError::Io {
            cmd: render_argv(&self.binary, args),
            source: e,
        })?;
        if output.status.success() {
            return Ok(output);
        }
        Err(classify_failure(args, &output))
    }

    /// Like [`Self::run`] but returns the raw [`Output`] on non-zero exit
    /// without classifying it. Used by callers that need to look at
    /// stdout / stderr together (e.g. `fetch_file_at_ref` distinguishing
    /// "ref missing" from "file missing in ref" from "archive
    /// unsupported").
    fn run_raw(&self, args: &[&str], cwd: Option<&Path>) -> Result<Output, GitError> {
        let mut cmd = Command::new(&self.binary);
        apply_common_env(&mut cmd);
        cmd.args(args);
        if let Some(d) = cwd {
            cmd.current_dir(d);
        }

        tracing::debug!(target: "vibe_registry::git", argv = ?render_argv(&self.binary, args), cwd = ?cwd, "running git (raw)");

        cmd.output().map_err(|e| GitError::Io {
            cmd: render_argv(&self.binary, args),
            source: e,
        })
    }
}

impl GitBackend for ShellGit {
    fn bootstrap(&self, url: &str, refname: &str, dest: &Path) -> Result<(), GitError> {
        self.preflight()?;
        let dest_s = dest.to_string_lossy();
        let args = ["clone", "--branch", refname, "--", url, dest_s.as_ref()];
        self.run(&args, None).map(|_| ())
    }

    fn update(&self, dest: &Path, refname: &str) -> Result<(), GitError> {
        self.preflight()?;
        // Fetch from origin, including tags, then hard-reset the working
        // tree to the fetched tip. `--hard` is intentional: the registry
        // cache is a read-only mirror and we want it to match upstream
        // exactly, never a surprise merge commit. `--tags` is required
        // because the per-package registry shape (PROP-002 §2.5) uses
        // tags as versions — without it, freshly-published versions are
        // invisible to a previously-bootstrapped clone.
        self.run(&["fetch", "--prune", "--tags", "origin"], Some(dest))?;
        // Try the tag-form first (PROP-002 §2.5: versions are git tags),
        // then fall back to the branch-form (legacy GitRegistry path,
        // and registry-level metadata refs). `refs/tags/<name>` and
        // `origin/<name>` are both unambiguous fully-qualified refs;
        // git resolves them without the heuristic-driven ambiguity of a
        // bare `<name>`. The fallback chain MUST stay in this order
        // because a `vN.M.K`-shaped tag is what every per-package repo
        // ships under M1.1-revision.
        let tag_ref = format!("refs/tags/{refname}");
        if self.run(&["reset", "--hard", &tag_ref], Some(dest)).is_ok() {
            return Ok(());
        }
        let branch_ref = format!("origin/{refname}");
        self.run(&["reset", "--hard", &branch_ref], Some(dest))
            .map(|_| ())
    }

    fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
        self.preflight()?;
        // `git ls-remote --tags <url>` outputs one line per ref:
        //   <hash>\trefs/tags/<name>
        // Annotated tags appear twice — once as the tag object and once
        // as the peeled commit (`refs/tags/<name>^{}`). We drop the
        // peeled form's `^{}` suffix and dedupe on the resulting names.
        let output = self.run(&["ls-remote", "--tags", "--", url], None)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tags: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for line in stdout.lines() {
            let Some((_hash, refpath)) = line.split_once('\t') else {
                continue;
            };
            let Some(name) = refpath.strip_prefix("refs/tags/") else {
                continue;
            };
            // Strip peeled-form suffix.
            let name = name.strip_suffix("^{}").unwrap_or(name).to_string();
            if seen.insert(name.clone()) {
                tags.push(name);
            }
        }
        Ok(tags)
    }

    fn fetch_file_at_ref(&self, url: &str, refname: &str, path: &str) -> Result<Vec<u8>, GitError> {
        self.preflight()?;
        // Normalise platform separators to forward slash — `git archive`
        // expects in-repo paths in posix form.
        let normalized = path.replace('\\', "/");

        // `git archive --remote=<url> --format=tar <refname> -- <path>`
        // emits a tar of just the requested path on stdout. We block on
        // the full output (these are tiny files — manifests under a kB).
        let remote_arg = format!("--remote={url}");
        let format_arg = "--format=tar";
        let args = [
            "archive",
            &remote_arg,
            format_arg,
            refname,
            "--",
            normalized.as_str(),
        ];
        let output = self.run_raw(&args, None)?;
        if output.status.success() {
            return extract_single_file_from_tar(&output.stdout, &normalized).ok_or_else(|| {
                GitError::FileNotFoundInRef {
                    url: url.to_string(),
                    refname: refname.to_string(),
                    path: normalized.clone(),
                }
            });
        }

        // Classify failure.
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined_lc = format!("{stderr}{stdout}").to_lowercase();

        if combined_lc.contains("path") && combined_lc.contains("does not exist")
            || combined_lc.contains("pathspec")
            || combined_lc.contains("did not match any files")
        {
            return Err(GitError::FileNotFoundInRef {
                url: url.to_string(),
                refname: refname.to_string(),
                path: normalized,
            });
        }
        if combined_lc.contains("operation not supported by protocol")
            || combined_lc.contains("upload-archive")
                && (combined_lc.contains("not permitted")
                    || combined_lc.contains("not allowed")
                    || combined_lc.contains("disabled"))
        {
            return Err(GitError::ArchiveUnsupported {
                url: url.to_string(),
            });
        }
        // GitHub disables `upload-archive` server-side. The HTTPS smart
        // protocol response is `HTTP 422` with the local git complaining
        // `expected ACK/NAK, got a flush packet` (verified 2026-04-29).
        // Treat that pattern as ArchiveUnsupported so callers fall back
        // to a clone — the same path other Gitea-clones / Forgejo /
        // SourceHut take when they don't expose `upload-archive` either.
        if (combined_lc.contains("http 422") || combined_lc.contains("error: 422"))
            && combined_lc.contains("git archive")
        {
            return Err(GitError::ArchiveUnsupported {
                url: url.to_string(),
            });
        }
        if combined_lc.contains("git archive")
            && combined_lc.contains("expected ack/nak")
            && combined_lc.contains("flush packet")
        {
            return Err(GitError::ArchiveUnsupported {
                url: url.to_string(),
            });
        }
        if combined_lc.contains("unknown revision")
            || combined_lc.contains("not a tree object")
            || combined_lc.contains("couldn't find remote ref")
        {
            return Err(GitError::RefNotFound {
                url: url.to_string(),
                refname: refname.to_string(),
            });
        }
        Err(classify_failure(&args, &output))
    }

    fn set_remote_url(&self, dest: &Path, remote: &str, url: &str) -> Result<(), GitError> {
        self.preflight()?;
        // `git -C <dest> remote set-url <remote> <url>`. The
        // `dest`-via-`-C` form is exactly what `self.run(.., Some(dest))`
        // produces (it sets `current_dir`), but `remote set-url` is
        // a config write, not a ref-network operation, so it does
        // not need any of the auth handling — `apply_common_env`
        // still applies for stable error parsing and Windows
        // window-suppression.
        self.run(&["remote", "set-url", remote, url], Some(dest))
            .map(|_| ())
    }
}

/// Pull a single file's bytes out of a tar stream. Returns `None` if the
/// requested path is not present.
///
/// Implemented inline (no `tar` crate) because the data shape is trivial:
/// a tar archive is a sequence of 512-byte headers, each followed by
/// `ceil(size / 512) * 512` bytes of payload, terminated by two empty
/// headers. We read filename, size, payload; skip over directory and
/// other-type entries.
fn extract_single_file_from_tar(bytes: &[u8], target_path: &str) -> Option<Vec<u8>> {
    let target_norm = target_path.trim_start_matches("./");
    let mut offset = 0usize;
    while offset + 512 <= bytes.len() {
        let header = &bytes[offset..offset + 512];
        // Empty header marks end-of-archive.
        if header.iter().all(|b| *b == 0) {
            return None;
        }

        // Filename is the first 100 bytes, NUL-terminated. Optionally
        // prefixed (UStar long-name extension via `prefix` field at
        // bytes 345..500), but git archive emits short paths.
        let name = read_cstr(&header[0..100]);
        let prefix = read_cstr(&header[345..500]);
        let full_name = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let full_norm = full_name.trim_start_matches("./").to_string();

        // Size is octal in bytes 124..136.
        let size = parse_octal(&header[124..136]).unwrap_or(0);

        // Type flag at byte 156: '0' or '\0' = regular file.
        let typeflag = header[156];

        let payload_start = offset + 512;
        let payload_end = payload_start + size;
        if payload_end > bytes.len() {
            return None;
        }

        let is_regular = typeflag == b'0' || typeflag == 0;
        if is_regular && full_norm == target_norm {
            return Some(bytes[payload_start..payload_end].to_vec());
        }

        // Advance past payload, rounded up to 512.
        let padded = size.div_ceil(512) * 512;
        offset = payload_start + padded;
    }
    None
}

fn read_cstr(buf: &[u8]) -> String {
    let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

fn parse_octal(buf: &[u8]) -> Option<usize> {
    let s = std::str::from_utf8(buf).ok()?;
    let trimmed = s.trim_matches(|c: char| c == ' ' || c == '\0');
    if trimmed.is_empty() {
        return Some(0);
    }
    usize::from_str_radix(trimmed, 8).ok()
}

/// Apply environment + global git flags that every spawned git
/// invocation needs. Two layers:
///
/// 1. **Always.** `LC_ALL=C` / `LANG=C` for stable error parsing;
///    `GIT_TERMINAL_PROMPT=0` so terminal-style prompts don't block
///    a non-TTY parent; on Windows `CREATE_NO_WINDOW` so a hostless
///    parent doesn't flash a console.
/// 2. **TTY-aware.** When running unattended (no stdin TTY OR
///    `VIBE_UNATTENDED` env-var truthy), suppress every channel git
///    might use for *interactive* credential entry: GCM popups
///    (`GCM_INTERACTIVE=Never`), system credential helpers
///    (`-c credential.helper=`), `core.askPass` (`-c core.askPass=`),
///    and `GIT_ASKPASS` (set to empty). This is the policy from
///    PROP-002 §2.2.1: in a scripted run a 401 must not become a
///    blocking GUI prompt; the resolver instead classifies it as an
///    `UnknownPackage` (for `auth = "none"` registries) or as an
///    `AuthFailed` halt (for `auth = "token-env"` /
///    `"credential-helper"` registries).
///
/// **Layering note.** This function MUST be called BEFORE the
/// caller pushes the actual git subcommand args, because the
/// silencing layer prepends `-c <key>=<value>` flags that have to
/// come before the subcommand. The `[run, run_raw, preflight]`
/// methods on this module already follow that order.
///
/// Override: `VIBEVM_GIT_SILENCE_HELPERS=1` forces silencing on
/// regardless of TTY; `VIBEVM_GIT_SILENCE_HELPERS=0` forces it off.
/// Neither value is the typical case — the TTY-derived default
/// covers most operators.
fn apply_common_env(cmd: &mut Command) {
    cmd.env("LC_ALL", "C").env("LANG", "C");
    cmd.env("GIT_TERMINAL_PROMPT", "0");

    if should_silence_credential_helpers() {
        cmd.env("GCM_INTERACTIVE", "Never");
        // Reset both ends of the system credential machinery for the
        // duration of this invocation. `credential.helper=` empty
        // disables every helper configured in `~/.gitconfig` /
        // `/etc/gitconfig`; `core.askPass=` empty stops git from
        // exec'ing a binary asker. `GIT_ASKPASS` is deliberately
        // NOT set here — on some platforms an empty string for
        // that variable can confuse git's startup probe of the
        // path; clearing the two git-config keys plus the
        // already-set `GIT_TERMINAL_PROMPT=0` is sufficient to
        // close every interactive avenue.
        cmd.arg("-c").arg("credential.helper=");
        cmd.arg("-c").arg("core.askPass=");
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
}

/// Decide whether to silence interactive credential mechanisms.
/// Resolution order:
///
/// 1. `VIBEVM_GIT_SILENCE_HELPERS` env-var — explicit operator
///    override (truthy / falsy vocabulary same as `VIBE_UNATTENDED`:
///    `1`, `true`, `yes`, `on` for true; everything else for false).
/// 2. `VIBE_UNATTENDED` env-var truthy — global unattended posture
///    (PROP-002 §2.2.1 + `vibe --unattended` flag).
/// 3. stdin not a TTY — we are running under a non-interactive
///    parent (CI, service, opencode harness).
///
/// Otherwise (interactive TTY without `--unattended`), leave git
/// alone — an operator running `vibe install` at a real terminal
/// might genuinely want to type a password.
fn should_silence_credential_helpers() -> bool {
    if let Ok(v) = std::env::var("VIBEVM_GIT_SILENCE_HELPERS") {
        return is_truthy(v.trim());
    }
    if let Ok(v) = std::env::var("VIBE_UNATTENDED")
        && is_truthy(v.trim())
    {
        return true;
    }
    use std::io::IsTerminal;
    !std::io::stdin().is_terminal()
}

fn is_truthy(s: &str) -> bool {
    matches!(s.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

fn classify_failure(args: &[&str], output: &Output) -> GitError {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}{stdout}");

    // Extract `--` followed by URL for clone. For fetch, URL is from
    // origin which we don't know here; fall back to an empty string.
    let url = args
        .iter()
        .skip_while(|a| **a != "--")
        .nth(1)
        .map(|s| s.to_string())
        .unwrap_or_default();

    let refname = args
        .iter()
        .skip_while(|a| **a != "--branch")
        .nth(1)
        .map(|s| s.to_string())
        .unwrap_or_default();

    classify_stderr_message(&combined, url, refname).unwrap_or_else(|| GitError::CommandFailed {
        cmd: render_argv_for_display(args),
        status: output.status.code().unwrap_or(-1),
        stderr: combined.trim_end().to_string(),
    })
}

/// Pure substring-driven classifier. Receives the combined stderr+stdout
/// of a failed `git` invocation plus the URL/refname extracted from the
/// argv, and returns the matching `GitError` variant — or `None` to let
/// the caller fall back to `CommandFailed { cmd, status, stderr }`.
///
/// Order of checks is significant: more specific matchers run before
/// broader ones (e.g. `unable to access` is a wrapper that frames many
/// of the lower-level failures, so we look for the inner connect-failure
/// substrings first and let `unable to access` ride along on whatever
/// they found).
fn classify_stderr_message(combined: &str, url: String, refname: String) -> Option<GitError> {
    let lc = combined.to_lowercase();

    if lc.contains("repository not found") || lc.contains("does not appear to be a git repository")
    {
        return Some(GitError::RepoNotFound { url });
    }
    // Authentication-required signals. Three families:
    //
    //   1. ssh / direct refusal — `Permission denied (publickey).`
    //      / `fatal: Authentication failed`.
    //   2. HTTPS without working credentials — git tried to ask the
    //      operator (or a credential helper) for username/password
    //      and could not get one. With our `GIT_TERMINAL_PROMPT=0` +
    //      silenced helpers (PROP-002 §2.2.1), git prints
    //      `fatal: could not read Username for '<url>'` /
    //      `fatal: could not read Password for ...` and exits
    //      non-zero. Some credential helpers also leave a `User
    //      cancelled dialog.` line on stderr when their GUI
    //      window is dismissed (the original GCM popup case).
    //   3. Direct HTTP status codes from the underlying transport —
    //      `HTTP 401` / `HTTP 403` / `401 Unauthorized` /
    //      `403 Forbidden`. These appear when the host returns a
    //      structured response without redirecting through the
    //      credential layer (some proxies, some CI runners).
    if lc.contains("permission denied (publickey)")
        || lc.contains("authentication failed")
        || lc.contains("could not read username")
        || lc.contains("could not read password")
        || lc.contains("user cancelled dialog")
        || lc.contains("http 401")
        || lc.contains("http 403")
        || lc.contains("401 unauthorized")
        || lc.contains("403 forbidden")
    {
        return Some(GitError::AuthFailed { url });
    }
    // Network / connect-failure shapes seen in the wild from git 2.x +
    // libcurl. `failed to connect` / `could not connect to server` cover
    // the curl path (the wording flipped over the 7.x → 8.x transition);
    // `connection refused` / `connection timed out` / `operation timed
    // out` cover bare-TCP and proxy paths; `could not resolve host` /
    // `network is unreachable` are the historical entries; `could not
    // read from remote repository` is the SSH-side equivalent that ssh
    // emits before git can class it any tighter.
    if lc.contains("could not resolve host")
        || lc.contains("could not read from remote repository")
        || lc.contains("network is unreachable")
        || lc.contains("failed to connect")
        || lc.contains("could not connect to")
        || lc.contains("connection refused")
        || lc.contains("connection timed out")
        || lc.contains("operation timed out")
    {
        return Some(GitError::NetworkUnreachable { url });
    }
    if lc.contains("remote branch") && lc.contains("not found")
        || lc.contains("couldn't find remote ref")
    {
        return Some(GitError::RefNotFound { url, refname });
    }

    None
}

fn render_argv(binary: &Path, args: &[&str]) -> String {
    let mut out = OsString::from(binary);
    for a in args {
        out.push(" ");
        out.push(a);
    }
    out.to_string_lossy().into_owned()
}

fn render_argv_for_display(args: &[&str]) -> String {
    let mut out = String::from("git");
    for a in args {
        out.push(' ');
        out.push_str(a);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    /// Build a bare git repo under `root/origin.git` seeded with one
    /// commit on `main`, and return its absolute path. Requires `git`
    /// on `PATH`; tests that need it skip themselves via
    /// `skip_without_git!()` below.
    fn make_bare_origin(root: &Path) -> PathBuf {
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        // Work repo: init, set identity, create a file, commit on main.
        run_or_panic(&src, &["init", "--initial-branch=main"]);
        run_or_panic(&src, &["config", "user.email", "t@example.com"]);
        run_or_panic(&src, &["config", "user.name", "Test"]);
        fs::write(src.join("README.md"), "hello\n").unwrap();
        run_or_panic(&src, &["add", "README.md"]);
        run_or_panic(&src, &["commit", "-m", "init"]);

        let bare = root.join("origin.git");
        run_or_panic(
            root,
            &[
                "clone",
                "--bare",
                src.to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        );
        // Make sure HEAD in the bare repo points at main.
        run_or_panic(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);

        bare
    }

    fn run_or_panic(cwd: &Path, args: &[&str]) {
        let mut cmd = Command::new("git");
        apply_common_env(&mut cmd);
        cmd.args(args);
        cmd.current_dir(cwd);
        let out = cmd.output().expect("failed to spawn git for test setup");
        if !out.status.success() {
            panic!(
                "test setup `git {}` failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    macro_rules! skip_without_git {
        () => {
            if !git_available() {
                eprintln!("skipping test: git not on PATH");
                return;
            }
        };
    }

    #[test]
    fn preflight_succeeds_when_git_installed() {
        skip_without_git!();
        let g = ShellGit::new();
        g.preflight().expect("preflight should succeed");
    }

    #[test]
    fn preflight_reports_not_installed_for_bogus_binary() {
        let g = ShellGit {
            binary: PathBuf::from("definitely-not-git-xyz"),
            preflight_cache: OnceLock::new(),
        };
        let err = g.preflight().unwrap_err();
        assert!(
            matches!(err, GitError::NotInstalled),
            "expected NotInstalled, got: {err:?}"
        );
    }

    #[test]
    fn clone_then_update_against_bare_origin() {
        skip_without_git!();
        let tmp = tempdir().unwrap();
        let bare = make_bare_origin(tmp.path());
        let dest = tmp.path().join("clone");

        let g = ShellGit::new();
        g.bootstrap(&bare.to_string_lossy(), "main", &dest)
            .expect("initial clone should succeed");
        assert!(dest.join("README.md").exists());

        // Push a new commit into origin, then update from the clone.
        let src2 = tmp.path().join("src2");
        run_or_panic(
            tmp.path(),
            &["clone", bare.to_str().unwrap(), src2.to_str().unwrap()],
        );
        run_or_panic(&src2, &["config", "user.email", "t@example.com"]);
        run_or_panic(&src2, &["config", "user.name", "Test"]);
        fs::write(src2.join("new.md"), "new\n").unwrap();
        run_or_panic(&src2, &["add", "new.md"]);
        run_or_panic(&src2, &["commit", "-m", "add new"]);
        run_or_panic(&src2, &["push", "origin", "main"]);

        g.update(&dest, "main").expect("update should succeed");
        assert!(dest.join("new.md").exists());
    }

    #[test]
    fn clone_reports_repo_not_found_for_missing_url() {
        skip_without_git!();
        let tmp = tempdir().unwrap();
        let bogus = tmp.path().join("does/not/exist.git");
        let dest = tmp.path().join("clone");

        let g = ShellGit::new();
        let err = g
            .bootstrap(&bogus.to_string_lossy(), "main", &dest)
            .unwrap_err();
        // The exact message varies by git version; classification should
        // land on either RepoNotFound or a generic CommandFailed with
        // the raw stderr — both are acceptable for this test.
        match err {
            GitError::RepoNotFound { .. } | GitError::CommandFailed { .. } => {}
            other => panic!("unexpected classification: {other:?}"),
        }
    }

    #[test]
    fn clone_reports_ref_not_found_for_missing_branch() {
        skip_without_git!();
        let tmp = tempdir().unwrap();
        let bare = make_bare_origin(tmp.path());
        let dest = tmp.path().join("clone");

        let g = ShellGit::new();
        let err = g
            .bootstrap(&bare.to_string_lossy(), "no-such-branch", &dest)
            .unwrap_err();
        match err {
            GitError::RefNotFound { .. } | GitError::CommandFailed { .. } => {}
            other => panic!("unexpected classification: {other:?}"),
        }
    }

    /// Build a bare origin that has multiple tags (`v0.1.0`, `v0.2.0`,
    /// `v1.0.0-rc.1`) plus one annotated tag (`v0.3.0`) so we exercise
    /// the peeled-form deduplication.
    fn make_bare_origin_with_tags(root: &Path) -> PathBuf {
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        run_or_panic(&src, &["init", "--initial-branch=main"]);
        run_or_panic(&src, &["config", "user.email", "t@example.com"]);
        run_or_panic(&src, &["config", "user.name", "Test"]);

        // Commit 1 + lightweight tag v0.1.0.
        fs::write(
            src.join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        run_or_panic(&src, &["add", "vibe.toml"]);
        run_or_panic(&src, &["commit", "-m", "0.1.0"]);
        run_or_panic(&src, &["tag", "v0.1.0"]);

        // Commit 2 + lightweight tag v0.2.0.
        fs::write(
            src.join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"0.2.0\"\n",
        )
        .unwrap();
        run_or_panic(&src, &["add", "vibe.toml"]);
        run_or_panic(&src, &["commit", "-m", "0.2.0"]);
        run_or_panic(&src, &["tag", "v0.2.0"]);

        // Commit 3 + ANNOTATED tag v0.3.0 (this is the one that produces
        // a peeled `^{}` line in `ls-remote --tags` output).
        fs::write(
            src.join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"0.3.0\"\n",
        )
        .unwrap();
        run_or_panic(&src, &["add", "vibe.toml"]);
        run_or_panic(&src, &["commit", "-m", "0.3.0"]);
        run_or_panic(&src, &["tag", "-a", "v0.3.0", "-m", "release 0.3.0"]);

        // Commit 4 + tag v1.0.0-rc.1.
        fs::write(
            src.join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"1.0.0-rc.1\"\n",
        )
        .unwrap();
        run_or_panic(&src, &["add", "vibe.toml"]);
        run_or_panic(&src, &["commit", "-m", "1.0.0-rc.1"]);
        run_or_panic(&src, &["tag", "v1.0.0-rc.1"]);

        let bare = root.join("origin.git");
        run_or_panic(
            root,
            &[
                "clone",
                "--bare",
                src.to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        );
        run_or_panic(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        bare
    }

    #[test]
    fn list_tags_returns_dedup_sorted_set() {
        skip_without_git!();
        let tmp = tempdir().unwrap();
        let bare = make_bare_origin_with_tags(tmp.path());

        let g = ShellGit::new();
        let mut tags = g.list_tags(&bare.to_string_lossy()).expect("list_tags ok");
        tags.sort();

        assert_eq!(
            tags,
            vec![
                "v0.1.0".to_string(),
                "v0.2.0".to_string(),
                "v0.3.0".to_string(),
                "v1.0.0-rc.1".to_string(),
            ],
            "annotated tag v0.3.0 must appear exactly once (peeled-form deduped)"
        );
    }

    #[test]
    fn list_tags_empty_repo_returns_empty() {
        skip_without_git!();
        let tmp = tempdir().unwrap();
        let bare = make_bare_origin(tmp.path()); // has no tags
        let g = ShellGit::new();
        let tags = g.list_tags(&bare.to_string_lossy()).expect("list_tags ok");
        assert!(tags.is_empty());
    }

    #[test]
    fn list_tags_reports_repo_not_found_for_missing_url() {
        skip_without_git!();
        let tmp = tempdir().unwrap();
        let bogus = tmp.path().join("does/not/exist.git");
        let g = ShellGit::new();
        let err = g.list_tags(&bogus.to_string_lossy()).unwrap_err();
        match err {
            GitError::RepoNotFound { .. } | GitError::CommandFailed { .. } => {}
            other => panic!("unexpected classification: {other:?}"),
        }
    }

    #[test]
    fn fetch_file_at_ref_returns_bytes_for_existing_file() {
        skip_without_git!();
        let tmp = tempdir().unwrap();
        let bare = make_bare_origin_with_tags(tmp.path());

        let g = ShellGit::new();
        let bytes = g
            .fetch_file_at_ref(&bare.to_string_lossy(), "v0.2.0", "vibe.toml")
            .expect("fetch ok");
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("version = \"0.2.0\""), "got: {text}");
    }

    #[test]
    fn fetch_file_at_ref_works_against_annotated_tag() {
        skip_without_git!();
        let tmp = tempdir().unwrap();
        let bare = make_bare_origin_with_tags(tmp.path());

        let g = ShellGit::new();
        let bytes = g
            .fetch_file_at_ref(&bare.to_string_lossy(), "v0.3.0", "vibe.toml")
            .expect("fetch via annotated tag ok");
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("version = \"0.3.0\""));
    }

    #[test]
    fn fetch_file_at_ref_normalises_backslash_paths() {
        skip_without_git!();
        let tmp = tempdir().unwrap();
        let bare = make_bare_origin_with_tags(tmp.path());

        let g = ShellGit::new();
        // Caller hands us a Windows-style path; the backend should
        // normalise to forward slash before talking to `git archive`.
        let bytes = g
            .fetch_file_at_ref(&bare.to_string_lossy(), "v0.1.0", "vibe.toml")
            .expect("fetch ok");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn fetch_file_at_ref_missing_ref() {
        skip_without_git!();
        let tmp = tempdir().unwrap();
        let bare = make_bare_origin_with_tags(tmp.path());

        let g = ShellGit::new();
        let err = g
            .fetch_file_at_ref(&bare.to_string_lossy(), "v9.9.9", "vibe.toml")
            .unwrap_err();
        match err {
            GitError::RefNotFound { .. } | GitError::CommandFailed { .. } => {}
            other => panic!("unexpected classification: {other:?}"),
        }
    }

    #[test]
    fn fetch_file_at_ref_missing_file_in_existing_ref() {
        skip_without_git!();
        let tmp = tempdir().unwrap();
        let bare = make_bare_origin_with_tags(tmp.path());

        let g = ShellGit::new();
        let err = g
            .fetch_file_at_ref(&bare.to_string_lossy(), "v0.1.0", "no-such-file.txt")
            .unwrap_err();
        match err {
            GitError::FileNotFoundInRef { .. } | GitError::CommandFailed { .. } => {}
            other => panic!("unexpected classification: {other:?}"),
        }
    }

    #[test]
    fn extract_single_file_from_tar_picks_match() {
        // Hand-build a minimal tar with two files; verify we extract the
        // requested one by name, ignoring the other.
        let tar = build_tar(&[("a.txt", b"AAA\n"), ("vibe.toml", b"hello world\n")]);
        let got = extract_single_file_from_tar(&tar, "vibe.toml").expect("file extracted");
        assert_eq!(got, b"hello world\n");

        let absent = extract_single_file_from_tar(&tar, "nope.txt");
        assert!(absent.is_none());
    }

    #[test]
    fn extract_single_file_from_tar_handles_dot_slash_prefix() {
        let tar = build_tar(&[("./vibe.toml", b"prefixed\n")]);
        let got = extract_single_file_from_tar(&tar, "vibe.toml").unwrap();
        assert_eq!(got, b"prefixed\n");
    }

    /// Build a USTar archive from `(name, bytes)` pairs. Plenty for our
    /// tests; not a complete tar implementation.
    fn build_tar(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        for (name, data) in entries {
            let mut header = vec![0u8; 512];
            // Name: bytes 0..100, NUL-terminated.
            let nb = name.as_bytes();
            let len = nb.len().min(100);
            header[0..len].copy_from_slice(&nb[..len]);
            // Mode: bytes 100..108 — "0000644\0".
            header[100..108].copy_from_slice(b"0000644\0");
            // UID/GID: bytes 108..116 / 116..124 — "0000000\0".
            header[108..116].copy_from_slice(b"0000000\0");
            header[116..124].copy_from_slice(b"0000000\0");
            // Size: bytes 124..136 — octal, 11 chars + NUL.
            let size_str = format!("{:011o}\0", data.len());
            header[124..136].copy_from_slice(size_str.as_bytes());
            // Mtime: bytes 136..148 — "00000000000\0".
            header[136..148].copy_from_slice(b"00000000000\0");
            // Checksum: bytes 148..156 — fill with spaces, compute later.
            for b in &mut header[148..156] {
                *b = b' ';
            }
            // Typeflag: byte 156 — '0' (regular file).
            header[156] = b'0';
            // Magic: bytes 257..263 — "ustar\0".
            header[257..263].copy_from_slice(b"ustar\0");
            // Version: bytes 263..265 — "00".
            header[263..265].copy_from_slice(b"00");
            // Compute checksum: sum of all bytes treating chksum field
            // as spaces (already done above).
            let cksum: u32 = header.iter().map(|b| *b as u32).sum();
            let cksum_str = format!("{cksum:06o}\0 ");
            header[148..156].copy_from_slice(cksum_str.as_bytes());

            out.extend_from_slice(&header);
            out.extend_from_slice(data);
            // Pad to 512.
            let pad = (512 - (data.len() % 512)) % 512;
            out.extend(std::iter::repeat_n(0, pad));
        }
        // Two empty 512-byte blocks to terminate.
        out.extend(std::iter::repeat_n(0, 1024));
        out
    }

    #[test]
    fn parse_octal_handles_padded_sizes() {
        assert_eq!(parse_octal(b"00000000027\0").unwrap(), 0o27);
        assert_eq!(parse_octal(b"  144 \0").unwrap(), 0o144);
        assert_eq!(parse_octal(b"\0\0\0\0\0\0\0\0\0\0\0\0").unwrap(), 0);
    }

    fn classify(stderr: &str) -> Option<GitError> {
        classify_stderr_message(stderr, "URL".into(), "REF".into())
    }

    #[test]
    fn classify_repo_not_found_message() {
        // GitHub / Gitea-shape 404 surfaces a verbatim "Repository not
        // found." line from the remote helper; that's the substring
        // the classifier locks onto.
        assert!(matches!(
            classify("remote: Repository not found.\nfatal: ...").unwrap(),
            GitError::RepoNotFound { .. }
        ));
        assert!(matches!(
            classify("fatal: 'x' does not appear to be a git repository").unwrap(),
            GitError::RepoNotFound { .. }
        ));
    }

    #[test]
    fn classify_auth_failed_message() {
        assert!(matches!(
            classify("git@github.com: Permission denied (publickey).").unwrap(),
            GitError::AuthFailed { .. }
        ));
        assert!(matches!(
            classify("remote: HTTP Basic: Access denied\nfatal: Authentication failed for ...")
                .unwrap(),
            GitError::AuthFailed { .. }
        ));
    }

    #[test]
    fn classify_credential_prompt_failure_after_silencing() {
        // PROP-002 §2.2.1: when our credential helpers are silenced
        // (`-c credential.helper=` + `GIT_TERMINAL_PROMPT=0`), git
        // can't ask anyone for a username/password and emits
        // `fatal: could not read Username for '...'`. This is what
        // the original opencode walk against GitVerse produced.
        // Must classify as AuthFailed so the resolver can apply the
        // per-`auth` walk-vs-halt rule (§2.3.1).
        for stderr in [
            "fatal: User cancelled dialog.\nfatal: could not read Username for 'https://gitverse.ru': terminal prompts disabled",
            "fatal: could not read Password for 'https://example.invalid': terminal prompts disabled",
        ] {
            assert!(
                matches!(classify(stderr).unwrap(), GitError::AuthFailed { .. }),
                "expected AuthFailed for: {stderr}"
            );
        }
    }

    #[test]
    fn classify_http_status_codes() {
        // Direct HTTP transport errors — when the host returns a
        // structured response without redirecting through the
        // credential layer (some proxies, some CI runners).
        for stderr in [
            "fatal: unable to access 'https://x/y.git/': The requested URL returned error: 401 Unauthorized",
            "fatal: unable to access 'https://x/y.git/': HTTP 401",
            "fatal: unable to access 'https://x/y.git/': The requested URL returned error: 403 Forbidden",
            "fatal: unable to access 'https://x/y.git/': HTTP 403",
        ] {
            assert!(
                matches!(classify(stderr).unwrap(), GitError::AuthFailed { .. }),
                "expected AuthFailed for: {stderr}"
            );
        }
    }

    #[test]
    fn classify_network_unreachable_classic_substrings() {
        // Substrings that the M1.1 classifier already recognised.
        for stderr in [
            "fatal: unable to access 'https://x/y.git/': Could not resolve host: x",
            "fatal: Could not read from remote repository.",
            "ssh: connect to host x port 22: Network is unreachable",
        ] {
            assert!(
                matches!(
                    classify(stderr).unwrap(),
                    GitError::NetworkUnreachable { .. }
                ),
                "expected NetworkUnreachable for: {stderr}"
            );
        }
    }

    #[test]
    fn classify_network_unreachable_connect_failure_substrings() {
        // The shapes M1.6 Scenario B4 surfaced — connect-failure on a
        // dead host. git 2.5x with libcurl 8.x emits the verbatim
        // strings below; the Scenario B4 walk on 2026-05-04 produced
        // the third one (`Could not connect to server`).
        for stderr in [
            "fatal: unable to access 'https://x/y.git/': Failed to connect to x port 443 after 2123 ms: Could not connect to server",
            "fatal: unable to access 'https://x/y.git/': Failed to connect to x port 443: Connection refused",
            "fatal: unable to access 'https://x/y.git/': Connection timed out after 30001 ms",
            "fatal: unable to access 'https://x/y.git/': Operation timed out after 30001 ms",
        ] {
            assert!(
                matches!(
                    classify(stderr).unwrap(),
                    GitError::NetworkUnreachable { .. }
                ),
                "expected NetworkUnreachable for: {stderr}"
            );
        }
    }

    #[test]
    fn classify_ref_not_found_message() {
        assert!(matches!(
            classify("fatal: Remote branch no-such-branch not found in upstream origin").unwrap(),
            GitError::RefNotFound { .. }
        ));
        assert!(matches!(
            classify("fatal: couldn't find remote ref refs/tags/v9.9.9").unwrap(),
            GitError::RefNotFound { .. }
        ));
    }

    #[test]
    fn classify_unknown_message_falls_through() {
        assert!(classify("error: something we have never seen before").is_none());
    }

    #[test]
    fn classify_specific_matchers_win_over_unable_to_access() {
        // `unable to access` is a wrapper that frames many other
        // failures; it must NOT shadow the inner connect-failure or
        // auth-failed classification.
        let stderr =
            "fatal: unable to access 'https://x/y.git/': Authentication failed for 'https://x/'";
        assert!(matches!(
            classify(stderr).unwrap(),
            GitError::AuthFailed { .. }
        ));
    }
}
