//! Parse `vibe.toml` and `subskills/<path>/vibe-subskill.toml` into
//! [`VersionEntry`](crate::types::VersionEntry) field components.
//!
//! The scanner parses through `vibe-core`'s own [`Manifest`] and
//! [`SubskillManifest`] — the very types the rest of vibevm uses — so the
//! index can never drift from the manifest schema. The pre-de-rot scanner
//! hand-duplicated a `vibe.toml` parser; nothing tied it to `vibe-core`,
//! and it rotted silently against the M1.17 / M1.18 schema churn. PROP-005
//! §3.2 / §9 item 11 record the reversal of the standalone-workspace
//! decision this dependency rests on.
//!
//! What stays converted is narrow: `vibe-core`'s closed eight-variant
//! [`PackageKind`](vibe_core::PackageKind) against the index's open wire
//! vocabulary ([`crate::types::PackageKind`] — a re-export of the
//! generated type, `Unknown(String)` and all). The manifest side is
//! closed because `vibe.toml` is written by this build's own tooling;
//! the wire side is open because a registry serves the future.
//! [`package_kind`] converts between the two with a total `match`, and
//! `FromStr` on the open side preserves an unfamiliar string verbatim
//! for the wire to carry.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use specmark::spec;
use vibe_core::PackageKind as CorePackageKind;
use vibe_core::manifest::{Manifest, PackageMeta};

use crate::error::{Error, Result};
use crate::types::PackageKind;

mod delivery;
mod documentation;
mod relations;
mod subskills;

pub use delivery::{boot_snippet_from, embedded_sources_from, workspace_origin_from};
pub use documentation::{
    documentation_from, documents_from, i18n_from, media_from, translates_from,
};
pub use relations::{
    compatibility_from, conflicts_from, features_from, obsoletes_from, provides_from,
    requires_any_from, requires_from,
};
pub use subskills::collect_subskills;

/// Parse a `vibe.toml` byte buffer into the canonical `vibe-core`
/// [`Manifest`]. Parse / validation failures surface as
/// [`Error::Malformed`] so the scan driver records a skip note for the
/// offending package rather than aborting the whole reindex.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#entry",
    r = 1
)]
pub fn parse_manifest(bytes: &[u8]) -> Result<Manifest> {
    let s = std::str::from_utf8(bytes)
        .map_err(|e| Error::Malformed(format!("vibe.toml is not UTF-8: {e}")))?;
    Manifest::parse_str(s).map_err(|e| Error::Malformed(format!("vibe.toml: {e}")))
}

/// The `[package]` table — every indexable node is a publishable package.
/// A manifest without one (a plain `[project]`, a bare `[workspace]`) is
/// not an index entry.
pub fn require_package(manifest: &Manifest) -> Result<&PackageMeta> {
    manifest.package.as_ref().ok_or_else(|| {
        Error::Malformed(
            "vibe.toml carries no [package] table — not a publishable package".to_string(),
        )
    })
}

/// Map a `vibe-core` package kind onto the index's own [`PackageKind`].
/// See the module docs for why the index keeps its own enum.
pub fn package_kind(kind: CorePackageKind) -> PackageKind {
    match kind {
        CorePackageKind::Flow => PackageKind::Flow,
        CorePackageKind::Feat => PackageKind::Feat,
        CorePackageKind::Stack => PackageKind::Stack,
        CorePackageKind::Tool => PackageKind::Tool,
        CorePackageKind::Mcp => PackageKind::Mcp,
        CorePackageKind::Lang => PackageKind::Lang,
        CorePackageKind::Doc => PackageKind::Doc,
        CorePackageKind::App => PackageKind::App,
    }
}

#[cfg(test)]
mod tests;
