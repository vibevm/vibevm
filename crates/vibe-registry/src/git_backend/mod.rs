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
//! [prop]: ../../../../../vibevm/vibespecs/modules/vibe-registry/PROP-001-git-backend.xml

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#backend-trait");

use specmark::spec;
use std::path::Path;

pub mod deferred;
/// The prose each [`GitError`] variant interpolates — the heads and the
/// standing fixes, kept together so one failure reads the same wherever
/// it surfaces.
mod messages;
pub mod shell;
pub mod transfers;
pub mod verbatim;
pub use verbatim::PayloadBytes;

pub(crate) use messages::{REDIRECTED_FIX, TLS_TRUST_FIX};
use messages::{auth_failed_fix, auth_failed_head, redirected_head, tls_trust_head};

pub use shell::ShellGit;

/// The fetched object, its peeled commit and the actual ref namespace.
/// These identifiers describe one prepared source; applying does not resolve
/// the remote reference again.
///
/// ```
/// use vibe_registry::git_backend::GitPreparedRevision;
/// fn identities(revision: &GitPreparedRevision) -> (&str, &str) {
///     (&revision.object, &revision.commit)
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub struct GitPreparedRevision {
    pub object: String,
    pub commit: String,
    pub reference: String,
    /// Actual future slot, used to reject mutable checkout-local attribute inputs.
    pub destination: std::path::PathBuf,
}

mod error;
pub use error::GitError;

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
    /// Actual endpoint observed by a retained native acquisition. None means
    /// the backend has no such observation; never infer it from a report label.
    fn prepared_serving_endpoint(&self, _objects: &Path) -> Result<Option<String>, GitError> {
        Ok(None)
    }
    fn uses_native_materialisation(&self) -> bool {
        false
    }
    /// Resolve deferred guarantee selection without initializing the executable.
    fn native_materialisation_for(&self, _address: &str) -> Result<bool, GitError> {
        Ok(self.uses_native_materialisation())
    }
    /// Native-only source acquisition before the actual manifest chooses its
    /// final cache/workspace destination. Non-native backends do no work here.
    fn prepare_native_source(
        &self,
        _url: &str,
        _reference: &str,
        _reuse: Option<&Path>,
        _objects: &Path,
    ) -> Result<Option<shell::native_admission::NativeSource>, GitError> {
        Ok(None)
    }
    /// [`Self::prepare_native_source`] into a **durable** object store of
    /// the backend's own, which a later preparation refreshes instead of
    /// cloning the repository again (PROP-073 `#NATIVE-PREPARATION`). Only
    /// an owner that owns that directory and its lock — the controlled
    /// registry cache — may ask: `objects` outlives the operation, so a
    /// caller handing a path it does not own would be handing its bytes to
    /// the next invocation.
    ///
    /// The default forwards to the ordinary preparation, so a backend with
    /// no durable store behaves exactly as before.
    fn prepare_cached_native_source(
        &self,
        url: &str,
        reference: &str,
        namespace: Option<&str>,
        objects: &Path,
    ) -> Result<Option<shell::native_admission::NativeSource>, GitError> {
        let _ = namespace;
        self.prepare_native_source(url, reference, None, objects)
    }
    /// What the selected endpoint advertises for `reference`: the object id
    /// at its tip and the ref namespace it was found in, read with one
    /// `ls-remote` and no objects at all. `None` from a backend with no
    /// controlled transport, whose caller then has no cheap freshness
    /// question to ask.
    fn native_advertised_tip(
        &self,
        _url: &str,
        _reference: &str,
    ) -> Result<Option<(String, String)>, GitError> {
        Ok(None)
    }
    /// An opaque digest of the attribute and conversion inputs a staging
    /// admission probe reads, for an owner that remembers a verdict across
    /// processes (PROP-073 `#NATIVE-MATERIALISATION`). `None` where there is
    /// no such probe.
    fn native_admission_inputs(&self) -> Result<Option<[u8; 32]>, GitError> {
        Ok(None)
    }
    /// Acquire a held endpoint while retaining warm checkout metadata separately.
    /// Unsupported backends must not silently reinterpret the warm origin.
    fn prepare_pinned_native_source(
        &self,
        _url: &str,
        _reference: &str,
        _reuse: Option<&Path>,
        _objects: &Path,
    ) -> Result<Option<shell::native_admission::NativeSource>, GitError> {
        if self.uses_native_materialisation() {
            return Err(GitError::NativeUnavailable {
                operation: "pinned source acquisition with warm metadata",
            });
        }
        Ok(None)
    }
    /// Stage a retained source under an explicitly supported native profile.
    /// None means this backend remains on its incumbent materialisation path.
    ///
    /// `bytes` is whose rendering the stage holds ([`PayloadBytes`]).
    fn stage_native_materialisation(
        &self,
        _objects: &Path,
        _revision: &GitPreparedRevision,
        _bytes: PayloadBytes,
    ) -> Result<Option<shell::native_admission::NativeMaterialisation>, GitError> {
        Ok(None)
    }
    /// Bring a data working copy to the exact bytes its blobs record, where a
    /// clone is *used* without being cloned or updated first: a registry
    /// inside its freshness window reaches no other git command ([`verbatim`]).
    fn normalise_owned_worktree(&self, _dest: &Path) -> Result<(), GitError> {
        Ok(())
    }
    /// Check the selected authored endpoint before credential lookup. No contact or
    /// ambient configuration evaluation is permitted here. Final backend admission
    /// still validates effective endpoints and credential use independently.
    fn preflight_selected_address(&self, _address: &str) -> Result<(), GitError> {
        Ok(())
    }

    /// Check declared credential semantics before attempting registry contact.
    fn preflight_registry_auth(
        &self,
        _auth: vibe_core::manifest::AuthKind,
    ) -> Result<(), GitError> {
        Ok(())
    }

    /// Return an immutable capability restricted to this registry's declared
    /// credential mechanism. Unbound backends retain their incumbent behavior.
    fn bound_for_registry_auth(
        &self,
        _auth: vibe_core::manifest::AuthKind,
    ) -> Result<Option<std::sync::Arc<dyn GitBackend>>, GitError> {
        Ok(None)
    }

    /// Refuse unsupported publication before its caller creates or removes paths.
    fn preflight_materialisation(&self) -> Result<(), GitError> {
        Ok(())
    }
    /// Read the actual origin used by an existing checkout refresh.
    fn origin_url(&self, _dest: &Path) -> Result<String, GitError> {
        Err(GitError::PreparedCheckoutUnsupported)
    }

    /// Read Git's stored origin spelling before URL rewrites. Fresh local clones
    /// record an absolute origin even when their caller used a relative source.
    fn recorded_origin_url(&self, _dest: &Path) -> Result<String, GitError> {
        Err(GitError::PreparedCheckoutUnsupported)
    }

    /// Capture exact objects in an operation-owned bare view. Existing-origin
    /// acquisition may write only objects/FETCH_HEAD in `reuse`, retaining its
    /// native transport context without changing HEAD, index, config, named refs
    /// or worktree. The borrowed original object store must remain available
    /// until the bound candidate has been applied or dropped. After the requested
    /// object is acquired, wrap inspection failures in [`GitError::PreparedValidation`]
    /// so a caller cannot replace that invalid source with a mirror.
    fn prepare_ref(
        &self,
        _url: &str,
        _reference: &str,
        _reuse: Option<&Path>,
        _objects: &Path,
        _destination: &Path,
    ) -> Result<GitPreparedRevision, GitError> {
        Err(GitError::PreparedCheckoutUnsupported)
    }

    /// Read a file from the exact prepared commit, without a worktree checkout.
    fn file_at_commit(
        &self,
        _objects: &Path,
        _commit: &str,
        _path: &str,
    ) -> Result<Vec<u8>, GitError> {
        Err(GitError::PreparedCheckoutUnsupported)
    }

    /// Apply only the captured object/commit from the held local object repository.
    fn apply_prepared_ref(
        &self,
        _objects: &Path,
        _revision: &GitPreparedRevision,
        _origin: &str,
        _authenticated_origin: Option<&str>,
        _dest: &Path,
        _refresh: bool,
    ) -> Result<(), GitError> {
        Err(GitError::PreparedCheckoutUnsupported)
    }
    /// Clone `url` (checked out at `refname`) into `dest`.
    ///
    /// The caller guarantees `dest` is either empty or absent. On error,
    /// the backend makes no guarantee about the partial state of `dest`
    /// — the caller cleans up.
    fn bootstrap(&self, url: &str, refname: &str, dest: &Path) -> Result<(), GitError>;

    /// [`Self::bootstrap`] into a working copy vibe owns and reads as data,
    /// checked out byte-exact ([`verbatim`]). The same chain also places an
    /// `in-place` package into the user's own slot (PROP-022 §2.4), so which
    /// a destination is, the caller says; it is never inferred.
    fn bootstrap_data(&self, url: &str, refname: &str, dest: &Path) -> Result<(), GitError> {
        self.bootstrap(url, refname, dest)
    }

    /// Clone and check out one exact ref without initialising submodules.
    ///
    /// Dependency-declared embedded sources are opaque upstream trees, not
    /// package graphs. Their own gitlinks must therefore never trigger network
    /// recursion or executable package machinery. Test backends inherit the
    /// historical bootstrap behaviour unless they need to assert this
    /// distinction; the shell backend implements the non-recursive operation.
    fn bootstrap_embedded(&self, url: &str, refname: &str, dest: &Path) -> Result<(), GitError> {
        self.bootstrap(url, refname, dest)
    }

    /// Refresh `dest` to `refname`. Tags are peeled to detached commits,
    /// branches track their `origin` branch, and exact commits stay detached.
    /// Assumes `dest` is a git repository previously populated by `bootstrap`.
    fn update(&self, dest: &Path, refname: &str) -> Result<(), GitError>;

    /// [`Self::update`] of a data working copy ([`Self::bootstrap_data`]).
    fn update_data(&self, dest: &Path, refname: &str) -> Result<(), GitError> {
        self.update(dest, refname)
    }

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

    /// The root tree object for the checked-out commit (`HEAD^{tree}`).
    /// This is recorded beside the independent portable content hash for
    /// audit/debugging; content authentication keys on the latter.
    fn head_tree(&self, _dest: &Path) -> Result<Option<String>, GitError> {
        Ok(None)
    }

    /// Whether tracked, untracked, or ignored working-tree state would be
    /// repaired by an in-place reset. Test backends with no real checkout are
    /// clean by default; [`ShellGit`] asks git directly.
    fn working_tree_dirty(&self, _dest: &Path) -> Result<bool, GitError> {
        Ok(false)
    }

    /// Remove untracked and ignored working-tree state for direct in-place
    /// slots. Generic registry-cache refresh deliberately does not call this.
    fn clean_worktree(&self, _dest: &Path) -> Result<(), GitError> {
        Ok(())
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

    /// Return an anonymous-posture variant of this backend for use against
    /// a public (`auth = "none"`) registry, or `None` when this backend
    /// has no interactive-credential surface to silence.
    ///
    /// The anonymous variant unconditionally suppresses every channel git
    /// might use for *interactive* credential entry — GCM popups,
    /// `credential.helper`, `core.askPass` — regardless of TTY, so a
    /// 401/403 from a public host classifies as "no answer here" and the
    /// registry walk continues, instead of blocking on a login prompt
    /// (PROP-002 §2.2.1). A public registry never sends credentials, so
    /// there is nothing to prompt for.
    ///
    /// `MultiRegistryResolver::from_manifest` calls this for every
    /// `auth = "none"` registry and falls back to the shared backend when
    /// it returns `None`. The default returns `None`: a test fake drives
    /// no real git and never prompts, so it needs no anonymous variant.
    /// The production [`ShellGit`] overrides it.
    fn anonymized_for_public(&self) -> Option<std::sync::Arc<dyn GitBackend>> {
        None
    }

    /// Hand the backend the invocation's journal, limit and verb, so a
    /// clone or fetch no observation surrounds is still journaled
    /// (`PROP-068#git`). The first call wins, as for the index client. The
    /// default ignores it: a test fake moves no bytes to report.
    fn attach_transfers(&self, _transfers: &vibe_core::progress::transfer::Transfers) {}
}
