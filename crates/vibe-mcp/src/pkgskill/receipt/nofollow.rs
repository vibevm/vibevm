//! The package-skill mutation surface over the shared no-follow cell.
//!
//! The capability machinery this module used to carry — the pinned project
//! root, no-follow directory descent, the staged-then-renamed publication and
//! the regular/single-link verification — now lives in the lower `vibe-safefs`
//! crate, because create-phase agent outputs need byte-identical guarantees
//! and a second copy of a containment law is how the weaker copy becomes the
//! one an attacker reaches. What stays here is the one package-skill-specific
//! fact: the name of this subsystem's cross-process lock.

#[cfg(test)]
pub(crate) use vibe_safefs::{LockGuard, split_relative};
pub(crate) use vibe_safefs::{Pinned, Project};

/// This subsystem's exclusive lock, taken for one whole receipt transaction.
pub(crate) const LOCK_FILE: &str = "package-skills.lock";
