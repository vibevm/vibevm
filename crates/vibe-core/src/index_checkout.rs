//! The `[index]` table of the machine-global `~/.vibe/registry.toml` — a
//! maintainer's local checkout of a registry's git-hosted index.
//!
//! It lives in the user-level file beside `[[registry]]`, never in a
//! project `vibe.toml`: where a maintainer keeps the index repository is
//! a fact about one machine, and the publish step that writes into it
//! (`vibe registry publish`, PROP-005 `##PUBLISH-UPDATES-INDEX`) runs from
//! that machine. `--index-checkout` on the command line wins over it.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#global-config");

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// `[index]` in `~/.vibe/registry.toml`.
///
/// ```
/// use vibe_core::GlobalRegistryConfig;
///
/// let g: GlobalRegistryConfig =
///     toml::from_str("[index]\ncheckout = \"/home/u/src/index\"\n").unwrap();
/// assert_eq!(g.index.checkout.unwrap().to_str(), Some("/home/u/src/index"));
/// // Absent, the table is empty and the publish step is skipped.
/// assert!(GlobalRegistryConfig::default().index.is_unset());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexCheckoutSection {
    /// The local checkout of the index repository (its data directory is
    /// the working copy's root, PROP-005 `DATA-DIR-IS-WORKTREE`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkout: Option<PathBuf>,
}

impl IndexCheckoutSection {
    /// Nothing configured — the table is left out when the file is written.
    pub fn is_unset(&self) -> bool {
        self.checkout.is_none()
    }
}
