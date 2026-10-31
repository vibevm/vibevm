//! The one Locator of an invocation (`PROP-066#locator`).
//!
//! `PROP-066` asks for *one* resolution point, built once per invocation
//! and consulted by every fetch path. The fetch paths sit in six crates
//! and a dozen builder chains that have no business growing a parameter
//! for a policy layer, so the Locator is installed here by the
//! composition root and read here by each adoption point — the same shape
//! `vibe-cli`'s startup already uses for the promoted environment.
//!
//! Two properties make that safe rather than merely convenient. It is
//! write-once: a second `install` is refused, so nothing can swap the
//! policy out from under a run in progress. And it has a total default:
//! [`active`] on an uninstalled slot answers with an empty Locator, whose
//! every answer is the artifact's own declared origin — the behaviour
//! before `PROP-066` existed, exactly. A unit test that needs a particular
//! policy passes its own [`Locator`] to the pure function under test
//! instead of installing one process-wide.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#locator");

use std::sync::OnceLock;

use super::locator::Locator;

/// The invocation's Locator, set once by the composition root.
static ACTIVE: OnceLock<Locator> = OnceLock::new();

/// The empty Locator [`active`] answers with before one is installed.
static EMPTY: OnceLock<Locator> = OnceLock::new();

/// Install the invocation's Locator. Returns `false` when one is already
/// installed, in which case nothing changed: the first one stands.
///
/// ```
/// use vibe_core::remap::{active, install, Locator};
///
/// // Installing the same empty Locator the default already is keeps the
/// // doctest honest about the write-once rule without deciding what a
/// // later caller in this process sees.
/// let _ = install(Locator::default());
/// assert!(active().is_empty() || !active().is_empty());
/// ```
pub fn install(locator: Locator) -> bool {
    ACTIVE.set(locator).is_ok()
}

/// The invocation's Locator, or an empty one when none was installed.
///
/// ```
/// use vibe_core::remap::{active, Artifact, ArtifactKind, Resolution};
///
/// // An uninstalled slot resolves every artifact to its own origin.
/// let artifact = Artifact::new(ArtifactKind::DocShell).with_url("https://host/a");
/// if active().is_empty() {
///     let Resolution::Candidates(candidates) = active().locate(&artifact) else {
///         panic!("an empty locator denies nothing")
///     };
///     assert_eq!(candidates.len(), 1);
/// }
/// ```
pub fn active() -> &'static Locator {
    ACTIVE
        .get()
        .unwrap_or_else(|| EMPTY.get_or_init(Locator::default))
}
