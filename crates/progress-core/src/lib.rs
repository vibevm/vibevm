//! # progress-core — Progress Control (PROP-043)
//!
//! The standalone core of the inline `<status>` markup system: parse a
//! Markdown tree, validate the closed vocabularies and placement law,
//! roll statuses up, render reports, and maintain the campaign data
//! contracts (cache, state projections, baseline, journal).
//!
//! Separability law (PROP-043 §2): this crate depends on **no** vibevm
//! subsystem. `vibe progress` (in vibe-cli) is an adapter over this API;
//! any other tool can embed the same core.
//!
//! ```
//! let doc = progress_core::parse::parse_document(
//!     "spec/x.md",
//!     "<status stage=\"impl\" state=\"work\"/>\n\n# T {#t}\n\n##b1 @test/plan Body.\n",
//! );
//! assert_eq!(doc.markers.len(), 2);
//! assert_eq!(doc.error_count(), 0);
//! let rollup = progress_core::rollup::rollup_doc(&doc);
//! assert!(rollup.explicit.is_some());
//! ```

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#root");

pub mod baseline;
pub mod cache;
pub mod doc;
pub mod element;
pub mod evidence;
pub mod journal;
pub mod model;
pub mod parse;
pub mod report;
pub mod rollup;
pub mod scope;
pub mod state;
pub mod weave;

use anyhow::{Context, Result};
use std::path::Path;

/// One full scan of an observed tree: hash every in-scope file, take its
/// parse from the cache when the record is current for those bytes, parse
/// it when it is not, and refresh the cache either way.
///
/// The cache may start empty — an empty cache is a cold scan, never an
/// error. Reuse is decided per file by content hash alone (PROP-043 §7.1),
/// so a warm scan and a cold one return the same documents; the cache
/// accelerates the scan and never changes its answer.
///
/// ```
/// use progress_core::{cache::Cache, scope::ScopeConfig, scan_tree};
///
/// let dir = tempfile::tempdir().expect("tempdir");
/// std::fs::create_dir_all(dir.path().join("spec")).expect("mkdir");
/// std::fs::write(
///     dir.path().join("spec/a.md"),
///     "# A {#a}\n\n<status stage=\"impl\" state=\"work\"/>\n\n##b1 Body. @test/plan\n",
/// ).expect("write");
///
/// let mut cache = Cache::default();
/// let docs = scan_tree(dir.path(), &ScopeConfig::default(), &mut cache)
///     .expect("scan");
/// assert_eq!(docs.len(), 1);
/// assert_eq!(docs[0].markers.len(), 2);
/// assert!(cache.is_current("spec/a.md", &docs[0].content_hash));
/// ```
pub fn scan_tree(
    root: &Path,
    cfg: &scope::ScopeConfig,
    cache: &mut cache::Cache,
) -> Result<Vec<doc::ParsedDoc>> {
    let files = scope::observed_files(root, cfg)?;
    let mut docs = Vec::new();
    for rel in files {
        let full = root.join(&rel);
        let text = std::fs::read_to_string(&full)
            .with_context(|| format!("reading {}", full.display()))?;
        let path = scope::rel_str(&rel);
        let hash = parse::content_hash(&text);
        let doc = match cache.cached_doc(&path, &hash) {
            Some(cached) => cached.clone(),
            None => parse::parse_document(&path, &text),
        };
        let r = rollup::rollup_doc(&doc);
        cache.upsert(&doc, &r);
        docs.push(doc);
    }
    cache.touch();
    Ok(docs)
}
