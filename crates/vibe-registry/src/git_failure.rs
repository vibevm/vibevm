//! What a failed registry git operation says to the operator.
//!
//! A [`GitError`] knows the shape of the failure and nothing about the
//! errand: git reports `git 'git fetch … -- origin …' exited with status
//! 128`, which reads as if vibe built the command wrong and never names
//! the registry, the real url, the package, or the reason the corporate
//! network gave. The context lives one layer up — the registry knows it
//! is listing the versions of `org.example/thing` for registry `corp` at
//! `https://…` — so this module is where the two meet.
//!
//! The order is fixed (PROP-002 `#failure-discriminator`): the errand in
//! words, the registry and its real url, the package, the translated
//! cause with its fix, the registries this profile allows, the
//! public-host remark when one applies, and git's own stderr tail LAST,
//! below the explanation rather than in place of it.
//!
//! One funnel: [`explain`] is the single construction of
//! [`RegistryError::GitOperation`], so a new call site cannot quietly
//! lose the context — it either names its [`GitOperation`] or it does
//! not reach this module at all.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
);

use std::fmt;
use std::sync::Arc;

use crate::RegistryError;
use crate::git_backend::GitError;
use crate::git_backend::shell::without_userinfo;

mod operation;
pub use operation::GitOperation;
mod render;
mod roster;
pub use roster::{RegistryRoster, RegistryRosterRow};

/// Everything the operator needs to read one failed registry git
/// operation, in the order [`fmt::Display`] prints it.
///
/// Boxed inside [`RegistryError::GitOperation`] — the same shape
/// `AllVersionsUnusable` and `StoreEntryMismatch` already pay, so one
/// rare, wide variant does not widen every `Result<_, RegistryError>` on
/// the stack.
#[derive(Debug)]
pub struct GitFailureDetail {
    /// The errand, in words — never an argv.
    pub operation: GitOperation,
    /// The `[[registry]] name` the attempt belongs to.
    pub registry: String,
    /// The endpoint actually contacted, credential-scrubbed. Never
    /// `origin`: that spelling is git's record of a remote, not an
    /// address an operator can act on.
    pub url: String,
    /// The registries the effective profile allows, so a missing
    /// `exclude_hosting` is visible in the failure itself.
    pub roster: Arc<RegistryRoster>,
    /// The classified git failure this explanation is about.
    pub source: GitError,
}

impl GitFailureDetail {
    /// The explanation, up to but excluding the spec/fix trailer the
    /// error variant appends and the stderr tail below it.
    pub fn explanation(&self) -> String {
        render::explanation(self)
    }

    /// Git's own stderr tail — bounded, credential-scrubbed, and
    /// introduced by its own line so it reads as evidence under the
    /// explanation. Empty when the classified failure carried none.
    pub fn git_said(&self) -> String {
        render::git_said(self)
    }
}

impl fmt::Display for GitFailureDetail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.explanation())
    }
}

/// The one construction of [`RegistryError::GitOperation`].
///
/// Only the failures whose current output is unreadable are re-framed:
/// an unclassified `CommandFailed`, a redirect, a TLS trust refusal.
/// Every other variant is a typed answer the resolver walk routes on —
/// `AuthFailed` is the public-401 walk-past signal, `RepoNotFound`
/// becomes `UnknownPackage`, `ArchiveUnsupported` selects the clone
/// fallback — so re-framing those would change resolution, not only its
/// prose. They keep their own identity and their own message; the rest
/// of that half is `BACKLOG.md` B-266.
pub(crate) fn explain(
    operation: GitOperation,
    registry: &str,
    url: &str,
    roster: &Arc<RegistryRoster>,
    error: GitError,
) -> RegistryError {
    if !wants_context(&error) {
        return RegistryError::from(error);
    }
    RegistryError::GitOperation {
        detail: Box::new(GitFailureDetail {
            operation,
            registry: registry.to_string(),
            url: without_userinfo(url),
            roster: Arc::clone(roster),
            source: error,
        }),
    }
}

/// [`explain`] for a layer that already converted its `GitError` — the
/// monorepo clone reports [`RegistryError`] from the start. A non-git
/// error, and a git error [`explain`] would leave alone, passes through
/// untouched.
pub(crate) fn upgrade(
    operation: GitOperation,
    registry: &str,
    url: &str,
    roster: &Arc<RegistryRoster>,
    error: RegistryError,
) -> RegistryError {
    match error {
        RegistryError::Git(inner) => explain(operation, registry, url, roster, inner),
        other => other,
    }
}

/// Whether this failure reads better with the errand around it.
///
/// `true` exactly for the variants whose message today says what git
/// printed and nothing about what vibe was doing.
fn wants_context(error: &GitError) -> bool {
    matches!(
        error,
        GitError::CommandFailed { .. }
            | GitError::HttpRedirected { .. }
            | GitError::TlsTrust { .. }
    )
}

#[cfg(test)]
#[path = "git_failure/tests.rs"]
mod tests;
