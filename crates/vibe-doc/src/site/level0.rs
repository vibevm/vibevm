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
//! This module prepares explicit generated pages and card metadata, with
//! diagnostic work files alongside them. For a package that ships no documentation the
//! pages are the ones composed here; for a `doc` package the very same
//! composition also carries the pages its author wrote, because those
//! pages are selected at their original root and copied from captured bytes.
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
//! Copied, byte for byte: selected native Spec/Doc sources, or legacy XML
//! under the spec root, at their original logical paths. This is the
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
pub(crate) mod sources;
// Public where its siblings are not: the block reader is the one piece
// of a composition that answers a question on its own — «what shape is
// this README» — and its example is worth compiling.
pub mod readme;

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
    /// Diagnostic work files and copied media for this composition.
    /// The site receives generated pages/card separately from authored admission.
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

/// Write the diagnostic composition for `source`, in `work`.
///
/// The site uses the internal prepared result rather than admitting this
/// directory as an authored package.
///
/// `work` is emptied first: a composition carries the package's current
/// bytes and nothing else, and a stale page left behind from an earlier
/// version would be published under an address its package no longer
/// claims.
pub fn compose(source: &Path, work: &Path, related: &Related) -> Result<Composed> {
    compose_prepared(source, work, related).map(|(composed, _)| composed)
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-GENERATED",
    r = 1
)]
pub(crate) fn compose_prepared(
    source: &Path,
    work: &Path,
    related: &Related,
) -> Result<(Composed, crate::build::Generated)> {
    // Only actual authored pages invoke source admission. A card-only view
    // can describe incomplete metadata without interpreting authored XML.
    let manifest = card::read(source)?;
    let authored = if source.join(SPEC_ROOT).is_dir() || manifest.has_source_declaration() {
        Some(std::sync::Arc::new(sources::SourcePages::capture(
            source,
            &manifest.text,
        )?))
    } else {
        None
    };
    let mut pages = authored
        .as_ref()
        .map(|source| source.set.clone())
        .unwrap_or_default();
    if work.exists() {
        std::fs::remove_dir_all(work).map_err(|e| DocError::io("clearing", work, e))?;
    }
    std::fs::create_dir_all(work).map_err(|e| DocError::io("creating", work, e))?;

    let mut composed = Composed {
        dir: work.to_path_buf(),
        ..Composed::default()
    };

    std::fs::write(work.join("vibe.toml"), card::synthesise(&manifest))
        .map_err(|e| DocError::io("writing", work.join("vibe.toml"), e))?;
    card::copy_media(source, work, &manifest, &mut composed.notes)?;
    card::copy_reviews(source, work)?;

    if let Some(authored) = &authored {
        authored.recheck()?;
        composed.copied = copy_specs(work, &authored.copies)?;
    }
    page::compose(source, work, &manifest, related, &mut composed, &mut pages)?;
    pages.pages.sort_by(|a, b| a.rel.cmp(&b.rel));
    let metadata: toml::Value = toml::from_str(&card::synthesise(&manifest))
        .map_err(|e| DocError::manifest(work, format!("generated card does not parse: {e}")))?;
    let card = crate::manifest::Card::from_value(&metadata, work)?;
    if let Some(authored) = &authored {
        authored.recheck()?;
    }
    Ok((
        composed,
        crate::build::Generated {
            pages,
            card,
            authored,
        },
    ))
}

/// Parse only bytes explicitly produced by the level-zero/failure generator.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-GENERATED",
    r = 1
)]
pub(crate) fn generated_page(
    set: &mut crate::pages::PageSet,
    rel: &str,
    path: PathBuf,
    text: &str,
) -> Result<()> {
    let doc = vibe_specdoc::from_xml_with(text, vibe_specdoc::Vocabulary::Doc)
        .map_err(|e| DocError::manifest(&path, format!("generated page does not parse: {e}")))?;
    set.pages.push(crate::pages::Page {
        rel: rel.to_owned(),
        path,
        doc,
    });
    Ok(())
}

/// Copy captured native bytes at their original logical paths.
fn copy_specs(work: &Path, copies: &[(String, Vec<u8>)]) -> Result<usize> {
    let into = work.join(SPEC_ROOT);
    let mut copied = 0;
    for (rel, bytes) in copies {
        let target = into.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DocError::io("creating", parent, e))?;
        }
        std::fs::write(&target, bytes).map_err(|e| DocError::io("writing", &target, e))?;
        copied += 1;
    }
    Ok(copied)
}

#[cfg(test)]
mod tests;
