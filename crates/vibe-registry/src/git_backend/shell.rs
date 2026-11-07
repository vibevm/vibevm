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
//! One operation does not always reach `git` at all:
//! [`GitBackend::fetch_file_at_ref`] first offers the read to the
//! `raw_http` module, which serves a single file over plain HTTPS on the
//! two hosts that expose one. See that module for what it may and may
//! not conclude on its own.
//!
//! [prop]: ../../../../../vibevm/vibespecs/modules/vibe-registry/PROP-001-git-backend.xml

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#backend");

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use super::{GitBackend, GitError, GitPreparedRevision};

/// `git` subprocess backend.
///
/// Safe to share across threads: every operation constructs a fresh
/// [`Command`] and blocks on its output. The only shared state is the
/// immutable path to the `git` binary.
#[derive(Debug)]
pub struct ShellGit {
    binary: PathBuf,
    admission: Option<Arc<crate::HttpAdmission>>,
    native: Option<Arc<native_admission::NativeState>>,
    // Force the interactive-credential silencing layer on for every
    // invocation of this instance, regardless of TTY (PROP-002 §2.2.1).
    // Set on the anonymous variant a `MultiRegistryResolver` hands to a
    // public (`auth = "none"`) registry, so a 401/403 from a public host
    // can never turn into a blocking GCM / askPass prompt — it classifies
    // as "no answer here" and the registry walk continues. `false` on the
    // default backend, whose silencing stays TTY-derived.
    force_anonymous: bool,
    // Base the HTTPS read fast path (`raw_http`) reads from, or `None`
    // for each host's own production base — which is every case but a
    // test pointing the read at a local mock server.
    raw_base: Option<String>,
    // The one HTTP client the fast path reads through, built on first
    // use. Built once rather than per read because the pool of live
    // connections lives inside it: a resolve walk over a 26-dependency
    // graph makes ~80 reads of the same two hosts, and a client per read
    // paid a fresh TLS handshake for every one of them. `None` inside
    // the lock means the client could not be built at all — then there
    // is no fast path and every read goes to git, which is the same
    // answer as "not a host we know".
    raw_client: OnceLock<Option<reqwest::blocking::Client>>,
    // Where the fast path's pauses between retries go. Real time in
    // production; a test hands in a recorder, so a suite that asserts
    // the backoff schedule never spends it.
    raw_sleeper: Sleeper,
    // Cached preflight result — populated on first `preflight()` call
    // for this instance. Kept per-instance (not global) so tests with
    // bogus binaries do not poison the cache for instances pointing at
    // a real `git`.
    preflight_cache: OnceLock<bool>,
    // The invocation's journal for clones and fetches no observation
    // surrounds (`PROP-068#git`). Shared with the anonymous variant, which
    // is made before the composition root attaches it.
    transfers: Arc<OnceLock<vibe_core::progress::transfer::Transfers>>,
}

impl Default for ShellGit {
    fn default() -> Self {
        ShellGit::new()
    }
}

mod sleeper;
pub(super) use sleeper::Sleeper;

impl ShellGit {
    // Keep the existing environment read at its recorded composition root.
    fn configured_binary() -> PathBuf {
        std::env::var_os("VIBE_GIT_BINARY")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("git"))
    }

    /// The shared HTTP client of the read fast path, built on first use.
    ///
    /// `None` when it could not be built — a broken TLS backend, not a
    /// transient condition, so the failed attempt is cached like any
    /// other and the reads all go to git instead of re-paying it.
    fn raw_client(&self) -> Option<&reqwest::blocking::Client> {
        self.raw_client
            .get_or_init(|| raw_http::build_client(self.admission.as_deref()))
            .as_ref()
    }

    fn admit_native(&self) -> Result<(), GitError> {
        if self.native.is_some() {
            return Err(native_admission::unavailable());
        }
        if self.admission.is_some() {
            return Err(crate::ContactRefusal::NativeTransportUnavailable.into());
        }
        Ok(())
    }

    /// Verify that the configured git binary responds to `--version`.
    ///
    /// Caches the result on this instance. Subsequent calls are free.
    pub fn preflight(&self) -> Result<(), GitError> {
        if let Some(native) = &self.native {
            return native.preflight();
        }
        self.admit_native()?;
        let ok = *self.preflight_cache.get_or_init(|| {
            let mut cmd = Command::new(&self.binary);
            apply_common_env(&mut cmd, self.force_anonymous);
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
        self.admit_native()?;
        let mut cmd = Command::new(&self.binary);
        // apply_common_env may push `-c <key>=<value>` flags that must
        // come BEFORE the subcommand — push the env / global-flag
        // layer first, then the actual git subcommand args.
        apply_common_env(&mut cmd, self.force_anonymous);
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
        self.admit_native()?;
        let mut cmd = Command::new(&self.binary);
        apply_common_env(&mut cmd, self.force_anonymous);
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
    fn preflight_registry_auth(&self, auth: vibe_core::manifest::AuthKind) -> Result<(), GitError> {
        if self.native.is_some()
            && !matches!(
                auth,
                vibe_core::manifest::AuthKind::None | vibe_core::manifest::AuthKind::TokenEnv
            )
        {
            return Err(native_admission::unsupported(
                "declared credential route requires a separate admitted profile",
            ));
        }
        Ok(())
    }

    fn preflight_materialisation(&self) -> Result<(), GitError> {
        self.admit_native()
    }

    fn stage_native_materialisation(
        &self,
        objects: &Path,
        revision: &GitPreparedRevision,
    ) -> Result<Option<native_admission::NativeMaterialisation>, GitError> {
        self.native
            .as_ref()
            .map(|native| native.stage(objects, revision))
            .transpose()
    }
    fn prepare_native_source(
        &self,
        url: &str,
        reference: &str,
        reuse: Option<&Path>,
        objects: &Path,
    ) -> Result<Option<native_admission::NativeSource>, GitError> {
        self.native
            .as_ref()
            .map(|native| {
                native.acquire(native_admission::Preparation {
                    reporter: self,
                    address: url,
                    reference,
                    reuse,
                    objects,
                    destination: objects,
                    anonymous: self.force_anonymous,
                })
            })
            .transpose()
    }
    fn uses_native_materialisation(&self) -> bool {
        self.native.is_some()
    }

    fn origin_url(&self, dest: &Path) -> Result<String, GitError> {
        if let Some(native) = &self.native {
            return native.origin(dest, true);
        }
        prepared::origin(self, dest)
    }

    fn recorded_origin_url(&self, dest: &Path) -> Result<String, GitError> {
        if let Some(native) = &self.native {
            return native.origin(dest, false);
        }
        prepared::recorded_origin(self, dest)
    }

    fn prepare_ref(
        &self,
        url: &str,
        reference: &str,
        reuse: Option<&Path>,
        objects: &Path,
        destination: &Path,
    ) -> Result<GitPreparedRevision, GitError> {
        self.preflight()?;
        prepared::prepare(self, url, reference, reuse, objects, destination)
    }

    fn file_at_commit(
        &self,
        objects: &Path,
        commit: &str,
        path: &str,
    ) -> Result<Vec<u8>, GitError> {
        if let Some(native) = &self.native {
            return native.file(objects, commit, path);
        }
        self.run(&["show", &format!("{commit}:{path}")], Some(objects))
            .map(|o| o.stdout)
    }

    fn apply_prepared_ref(
        &self,
        objects: &Path,
        revision: &GitPreparedRevision,
        origin: &str,
        authenticated_origin: Option<&str>,
        dest: &Path,
        refresh: bool,
    ) -> Result<(), GitError> {
        self.preflight_materialisation()?;
        prepared::apply(
            self,
            objects,
            revision,
            origin,
            authenticated_origin,
            dest,
            refresh,
        )
    }
    fn bootstrap(&self, url: &str, refname: &str, dest: &Path) -> Result<(), GitError> {
        self.preflight()?;
        checkout::bootstrap(self, url, refname, dest)
    }

    fn bootstrap_embedded(&self, url: &str, refname: &str, dest: &Path) -> Result<(), GitError> {
        self.preflight()?;
        checkout::bootstrap_embedded(self, url, refname, dest)
    }

    fn update(&self, dest: &Path, refname: &str) -> Result<(), GitError> {
        self.preflight()?;
        checkout::update(self, dest, refname)
    }

    fn head_commit(&self, dest: &Path) -> Result<Option<String>, GitError> {
        if let Some(native) = &self.native {
            return native.head(dest);
        }
        self.preflight()?;
        // `git -C <dest> rev-parse HEAD` — the resolved commit of the
        // currently checked-out ref. The clone is already at the target ref
        // when this is called (post-bootstrap / post-update), so HEAD is the
        // version tag's commit. Trimmed of the trailing newline.
        let output = self.run(&["rev-parse", "HEAD"], Some(dest))?;
        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if sha.is_empty() {
            Ok(None)
        } else {
            Ok(Some(sha))
        }
    }

    fn head_tree(&self, dest: &Path) -> Result<Option<String>, GitError> {
        self.preflight()?;
        let output = self.run(&["rev-parse", "HEAD^{tree}"], Some(dest))?;
        let oid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if oid.is_empty() {
            Ok(None)
        } else {
            Ok(Some(oid))
        }
    }

    fn working_tree_dirty(&self, dest: &Path) -> Result<bool, GitError> {
        self.preflight()?;
        let output = self.run(
            &[
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=normal",
                "--ignored=matching",
            ],
            Some(dest),
        )?;
        Ok(!output.stdout.is_empty())
    }

    fn clean_worktree(&self, dest: &Path) -> Result<(), GitError> {
        self.preflight()?;
        self.run(&["clean", "-dfx"], Some(dest)).map(|_| ())
    }

    fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
        self.preflight()?;
        // `git ls-remote --tags <url>` outputs one line per ref:
        //   <hash>\trefs/tags/<name>
        // Annotated tags appear twice — once as the tag object and once
        // as the peeled commit (`refs/tags/<name>^{}`). We drop the
        // peeled form's `^{}` suffix and dedupe on the resulting names.
        let output = if let Some(native) = &self.native {
            native.tags(url, self.force_anonymous)?
        } else {
            self.run(&["ls-remote", "--tags", "--", url], None)?
        };
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
        if let Some(native) = &self.native {
            native.preflight()?;
            let temporary = tempfile::tempdir().map_err(|source| GitError::Io {
                cmd: "preparing native file read".into(),
                source,
            })?;
            let objects = temporary.path().join("objects.git");
            let revision = native.prepare(native_admission::Preparation {
                reporter: self,
                address: url,
                reference: refname,
                reuse: None,
                objects: &objects,
                destination: &temporary.path().join("unused-slot"),
                anonymous: self.force_anonymous,
            })?;
            return native.file(&objects, &revision.commit, &path.replace('\\', "/"));
        }
        // Normalise platform separators to forward slash — `git archive`
        // expects in-repo paths in posix form, and so do both hosts the
        // HTTPS read addresses.
        let normalized = path.replace('\\', "/");

        // The HTTPS read, first and without a subprocess: on GitHub and
        // GitVerse a single file is one plain GET away, and the archive
        // below would only be refused there anyway (PROP-002 §2.12 — see
        // `raw_http`). It answers for the file or for its absence, or it
        // declines, and declining costs nothing: the archive → clone
        // ladder below is entered exactly as it was before. Deliberately
        // ahead of `preflight` — a read this path serves needs no `git`
        // on the machine at all.
        if let Some(client) = self.raw_client()
            && let Some(outcome) = raw_http::try_read(
                client,
                self.admission.as_deref(),
                self.raw_base.as_deref(),
                &self.raw_sleeper,
                url,
                refname,
                &normalized,
            )
        {
            return outcome;
        }
        self.preflight()?;

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
                    url: without_userinfo(url),
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
                url: without_userinfo(url),
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
                url: without_userinfo(url),
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
                url: without_userinfo(url),
            });
        }
        if combined_lc.contains("git archive")
            && combined_lc.contains("expected ack/nak")
            && combined_lc.contains("flush packet")
        {
            return Err(GitError::ArchiveUnsupported {
                url: without_userinfo(url),
            });
        }
        if combined_lc.contains("unknown revision")
            || combined_lc.contains("not a tree object")
            || combined_lc.contains("couldn't find remote ref")
        {
            return Err(GitError::RefNotFound {
                url: without_userinfo(url),
                refname: refname.to_string(),
            });
        }
        Err(classify_failure(&args, &output))
    }

    fn set_remote_url(&self, dest: &Path, remote: &str, url: &str) -> Result<(), GitError> {
        if let Some(native) = &self.native {
            return native.set_origin(dest, remote, url);
        }
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

    fn anonymized_for_public(&self) -> Option<Arc<dyn GitBackend>> {
        // A public-registry backend: same git binary, but with the
        // interactive-credential silencing layer forced on for every
        // invocation (PROP-002 §2.2.1). Fresh `preflight_cache` — the
        // probe is per-instance and cheap to redo. Cannot derive `Clone`
        // (`OnceLock` is not `Clone`), so reconstruct explicitly.
        Some(Arc::new(ShellGit {
            binary: self.binary.clone(),
            admission: self.admission.clone(),
            native: self.native.clone(),
            force_anonymous: true,
            raw_base: self.raw_base.clone(),
            // The anonymous backend retains the same admission Arc, so
            // it can share the pool built under exactly that binding.
            // Credentials remain request-scoped.
            raw_client: OnceLock::from(self.raw_client().cloned()),
            raw_sleeper: self.raw_sleeper.clone(),
            preflight_cache: OnceLock::new(),
            transfers: Arc::clone(&self.transfers),
        }))
    }

    fn attach_transfers(&self, transfers: &vibe_core::progress::transfer::Transfers) {
        let _ = self.transfers.set(transfers.clone());
    }
}

mod tar;
use tar::extract_single_file_from_tar;

/// Clone/fetch and ref-kind-aware checkout mechanics.
mod checkout;
mod construction;
/// Explicit trusted-installation profile and bounded native preparation.
pub mod native_admission;
mod prepared;

/// The HTTPS single-file read that runs before `git archive` on the two
/// hosts that serve one, and the rules that keep its misses honest.
pub mod raw_http;

/// The read-only checkout queries (`last_commit_iso`, `branch`) — an
/// `impl ShellGit` block of its own, split out per the file-length budget.
mod query;

/// The environment and global flags every spawned git invocation gets.
mod env;
use env::apply_common_env;
#[cfg(test)]
use env::should_silence_credential_helpers;

/// Clones and fetches run as reported transfers (`PROP-068#git`).
mod reported;

mod diagnostics;
use diagnostics::*;

#[cfg(test)]
#[path = "shell/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "shell/tests_pure.rs"]
mod tests_pure;
