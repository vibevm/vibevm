//! `[[package.subskills_active]]` — one subskill of a locked package, with
//! the delivery mode it resolved to and the files it contributed.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#lockfile");

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// One subskill entry under a package's `subskills_active` list.
///
/// ```
/// use vibe_core::manifest::LockedSubskill;
///
/// let s: LockedSubskill = toml::from_str(r#"
///     path = "stack/rust"
///     delivery = "eager"
///     files_written = ["spec/boot/15-flow-wal-rust.md"]
/// "#).unwrap();
/// assert_eq!(s.path, "stack/rust");
/// assert_eq!(s.delivery, "eager");
/// assert_eq!(s.files_written.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedSubskill {
    /// Subskill canonical path within the parent package, e.g.
    /// `stack/rust`.
    pub path: String,
    /// Resolved delivery mode — `eager` / `lazy-push` / `lazy-pull`.
    pub delivery: String,
    /// PURL inherited or declared on the subskill, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub describes: Option<String>,
    /// Project-relative files this subskill *specifically* contributed
    /// — distinct from the package-level `files_written` aggregate.
    /// Empty for `delivery=lazy-pull` since those files are never
    /// materialised on disk; the per-subskill index lives in the
    /// lockfile so `vibe-mcp::read_subskill` can resolve them by
    /// loading from the package cache rather than the project tree.
    /// Empty for legacy `subskills_active` entries written before
    /// this field landed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files_written: Vec<PathBuf>,

    /// For `delivery=lazy-pull`: the files the subskill carries
    /// inside the package cache, relative to the subskill's own
    /// root (`<cache>/subskills/<path>/<...>`). Used by
    /// `vibe-mcp::read_subskill` to fetch on-demand without ever
    /// touching the project tree.
    /// Absent for `eager` / `lazy-push` deliveries (they
    /// materialise into the project, recorded under
    /// `files_written` above).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cache_files: Vec<PathBuf>,
}
