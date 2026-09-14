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
    AdaptedSource, Audience, Authorship, BridgeAuthorship, CardMedia, DocManifest, DocPackage,
    DocPage, DocumentedSubject, Navigation, NavigationSection,
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
    // The card's images: their addresses, from the one place that decides
    // them. A declared image is read to take its content name, which is
    // the price of an address a cache may keep forever.
    let coordinate = format!("{}/{}", card.group, card.name);
    let media = crate::media::slots(package_dir, &coordinate)?;
    Ok(assemble(&card, &set, &reviews, &media, sources, options))
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
    media: &[crate::media::Slot],
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
        // Carried, never inferred: a documentation that says nothing
        // about its prose is unknown, and the site shows no badge for it
        // rather than deciding on its behalf (`##CARD-AUTHORSHIP`).
        authorship: card.authorship.clone(),
        // What the site made and what an author wrote are different
        // pages, and only the manifest can tell a shelf which one it is
        // looking at (`##LEVEL-ZERO-MARKED`).
        projection: card.projection,
        bridge: card.bridge.clone(),
        status: status::strongest(&subjects),
        subjects,
        translation,
        audiences,
        // Named, never re-derived: the shell parses nothing and computes
        // nothing, and a content name is exactly the kind of thing a
        // second implementation would get subtly wrong (X-042). Every
        // build writes it; the member is optional on the wire because a
        // document written before it existed is still a document.
        media: Some(CardMedia {
            icon: crate::media::address_of(media, "icon"),
            banner: crate::media::address_of(media, "banner"),
            preview: crate::media::address_of(media, "preview"),
        }),
        published_at: options.published_at,
        rendered_at: options.rendered_at,
    };

    Built {
        manifest: DocManifest {
            schema_version: SCHEMA_VERSION,
            package,
            // Carried as the package wrote it, and applied to nothing
            // here: `pages` stays in the layer law's order, and where a
            // pinned page STANDS is the reader's business
            // (`##NAV-PINNED`). A projection that also reordered the
            // list would give the site two orders to choose between.
            navigation: card.navigation.clone(),
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
    /// Who wrote the prose, when the package says so.
    authorship: Option<Authorship>,
    /// What the package asked its navigation to look like, when it
    /// asked.
    navigation: Option<Navigation>,
    /// Whether this rendering is the site's own projection of a
    /// package's bytes rather than pages an author wrote.
    projection: bool,
    /// The two authorships a bridge keeps apart, when this is one.
    bridge: Option<BridgeAuthorship>,
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
            authorship: field("authorship").as_deref().and_then(authorship),
            navigation: navigation(&parsed),
            // A rendering is a projection unless the package it renders
            // is documentation. Read off the KIND rather than off a
            // marker the composition would have to remember to write:
            // the level-0 render of a `doc` package is the pages its
            // author wrote, and of everything else it is the site's own
            // view of bytes (`##LEVEL-ZERO-MARKED`).
            projection: field("kind").as_deref() != Some(DOC_KIND),
            bridge: bridge(&parsed),
            documents: relation(&parsed, "documents"),
            translates: relation(&parsed, "translates").into_iter().next(),
        })
    }
}

/// The kind whose pages an author wrote, and therefore the one kind
/// whose rendering is not a projection.
const DOC_KIND: &str = "doc";

/// Read `[package].authorship` as the wire spells it.
///
/// A word outside the three reads as absent, which is this reader's
/// standing posture towards a manifest it did not write: the question
/// here is «what does this package say about itself», and the place that
/// REFUSES a misspelled value is the manifest grammar (`vibe check`), not
/// a projection that would otherwise refuse to render a whole package
/// over one word.
fn authorship(word: &str) -> Option<Authorship> {
    match word {
        "human" => Some(Authorship::Human),
        "ai" => Some(Authorship::Ai),
        "mixed" => Some(Authorship::Mixed),
        _ => None,
    }
}

/// The two authorships of a bridge, when this package is one.
///
/// Both lists are already in the manifest and neither is computed here:
/// `[package].authors` names who wrote the WRAPPER, and the
/// `upstream_authors` of `[[embedded_source]]` name who wrote the bytes
/// it points at. PROP-023 `##AUTHORSHIP-SEPARATION` keeps them apart in
/// the manifest, and this keeps them apart on the wire, so no shell has
/// to decide which of two lists a name belongs to.
///
/// `None` for a package that is not a bridge: the block would then be an
/// invitation to print «maintainers» over the authors of the work
/// itself.
fn bridge(parsed: &toml::Value) -> Option<BridgeAuthorship> {
    let package = parsed.get("package")?;
    if package.get("bridge").and_then(toml::Value::as_bool) != Some(true) {
        return None;
    }
    let list = |value: Option<&toml::Value>| -> Vec<String> {
        value
            .and_then(toml::Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let sources: Vec<&toml::Value> = match parsed.get("embedded_source") {
        Some(toml::Value::Array(entries)) => entries.iter().collect(),
        Some(entry) => vec![entry],
        None => Vec::new(),
    };

    // Declaration order, each name once: a reader is being told who
    // wrote the work, not how many sources repeat a name.
    let mut upstream_authors: Vec<String> = Vec::new();
    for source in &sources {
        for author in list(source.get("upstream_authors")) {
            if !upstream_authors.contains(&author) {
                upstream_authors.push(author);
            }
        }
    }

    // One line cannot state two licences, so sources that disagree
    // state none: a page naming the first would be asserting a legal
    // fact that is not true.
    let licenses: Vec<&str> = sources
        .iter()
        .filter_map(|source| source.get("upstream_license"))
        .filter_map(toml::Value::as_str)
        .collect();
    let upstream_license = match licenses.split_first() {
        Some((first, rest)) if rest.iter().all(|other| other == first) => Some((*first).to_owned()),
        _ => None,
    };

    Some(BridgeAuthorship {
        maintainers: list(package.get("authors")),
        upstream_authors,
        upstream_license,
    })
}

/// Read `[navigation]` as the wire carries it: the pinned paths in the
/// order they were written, and one row per named section.
///
/// Read as data like the rest of the card, and `None` when the package
/// says nothing — which is the state of every documentation written
/// before the table existed, and the state the site renders as «the
/// pages in the order the manifest gives them».
fn navigation(parsed: &toml::Value) -> Option<Navigation> {
    let table = parsed.get("navigation")?;
    let strings = |value: Option<&toml::Value>| -> Vec<String> {
        value
            .and_then(toml::Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let sections = table
        .get("section")
        .and_then(toml::Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    Some(NavigationSection {
                        id: row.get("id").and_then(toml::Value::as_str)?.to_owned(),
                        title: row.get("title").and_then(toml::Value::as_str)?.to_owned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Navigation {
        pinned: strings(table.get("pinned")),
        sections,
    })
}

/// One relation table of a package's manifest — `documents` or
/// `translates` — as `(coordinate, version constraint)` pairs, read
/// straight off the file.
///
/// The narrow door for a caller that wants one relation and not a whole
/// card: the translation check asks what a package adapts, and making it
/// build a card first would make a missing `title` look like a
/// translation defect.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// std::fs::write(
///     dir.path().join("vibe.toml"),
///     "[translates]\npackage = \"org.demo/lib-docs\"\nversion = \"^0.1\"\n",
/// )
/// .unwrap();
///
/// let adapted = vibe_doc::manifest::relations(dir.path(), "translates").unwrap();
/// assert_eq!(adapted, vec![("org.demo/lib-docs".to_owned(), "^0.1".to_owned())]);
/// assert!(vibe_doc::manifest::relations(dir.path(), "documents").unwrap().is_empty());
/// ```
pub fn relations(package_dir: &Path, key: &str) -> Result<Vec<(String, String)>> {
    let path = package_dir.join(crate::derived::manifest::MANIFEST);
    let text = std::fs::read_to_string(&path).map_err(|e| DocError::io("reading", &path, e))?;
    let parsed: toml::Value = toml::from_str(&text)
        .map_err(|e| DocError::manifest(&path, format!("does not parse: {e}")))?;
    Ok(relation(&parsed, key))
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

/// The language a documentation package is written in — `[i18n]
/// canonical`, or the project default when the manifest declares none.
///
/// One documentation package is one language
/// (PROP-057 `##LOC-PACKAGE-PER-LANGUAGE`),
/// so this is a property of the PACKAGE and never of a page. The style
/// linter asks it to pick the banned list; the build asks it to refuse a
/// `--lang` the package does not hold.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// std::fs::write(
///     dir.path().join("vibe.toml"),
///     "[package]\nname = \"m-ru\"\n\n[i18n]\ncanonical = \"ru\"\n",
/// )
/// .unwrap();
///
/// assert_eq!(vibe_doc::manifest::language(dir.path()).unwrap(), "ru");
/// ```
pub fn language(package_dir: &Path) -> Result<String> {
    let path = package_dir.join(crate::derived::manifest::MANIFEST);
    let text = std::fs::read_to_string(&path).map_err(|e| DocError::io("reading", &path, e))?;
    let parsed: toml::Value = toml::from_str(&text)
        .map_err(|e| DocError::manifest(&path, format!("does not parse: {e}")))?;
    Ok(parsed
        .get("i18n")
        .and_then(|i| i.get("canonical"))
        .and_then(toml::Value::as_str)
        .unwrap_or(vibe_core::manifest::i18n::DEFAULT_CANONICAL_LANGUAGE)
        .to_owned())
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
