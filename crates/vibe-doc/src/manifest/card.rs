//! Presentation card parsing shared by authored manifests and explicit generated inputs.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#CARD-FIELDS");

use super::{navigation, relation};
use crate::error::{DocError, Result};
use std::path::Path;
use vibe_wire::generated::doc_manifest::{Authorship, BridgeAuthorship, Navigation, PackageKind};

/// Presentation fields read from TOML metadata, not an admission decision.
/// Authored callers separately perform full manifest/source-policy admission;
/// generated callers supply explicit producer metadata for their subject.
#[derive(Debug, Clone)]
pub(crate) struct Card {
    pub(super) group: String,
    pub(super) group_parsed: vibe_core::Group,
    pub(super) name: String,
    pub(super) version: semver::Version,
    pub(super) title: String,
    pub(super) description: Option<String>,
    pub(super) abstract_: String,
    pub(crate) lang: String,
    /// Who wrote the prose, when the package says so.
    pub(super) authorship: Option<Authorship>,
    /// Which of the eight kinds the package declares itself to be, when
    /// it declares one the register knows.
    pub(super) kind: Option<PackageKind>,
    pub(super) original_kind: Option<vibe_core::PackageKind>,
    /// What the package asked its navigation to look like, when it
    /// asked.
    pub(super) navigation: Option<Navigation>,
    /// Whether this rendering is the site's own projection of a
    /// package's bytes rather than pages an author wrote.
    pub(super) projection: bool,
    /// The two authorships a bridge keeps apart, when this is one.
    pub(super) bridge: Option<BridgeAuthorship>,
    /// `<coordinate>` → the semver constraint, in declaration order.
    pub(super) documents: Vec<(String, String)>,
    pub(super) translates: Option<(String, String)>,
}

impl Card {
    pub(crate) fn coordinate(&self) -> String {
        format!("{}/{}", self.group, self.name)
    }

    pub(crate) fn placeholder_kind(&self) -> vibe_core::PackageKind {
        self.original_kind.unwrap_or(vibe_core::PackageKind::Doc)
    }

    pub(super) fn read(package_dir: &Path) -> Result<Card> {
        let path = package_dir.join(crate::derived::manifest::MANIFEST);
        let text = std::fs::read_to_string(&path).map_err(|e| DocError::io("reading", &path, e))?;
        let parsed: toml::Value = toml::from_str(&text)
            .map_err(|e| DocError::manifest(&path, format!("does not parse: {e}")))?;
        Self::from_value(&parsed, &path)
    }

    /// Read presentation metadata supplied by a generated-page producer.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-GENERATED",
        r = 1
    )]
    pub(crate) fn from_value(parsed: &toml::Value, path: &Path) -> Result<Card> {
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
                    path,
                    format!(
                        "carries no `[package].{key}`, which a package of kind `doc` must \
                         declare for its card"
                    ),
                )
            })
        };
        let group = required("group")?;
        let group_parsed = vibe_core::Group::parse(&group)
            .map_err(|e| DocError::manifest(path, format!("`[package].group`: {e}")))?;
        let version = required("version")?;
        let version = semver::Version::parse(&version)
            .map_err(|e| DocError::manifest(path, format!("`[package].version`: {e}")))?;
        // The word the package calls itself by, read once and answering
        // two questions: which of the eight marks a card wears, and
        // whether the rendering is a projection at all.
        let kind = field("kind");
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
            kind: kind.as_deref().and_then(package_kind),
            original_kind: kind.as_deref().and_then(|word| word.parse().ok()),
            navigation: navigation::read(parsed),
            // A rendering is a projection unless the package it renders
            // is documentation. Read off the KIND rather than off a
            // marker the composition would have to remember to write:
            // the level-0 render of a `doc` package is the pages its
            // author wrote, and of everything else it is the site's own
            // view of bytes (`##LEVEL-ZERO-MARKED`).
            projection: kind.as_deref() != Some(DOC_KIND),
            bridge: bridge(parsed),
            documents: relation(parsed, "documents"),
            translates: relation(parsed, "translates").into_iter().next(),
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

/// Read `[package].kind` as the register spells it (PROP-000
/// `##KIND-SET`).
///
/// A word outside the eight reads as absent, for the reason a misspelled
/// `authorship` does — and for one more that is not a matter of posture.
/// An unrecognised original kind stays absent on a generated card.
/// Mapping it to a variant would mint a kind from missing information.
fn package_kind(word: &str) -> Option<PackageKind> {
    match word {
        "flow" => Some(PackageKind::Flow),
        "feat" => Some(PackageKind::Feat),
        "stack" => Some(PackageKind::Stack),
        "tool" => Some(PackageKind::Tool),
        "mcp" => Some(PackageKind::Mcp),
        "lang" => Some(PackageKind::Lang),
        DOC_KIND => Some(PackageKind::Doc),
        "app" => Some(PackageKind::App),
        "remap" => Some(PackageKind::Remap),
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
