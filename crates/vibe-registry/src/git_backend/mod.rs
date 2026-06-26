//! Git backend abstraction.
//!
//! Every operation the registry performs against git goes through
//! [`GitBackend`]. `M1` ships exactly one implementation — [`ShellGit`],
//! which spawns the system `git` binary. A future `libgit2`-based
//! implementation would add a second type alongside it without touching
//! [`GitBackend`] consumers.
//!
//! Spec: [`spec/modules/vibe-registry/PROP-001-git-backend.md`][prop].
//!
//! [prop]: ../../../../../spec/modules/vibe-registry/PROP-001-git-backend.md

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-001#backend-trait");

use std::path::Path;

use specmark::spec;
use thiserror::Error;

pub mod shell;

pub use shell::ShellGit;

/// Errors a [`GitBackend`] operation may surface.
///
/// Variants correspond to stderr patterns stable enough to key on.
/// Anything unclassified surfaces as [`GitError::CommandFailed`] with the
/// raw stderr attached.
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
pub enum GitError {
    #[error(
        "the `git` executable is not available on PATH; install git \
         (https://git-scm.com/downloads) and retry \
         (violates spec://vibevm/modules/vibe-registry/PROP-001#backend; \
          fix: install git and ensure it is on PATH)"
    )]
    NotInstalled,

    #[error(
        "remote repository `{url}` not found (does it exist? is access granted?) \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: verify the repo URL and your read access)"
    )]
    RepoNotFound { url: String },

    #[error(
        "ssh authentication failed for `{url}` — check your ssh-agent / keys \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#registry-auth; \
          fix: load your key into ssh-agent or fix the [[registry]] auth setting)"
    )]
    AuthFailed { url: String },

    #[error(
        "unable to reach `{url}` (network or DNS error) \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: check connectivity and the host name)"
    )]
    NetworkUnreachable { url: String },

    #[error(
        "branch / ref `{refname}` not found on `{url}` \
         (violates spec://vibevm/modules/vibe-registry/PROP-001#backend-trait; \
          fix: verify the ref with `git ls-remote`)"
    )]
    RefNotFound { url: String, refname: String },

    #[error(
        "file `{path}` not found in `{url}` at ref `{refname}` \
         (violates spec://vibevm/modules/vibe-registry/PROP-001#backend-trait; \
          fix: ensure the file is committed at that ref)"
    )]
    FileNotFoundInRef {
        url: String,
        refname: String,
        path: String,
    },

    #[error(
        "remote `{url}` does not support `git archive` for fetching individual files \
         (uploadarch service refused). Caller should fall back to a clone \
         (violates spec://vibevm/modules/vibe-registry/PROP-001#backend-trait; \
          fix: use the clone fallback or enable upload-archive on the host)"
    )]
    ArchiveUnsupported { url: String },

    #[error(
        "git `{cmd}` exited with status {status} \
         (violates spec://vibevm/modules/vibe-registry/PROP-001#windows-ux; \
          fix: re-run the command by hand and read the stderr below):\n{stderr}"
    )]
    CommandFailed {
        cmd: String,
        status: i32,
        stderr: String,
    },

    #[error(
        "I/O error spawning git `{cmd}` \
         (violates spec://vibevm/modules/vibe-registry/PROP-001#backend; \
          fix: check the git installation and PATH): {source}"
    )]
    Io {
        cmd: String,
        #[source]
        source: std::io::Error,
    },
}

/// Narrow abstraction over the git operations the registry needs.
///
/// The trait deliberately stays small — every new method is a deliberate
/// widening, not an accident. Today it carries:
///
/// - `bootstrap` / `update` — full clone and refresh of a working tree.
/// - `list_tags` / `fetch_file_at_ref` — *shallow* primitives the depsolver
///   uses to enumerate versions and read manifests *without* a clone (see
///   PROP-002 §2.12 — performance strategy). A resolver pass that touches
///   N candidate versions of a package must not clone all N; it walks
///   `list_tags` then reads `vibe.toml` per candidate via
///   `fetch_file_at_ref`, and only `bootstrap`s the version it commits to.
///
/// **Method names.** `bootstrap` (not `clone`) avoids collision with
/// `std::clone::Clone::clone` when the backend is held behind
/// `Arc<dyn GitBackend>`, where `Arc::clone` would otherwise be
/// ambiguous at the call site.
///
/// # Example
///
/// Consumers hold the seam as a trait object; any implementation
/// slots in. Production code constructs [`ShellGit`]; a test fake
/// needs only the four required methods (`set_remote_url` has a
/// default impl).
///
/// ```
/// use std::path::Path;
/// use std::sync::Arc;
/// use vibe_registry::{GitBackend, GitError};
///
/// struct StaticTags;
///
/// impl GitBackend for StaticTags {
///     fn bootstrap(&self, _url: &str, _refname: &str, _dest: &Path) -> Result<(), GitError> {
///         Ok(())
///     }
///     fn update(&self, _dest: &Path, _refname: &str) -> Result<(), GitError> {
///         Ok(())
///     }
///     fn list_tags(&self, _url: &str) -> Result<Vec<String>, GitError> {
///         Ok(vec!["v0.1.0".into(), "v0.2.0".into()])
///     }
///     fn fetch_file_at_ref(&self, url: &str, refname: &str, path: &str) -> Result<Vec<u8>, GitError> {
///         Err(GitError::FileNotFoundInRef {
///             url: url.into(),
///             refname: refname.into(),
///             path: path.into(),
///         })
///     }
/// }
///
/// let backend: Arc<dyn GitBackend> = Arc::new(StaticTags);
/// let tags = backend.list_tags("git@example.com:org/repo.git")?;
/// assert_eq!(tags, ["v0.1.0", "v0.2.0"]);
/// # Ok::<(), GitError>(())
/// ```
pub trait GitBackend: Send + Sync {
    /// Clone `url` (checked out at `refname`) into `dest`.
    ///
    /// The caller guarantees `dest` is either empty or absent. On error,
    /// the backend makes no guarantee about the partial state of `dest`
    /// — the caller cleans up.
    fn bootstrap(&self, url: &str, refname: &str, dest: &Path) -> Result<(), GitError>;

    /// Fast-forward `dest` to `origin/<refname>`. Assumes `dest` is a git
    /// repository previously populated by `bootstrap`.
    fn update(&self, dest: &Path, refname: &str) -> Result<(), GitError>;

    /// The commit SHA the working tree at `dest` is checked out at —
    /// `git -C <dest> rev-parse HEAD`. Recorded as the lockfile's
    /// `resolved_commit` so a re-clone reconstructs byte-identical content,
    /// including the exact gitlink commit of every submodule (PROP-021 §2.4),
    /// and so an `in-place` slot's identity is its commit (PROP-022 §2.5).
    ///
    /// Returns `Ok(None)` from the default impl — a test backend that tracks
    /// no real checkout has no commit to report, and a `None` keeps the
    /// lockfile field absent exactly as before this method existed. The
    /// production [`ShellGit`] overrides it to return the real SHA.
    fn head_commit(&self, _dest: &Path) -> Result<Option<String>, GitError> {
        Ok(None)
    }

    /// List the tag names available on `url` without cloning. Implemented
    /// via `git ls-remote --tags`. Tags annotated with the
    /// `^{}` peeled-form suffix are stripped so the caller sees clean
    /// tag names; duplicates (peeled + annotated) are deduplicated.
    ///
    /// Returns tag names verbatim — semver coercion (e.g. stripping the
    /// `v` prefix) is the caller's job.
    fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError>;

    /// Fetch the contents of a single file at the given ref from `url`,
    /// without populating a working tree. Implemented via `git archive
    /// --remote=<url> --format=tar <refname> <path>` piped through
    /// in-process tar extraction.
    ///
    /// `path` is the path inside the repo; both forward-slash and
    /// platform-native separators are accepted and normalised to forward
    /// slash (the form `git archive` expects).
    ///
    /// Returns the file's bytes. Errors:
    /// - [`GitError::RefNotFound`] if `refname` does not exist on `url`.
    /// - [`GitError::FileNotFoundInRef`] if `path` is missing in that ref.
    ///
    /// Note that `git archive` over `git://`-style protocols requires
    /// server support (`uploadarch.allowAnySHA1InWant` etc). Hosted git
    /// providers (GitHub, GitLab, Gitea, GitVerse) typically support
    /// this; a private bare server may not. The `GitBackend` returns
    /// [`GitError::ArchiveUnsupported`] in that case so the caller can
    /// fall back to a shallow clone.
    fn fetch_file_at_ref(&self, url: &str, refname: &str, path: &str) -> Result<Vec<u8>, GitError>;

    /// Rewrite the `<remote>` URL inside an existing clone at `dest`
    /// to `url`. Implemented via `git -C <dest> remote set-url
    /// <remote> <url>`. Used by the per-package registry under
    /// `auth = "token-env"` (PROP-002 §2.2.1) to scrub the
    /// credentialised URL out of the freshly-cloned `.git/config`
    /// after `bootstrap` — the token only ever lives in memory and
    /// in the spawned `git clone` invocation; once the clone is on
    /// disk the recorded `remote.<remote>.url` is the plain
    /// (credential-free) URL. Subsequent `update` calls re-issue
    /// auth via the `bootstrap` retry path when the plain URL
    /// returns 401, rather than persisting credentials in the
    /// clone state.
    ///
    /// Returns `GitError::CommandFailed` when the remote is
    /// unknown or git rejects the URL; the caller is responsible
    /// for ensuring the remote exists (it always does after a
    /// successful `bootstrap`, where `origin` is the default).
    ///
    /// Default impl is provided so non-shell test backends don't
    /// have to stub it explicitly when they don't exercise the
    /// auth path.
    fn set_remote_url(&self, _dest: &Path, _remote: &str, _url: &str) -> Result<(), GitError> {
        Ok(())
    }
}
