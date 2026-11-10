//! One package tree → one [`VersionEntry`] — the projection every birth
//! path of an entry shares: the org scanner (a tag materialised into a
//! scratch tree), `vibe-index add` (a package directory) and the layout
//! registry scanner (`<group>/<name>/v<version>/`, [`crate::layout`]).
//! Where the bytes came from is the caller's knowledge and arrives as an
//! [`EntrySource`]; everything else is read off the tree.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#entry");

use std::path::Path;

use chrono::{DateTime, Utc};

use crate::content_hash::compute_content_hash;
use crate::error::{Error, Result};
use crate::scanner::manifest as mfst;
use crate::types::VersionEntry;

/// The facts about an entry that the tree itself cannot tell.
#[derive(Debug, Clone)]
pub struct EntrySource {
    pub source_url: String,
    pub source_ref: String,
    pub resolved_commit: Option<String>,
    pub registry: String,
    pub indexed_at: DateTime<Utc>,
    pub indexed_by: String,
}

/// Read `dir/vibe.toml`, hash `dir`, and assemble the entry.
///
/// ```
/// use vibe_index::scanner::tree_entry::{EntrySource, entry_from_tree};
///
/// let dir = tempfile::tempdir().unwrap();
/// std::fs::write(
///     dir.path().join("vibe.toml"),
///     "[package]\ngroup='org.corp'\nname='demo'\nkind='flow'\nversion='1.0.0'\n",
/// )
/// .unwrap();
/// let entry = entry_from_tree(
///     dir.path(),
///     EntrySource {
///         source_url: "file:///srv/vibe#org.corp/demo/v1.0.0".into(),
///         source_ref: "v1.0.0".into(),
///         resolved_commit: None,
///         registry: "corp".into(),
///         indexed_at: chrono::DateTime::UNIX_EPOCH,
///         indexed_by: "doctest".into(),
///     },
/// )
/// .unwrap();
/// assert_eq!(entry.name, "demo");
/// assert_eq!(entry.files_count, 1);
/// ```
pub fn entry_from_tree(dir: &Path, source: EntrySource) -> Result<VersionEntry> {
    let manifest_path = dir.join("vibe.toml");
    let manifest_bytes = std::fs::read(&manifest_path).map_err(|e| Error::Io {
        path: manifest_path.clone(),
        message: e.to_string(),
    })?;
    let manifest = mfst::parse_manifest(&manifest_bytes)?;
    let pkg = mfst::require_package(&manifest)?;
    let content_hash = compute_content_hash(dir)?;
    let files_count = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count() as u32;
    Ok(VersionEntry {
        schema_version: VersionEntry::SCHEMA_VERSION,
        kind: mfst::package_kind(pkg.kind),
        group: pkg.group.clone(),
        name: pkg.name.clone(),
        version: pkg.version.clone(),
        content_hash,
        source_url: source.source_url,
        source_ref: source.source_ref,
        resolved_commit: source.resolved_commit,
        registry: source.registry,
        workspace_origin: mfst::workspace_origin_from(&manifest.origin),
        license: pkg.license.clone(),
        authors: pkg.authors.clone(),
        description: pkg.description.clone(),
        homepage: pkg.homepage.clone(),
        keywords: pkg.keywords.clone(),
        describes: pkg.describes.as_ref().map(|p| p.to_string()),
        title: pkg.title.clone(),
        abstract_: pkg.abstract_text.clone(),
        documents: mfst::documents_from(&manifest.documents),
        documentation: mfst::documentation_from(&manifest.documentation),
        translates: mfst::translates_from(&manifest.translates),
        media: mfst::media_from(&manifest.media),
        bridge: pkg.bridge,
        embedded_sources: mfst::embedded_sources_from(&manifest.embedded_sources),
        compatibility: mfst::compatibility_from(&manifest.compatibility),
        provides: mfst::provides_from(&manifest.provides),
        requires: mfst::requires_from(&manifest.requires),
        requires_any: mfst::requires_any_from(&manifest.requires_any),
        obsoletes: mfst::obsoletes_from(&manifest.obsoletes),
        conflicts: mfst::conflicts_from(&manifest.conflicts),
        features: mfst::features_from(&manifest.features),
        subskills: mfst::collect_subskills(dir)?,
        i18n: mfst::i18n_from(&manifest.i18n),
        boot_snippet: mfst::boot_snippet_from(&manifest.boot_snippet),
        files_count,
        must_understand: Vec::new(),
        yanked: false,
        frozen: pkg.frozen,
        // PROP-075 `#TEMPLATE-DISCOVERY`: the projection reads both off
        // the manifest, so an indexed registry and a layout registry
        // agree by construction — `vibe_index::layout::project` derives
        // the index from exactly these manifests.
        template: pkg.template,
        channels: pkg.channels.clone(),
        indexed_at: source.indexed_at,
        indexed_by: source.indexed_by,
    })
}
