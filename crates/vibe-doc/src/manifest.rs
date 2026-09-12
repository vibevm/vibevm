//! The page manifest — one documentation package, one language, every
//! page (PROP-057 `##SEO-MANIFEST-AND-RESOLVER`, `##DISC-MACHINE-MIRROR`).
//!
//! This is the document everything machine-facing is built from. The
//! site's navigation is folded out of it, every `llms` tier is rendered
//! from it ([`crate::llms`]), the shell reads it as a JTD contract with
//! generated types, and a crawler fetches it as `/doc/manifest.json`. One
//! walk of the package answers all four, which is the point: a second
//! walk would be a second answer, and the day they disagreed nobody would
//! know which one the reader saw.
//!
//! ## Four things the manifest deliberately does not carry
//!
//! **No officiality flag.** `primary`, `official` and `community` are
//! COMPUTED here, at every build, from the convergence of two edges — the
//! subject naming its documentation and the documentation naming its
//! subject (`##REL-OFFICIAL-IS-CONVERGENCE`). A stored `official = true`
//! is a design error by name (`##REL-NO-OFFICIAL-FLAG`); this document is
//! a render, which is exactly why it may hold the answer.
//!
//! **No clock.** Both instants arrive in [`Options`]. A writer that calls
//! `now()` produces different bytes for the same tree, and «the same
//! sources render to the same bytes» is what makes a site build cacheable
//! (PROP-044 `##M-CANONICAL-BYTES`).
//!
//! **No product.** Building a manifest runs no binary and generates no
//! `derived` block. It is a function of the package's own bytes, the
//! subjects its sources can reach, and the two instants. That is what
//! lets a reading time and a navigation exist for a package warmed into
//! the store on a machine that has never built anything.
//!
//! **No history.** No revision, no content hash, no «behind by», no «a
//! new version is available». Those need a past the project does not keep
//! by design (`##OBS-VERSION-CONTRACT`, `##OBS-NOTHING-LEAKS`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SEO-MANIFEST-AND-RESOLVER");

pub mod layer;
pub mod page;
pub mod reviews;
pub mod status;

use std::path::Path;

use chrono::{DateTime, Utc};
use vibe_wire::generated::doc_manifest::{
    AdaptedSource, Audience, DocManifest, DocPackage, DocPage, DocumentedSubject,
};

use crate::citations::SpecSources;
use crate::error::{DocError, Result};
use crate::pages::{self, PageSet};

pub use reviews::Reviews;

/// The manifest's own version, written into every document this library
/// produces. It moves when the shape moves, and the format registry's
/// epoch moves with it (`formats/REGISTRY.toml`, record `doc-manifest`).
pub const SCHEMA_VERSION: u32 = 1;

/// The file the package keeps its read-aloud dates in.
pub const REVIEWS: &str = "reviews.toml";

/// What the build knows that the package cannot.
///
/// Both fields are instants the CALLER supplies. The library reads no
/// clock and no environment: a documentation build must render the same
/// bytes from the same tree, and a `now()` inside here would quietly make
/// that false.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// When this build rendered the pages — one of the exactly two dates
    /// a reader may see (`##READER-META-AND-PRINT`).
    pub rendered_at: DateTime<Utc>,
    /// When this version of the package was published, when it was
    /// published at all. `None` for a package read from a checkout or an
    /// in-tree registry: it has no publication date, and inventing one
    /// would put a date in a sitemap that nothing on earth supports.
    pub published_at: Option<DateTime<Utc>>,
}

impl Options {
    /// A build at `rendered_at` of something never published.
    pub fn at(rendered_at: DateTime<Utc>) -> Options {
        Options {
            rendered_at,
            published_at: None,
        }
    }

    /// The same build, of a package published at `published_at`.
    #[must_use]
    pub fn published(mut self, published_at: DateTime<Utc>) -> Options {
        self.published_at = Some(published_at);
        self
    }
}

/// Build the manifest of the documentation package at `package_dir`.
///
/// `sources` is the world the statuses are computed against — the same
/// four sources a `rule` citation resolves through
/// ([`SpecSources`]). A subject no source holds yields `community`, which
/// is not a fallback but the definition: community documentation is the
/// one whose only known edge is its own (`##REL-OFFICIAL-IS-CONVERGENCE`).
///
/// A page the pivot refuses is NOT fatal. It is left out of `pages` and
/// named in the returned [`Built::unreadable`], because a manifest that
/// aborts on the first bad page reports one defect per run, and the
/// checks — not a projection — are where a refusal belongs.
///
/// ```no_run
/// use chrono::{TimeZone, Utc};
/// use vibe_doc::citations::SpecSources;
/// use vibe_doc::manifest::{self, Options};
///
/// let world = SpecSources::for_checkout("/repo", Some("org.vibevm.core"), "vibevm");
/// let when = Utc.with_ymd_and_hms(2026, 9, 12, 0, 0, 0).unwrap();
/// let built = manifest::build(
///     std::path::Path::new("/repo/vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0"),
///     &world,
///     &Options::at(when),
/// )
/// .unwrap();
/// assert_eq!(built.manifest.package.name, "vibevm-docs");
/// ```
pub fn build(package_dir: &Path, sources: &SpecSources, options: &Options) -> Result<Built> {
    let card = Card::read(package_dir)?;
    let set = pages::read_package(package_dir)?;
    let reviews = Reviews::read(package_dir.join(REVIEWS))?;
    Ok(assemble(&card, &set, &reviews, sources, options))
}

/// One built manifest and the pages that did not parse.
#[derive(Debug, Clone)]
pub struct Built {
    pub manifest: DocManifest,
    /// Page addresses the pivot refused, in walk order.
    pub unreadable: Vec<String>,
}

fn assemble(
    card: &Card,
    set: &PageSet,
    reviews: &Reviews,
    sources: &SpecSources,
    options: &Options,
) -> Built {
    let mut rows: Vec<DocPage> = set
        .pages
        .iter()
        .map(|p| page::row(p, &card.lang, reviews))
        .collect();
    layer::order(&mut rows, &set.pages);

    let subjects: Vec<DocumentedSubject> = card
        .documents
        .iter()
        .map(|(subject, constraint)| DocumentedSubject {
            package: subject.clone(),
            version: constraint.clone(),
            status: status::documentation_status(subject, &card.group, &card.name, sources),
        })
        .collect();

    let translation = card.translates.as_ref().map(|(source, constraint)| {
        let status = status::translation_status(source, &card.group, &card.name, &card.lang);
        AdaptedSource {
            package: source.clone(),
            version: constraint.clone(),
            status,
        }
    });

    // The audiences of the whole documentation are the audiences of its
    // pages, which is what `##CARD-NO-AUDIENCE-DECLARATION` means when it
    // says they are derived from the markup rather than declared.
    let audiences: Vec<Audience> = AUDIENCES
        .iter()
        .filter(|a| rows.iter().any(|row| row.audiences.contains(a)))
        .cloned()
        .collect();

    let package = DocPackage {
        group: card.group_parsed.clone(),
        name: card.name.clone(),
        version: card.version.clone(),
        // The publisher IS the group; it is written out rather than left
        // to be re-derived because the shell computes nothing
        // (`##PIPE-SHELL-PARSES-NOTHING`).
        publisher: card.group.clone(),
        title: card.title.clone(),
        description: card.description.clone(),
        abstract_: card.abstract_.clone(),
        lang: card.lang.clone(),
        status: status::strongest(&subjects),
        subjects,
        translation,
        audiences,
        published_at: options.published_at,
        rendered_at: options.rendered_at,
    };

    Built {
        manifest: DocManifest {
            schema_version: SCHEMA_VERSION,
            package,
            pages: rows,
        },
        unreadable: set.unreadable.iter().map(|u| u.rel.clone()).collect(),
    }
}

/// Render a manifest as the bytes `/doc/manifest.json` serves: pretty
/// JSON with a closing newline, the shape every other machine document in
/// this repository is written in.
pub fn to_json(manifest: &DocManifest) -> String {
    let mut text = serde_json::to_string_pretty(manifest)
        // The type is generated from the schema and holds only JSON
        // scalars, maps and sequences, so there is no serialisable state
        // that can fail here; a fallible signature would push an
        // impossible arm onto every caller.
        .unwrap_or_default();
    text.push('\n');
    text
}

/// The card fields a documentation package's manifest carries, read as
/// TOML data.
///
/// Data, not the typed model, for the reason the rest of this crate reads
/// a manifest that way: the question is «what does this package say about
/// itself», and a strict parse would fail over a field this library never
/// looks at.
#[derive(Debug, Clone)]
struct Card {
    group: String,
    group_parsed: vibe_core::Group,
    name: String,
    version: semver::Version,
    title: String,
    description: Option<String>,
    abstract_: String,
    lang: String,
    /// `<coordinate>` → the semver constraint, in declaration order.
    documents: Vec<(String, String)>,
    translates: Option<(String, String)>,
}

impl Card {
    fn read(package_dir: &Path) -> Result<Card> {
        let path = package_dir.join(crate::derived::manifest::MANIFEST);
        let text = std::fs::read_to_string(&path).map_err(|e| DocError::io("reading", &path, e))?;
        let parsed: toml::Value = toml::from_str(&text)
            .map_err(|e| DocError::manifest(&path, format!("does not parse: {e}")))?;
        let field = |key: &str| {
            parsed
                .get("package")
                .and_then(|p| p.get(key))
                .and_then(toml::Value::as_str)
                .map(str::to_owned)
        };
        let required = |key: &str| {
            field(key).ok_or_else(|| {
                DocError::manifest(
                    &path,
                    format!(
                        "carries no `[package].{key}`, which a package of kind `doc` must \
                         declare for its card"
                    ),
                )
            })
        };
        let group = required("group")?;
        let group_parsed = vibe_core::Group::parse(&group)
            .map_err(|e| DocError::manifest(&path, format!("`[package].group`: {e}")))?;
        let version = required("version")?;
        let version = semver::Version::parse(&version)
            .map_err(|e| DocError::manifest(&path, format!("`[package].version`: {e}")))?;
        Ok(Card {
            group,
            group_parsed,
            name: required("name")?,
            version,
            title: required("title")?,
            description: field("description"),
            abstract_: required("abstract")?.trim().to_owned(),
            lang: parsed
                .get("i18n")
                .and_then(|i| i.get("canonical"))
                .and_then(toml::Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    vibe_core::manifest::i18n::DEFAULT_CANONICAL_LANGUAGE.to_owned()
                }),
            documents: relation(&parsed, "documents"),
            translates: relation(&parsed, "translates").into_iter().next(),
        })
    }
}

/// Read `[[documents]]` or `[translates]` as pairs. Both spellings are
/// accepted for both keys — a table and an array of tables read the same
/// here — because which one a relation takes is the manifest's law
/// (`##REL-FIELD-PLACEMENT`), not this reader's, and a projection that
/// refused the other spelling would be a second opinion on it.
fn relation(parsed: &toml::Value, key: &str) -> Vec<(String, String)> {
    let read = |entry: &toml::Value| {
        let package = entry.get("package").and_then(toml::Value::as_str)?;
        let version = entry
            .get("version")
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        Some((package.to_owned(), version.to_owned()))
    };
    match parsed.get(key) {
        Some(toml::Value::Array(entries)) => entries.iter().filter_map(read).collect(),
        Some(entry) => read(entry).into_iter().collect(),
        None => Vec::new(),
    }
}

/// Every audience of a manifest, as the wire spells them — the order the
/// vocabulary is declared in, which is the order a card shows them.
pub const AUDIENCES: &[Audience] = &[
    Audience::User,
    Audience::Author,
    Audience::Dev,
    Audience::Agent,
];

#[cfg(test)]
pub(crate) mod tests;
