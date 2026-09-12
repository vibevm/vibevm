//! Level 0 — any package, rendered from its own bytes
//! (PROP-057 `##LEVEL-ZERO`, `##LEVEL-ONE`).
//!
//! «The site renders any published version of any package from its own
//! bytes: the manifest as a reference page, the README, the boot snippet
//! marked read by the session, the specs with anchors, the declared
//! skills, binaries and MCP servers, and the images of `[media]`.» That
//! is free for an author and covers the whole registry at once, the way
//! rustdoc covers every crate; level 1 — a separate `doc` package — is
//! needed only where somebody wants more.
//!
//! ## One path for both levels, and why that is not a trick
//!
//! This module writes a documentation package: a manifest with a card, a
//! spec root of pages. For a package that ships no documentation the
//! pages are the ones composed here; for a `doc` package the very same
//! composition also carries the pages its author wrote, because those
//! pages already live under the spec root and are copied as they are.
//! So one function answers both levels, and everything downstream — the
//! block numbering, the three projections, the manifest, the `llms`
//! tiers, the card's images — is the pipeline that already exists rather
//! than a second one for packages that wrote nothing.
//!
//! The alternative was a second rendering path for level 0. It would
//! have had to reimplement numbering, projections and the card, and the
//! day the two disagreed a reader would have had no way to tell which
//! page they were looking at.
//!
//! ## What is copied and what is composed
//!
//! Copied, byte for byte: every `.xml` under the package's spec root,
//! at the same relative path. That is not an optimisation — it is the
//! address map. `spec://<group>/<name>@<version>/<document>` maps to
//! `/doc/<group>/<name>/<version>/<document>/` (`##SITE-MOUNT`), so a
//! document has to keep its path or every citation to it would break.
//!
//! Composed: the manifest reference page, the README and the boot
//! snippet. They are generated into the same spec root under reserved
//! names, and a package that already carries a document under one of
//! those names keeps its own — a generated page must never overwrite an
//! authored one.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#LEVEL-ZERO");

use std::path::{Path, PathBuf};

use crate::error::{DocError, Result};
use crate::pages::SPEC_ROOT;

mod card;
mod page;
mod readme;

pub use page::{BOOT_SNIPPET_PAGE, MANIFEST_PAGE, README_PAGE};

/// What the rest of the catalog says about this package: the shelves of
/// `##REL-OFFICIAL-IS-CONVERGENCE` and `##LOC-OFFICIAL-TRANSLATION`,
/// already ranked.
///
/// It arrives as ROWS and not as a catalog to search, because a
/// composition is about one package and the fold that answered these
/// three questions was done once for the whole site
/// (`##REL-REVERSE-QUERIES-SITE-SIDE`). Empty is the honest state of a
/// package nobody documents, adapts or depends on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Related {
    pub documentation: Vec<crate::site::shelves::Row>,
    pub translations: Vec<crate::site::shelves::Row>,
    pub dependants: Vec<crate::site::shelves::Row>,
    /// How this adaptation stands against the documentation it adapts,
    /// when it is one.
    pub adaptation: Option<Adaptation>,
}

/// What an adaptation's own page says about the mirror.
///
/// Structure only, and never «how far behind». A translation stores no
/// revision and no hash of the source page (`##LOC-NO-REVISION`), so
/// «behind by» would need a history this project deliberately does not
/// keep. What CAN be said is whether the two trees still mirror — the
/// same pages, the same anchors, the same blocks — which is exactly what
/// `vibe doc check --translations` asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adaptation {
    /// The documentation this package adapts.
    pub source: String,
    /// How many pages the adaptation carries.
    pub pages: usize,
    /// How many of them do not mirror.
    pub divergences: usize,
}

impl Related {
    /// Everything the catalog says about one coordinate.
    pub fn of(shelves: &crate::site::shelves::Shelves, coordinate: &str) -> Related {
        Related {
            documentation: shelves.documentation_of(coordinate).to_vec(),
            translations: shelves.translations_of(coordinate).to_vec(),
            dependants: shelves.dependants_of(coordinate).to_vec(),
            adaptation: None,
        }
    }

    /// The same, with the mirror of an adaptation measured.
    #[must_use]
    pub fn adapting(mut self, adaptation: Option<Adaptation>) -> Related {
        self.adaptation = adaptation;
        self
    }
}

/// What one composition produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Composed {
    /// The documentation package that was written — what the rest of the
    /// pipeline is pointed at.
    pub dir: PathBuf,
    /// Spec documents copied from the package, at their own paths.
    pub copied: usize,
    /// The pages this module wrote, in the order it wrote them.
    pub generated: Vec<String>,
    /// Things worth saying out loud about this package: a generated page
    /// that gave way to an authored one, a declared image that is not
    /// there. Never fatal — a level-0 render exists precisely for
    /// packages nobody prepared for it.
    pub notes: Vec<String>,
}

/// Compose the documentation package for `source`, in `work`.
///
/// `work` is emptied first: a composition carries the package's current
/// bytes and nothing else, and a stale page left behind from an earlier
/// version would be published under an address its package no longer
/// claims.
pub fn compose(source: &Path, work: &Path, related: &Related) -> Result<Composed> {
    if work.exists() {
        std::fs::remove_dir_all(work).map_err(|e| DocError::io("clearing", work, e))?;
    }
    std::fs::create_dir_all(work).map_err(|e| DocError::io("creating", work, e))?;

    let manifest = card::read(source)?;
    let mut composed = Composed {
        dir: work.to_path_buf(),
        ..Composed::default()
    };

    std::fs::write(work.join("vibe.toml"), card::synthesise(&manifest))
        .map_err(|e| DocError::io("writing", work.join("vibe.toml"), e))?;
    card::copy_media(source, work, &manifest, &mut composed.notes)?;
    card::copy_reviews(source, work)?;

    composed.copied = copy_specs(source, work)?;
    page::compose(source, work, &manifest, related, &mut composed)?;
    Ok(composed)
}

/// Copy every spec document at its own path.
fn copy_specs(source: &Path, work: &Path) -> Result<usize> {
    let from = source.join(SPEC_ROOT);
    if !from.is_dir() {
        return Ok(0);
    }
    let into = work.join(SPEC_ROOT);
    let mut copied = 0;
    for entry in walkdir::WalkDir::new(&from).sort_by_file_name() {
        let entry = entry.map_err(|e| {
            DocError::io(
                "walking",
                from.clone(),
                e.into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("walk failed")),
            )
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("xml") {
            continue;
        }
        let Ok(rel) = path.strip_prefix(&from) else {
            continue;
        };
        let target = into.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DocError::io("creating", parent, e))?;
        }
        std::fs::copy(path, &target).map_err(|e| DocError::io("copying", path, e))?;
        copied += 1;
    }
    Ok(copied)
}

#[cfg(test)]
mod tests;
