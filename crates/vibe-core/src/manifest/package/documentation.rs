//! The documentation relations, the localization edge and the card —
//! the manifest half of [PROP-057](../../../../vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml).
//!
//! Four tables and two scalars, and the reason they are shaped this way
//! is one idea: **nothing about documentation is a stored flag**.
//! Whether a documentation package is official, which translations a
//! manual has, which version of a subject a page belongs to — every one
//! of those is computed from edges at render time. What the manifest
//! carries is only the edges themselves, declared from both ends:
//!
//! - `[[documents]]` — «I document this package», written by the
//!   documentation (`##REL-DOCUMENTS-REQUIRED`);
//! - `[documentation]` — «this is my documentation», written by the
//!   subject, and deliberately without versions
//!   (`##REL-DOCUMENTATION-UNVERSIONED`);
//! - `[translates]` — «I am an adaptation of that documentation»
//!   (`##LOC-PACKAGE-PER-LANGUAGE`);
//! - `[media]` — the card's images, as source files in the package tree
//!   (`##CARD-MEDIA-SOURCE`).
//!
//! Officiality is the convergence of the first two, and a package is
//! never asked to declare it (`##REL-NO-OFFICIAL-FLAG`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#relation");

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::is_false;

/// `[[documents]]` — one subject this documentation documents.
///
/// A `doc` package declares at least one; the `version` is a semver
/// constraint, so one documentation version can serve a range of subject
/// versions and the site picks the newest documentation whose constraint
/// admits the subject version on screen.
///
/// ```
/// use vibe_core::manifest::DocumentsDecl;
///
/// let d: DocumentsDecl = toml::from_str(r#"
///     package = "org.vibevm.world/multi-user-planning"
///     version = "^1.0"
/// "#).unwrap();
/// assert_eq!(d.package, "org.vibevm.world/multi-user-planning");
/// assert_eq!(d.version, "^1.0");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentsDecl {
    /// The subject's coordinate — `<group>/<name>`, never a versioned
    /// or kind-prefixed pkgref. The subject may be a project coordinate
    /// rather than a registry package (the host itself is one), so the
    /// form is checked and selectability is not
    /// (PROP-057 `##REL-HOST-SUBJECT`).
    pub package: String,
    /// The semver constraint on the subject's version.
    pub version: String,
}

/// `[documentation]` — the subject's own pointer at its documentation.
///
/// Written in the SUBJECT's manifest, of any kind. It carries no
/// versions: documentation almost always ships after the code it
/// describes, and a versioned pointer would force the subject to
/// re-release for every documentation edit.
///
/// ```
/// use vibe_core::manifest::DocumentationDecl;
///
/// let d: DocumentationDecl = toml::from_str(r#"
///     primary  = "org.vibevm.world/multi-user-planning-docs"
///     official = ["org.vibevm.world/multi-user-planning-tutorials"]
/// "#).unwrap();
/// assert_eq!(d.primary.as_deref(), Some("org.vibevm.world/multi-user-planning-docs"));
/// assert_eq!(d.official.len(), 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentationDecl {
    /// At most one package: the documentation a reader is sent to first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<String>,
    /// Any number of further documentation packages the subject
    /// recognises as its own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub official: Vec<String>,
}

impl DocumentationDecl {
    /// `true` when the table names nothing — lets the serializer skip a
    /// section the author never filled in.
    pub fn is_empty(&self) -> bool {
        self.primary.is_none() && self.official.is_empty()
    }
}

/// `[translates]` — the documentation this package adapts.
///
/// A translation is a package of its own, not a sidecar file: different
/// authors, different rhythms, and officiality per language are all
/// questions of ownership, and the unit of ownership here is the
/// package (PROP-057 `##LOC-PACKAGE-PER-LANGUAGE`).
///
/// ```
/// use vibe_core::manifest::TranslatesDecl;
///
/// let t: TranslatesDecl = toml::from_str(r#"
///     package = "org.vibevm.core/vibevm-docs"
///     version = "^0.3"
/// "#).unwrap();
/// assert_eq!(t.package, "org.vibevm.core/vibevm-docs");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranslatesDecl {
    /// The source documentation's coordinate — `<group>/<name>`.
    pub package: String,
    /// The semver constraint on the source's version.
    pub version: String,
}

/// `[media]` — the card's images.
///
/// Source files in the package tree, never URLs: the local reader serves
/// the images of proprietary packages exactly as they are, and a foreign
/// address would be a hole in that. Format, proportions and size are the
/// gate's business; this type carries the paths and refuses only the
/// shapes that could point outside the package
/// (PROP-057 `##CARD-MEDIA-SOURCE`).
///
/// ```
/// use vibe_core::manifest::MediaDecl;
///
/// let m: MediaDecl = toml::from_str(r#"
///     icon   = "media/icon.png"
///     banner = "media/banner.jpg"
/// "#).unwrap();
/// assert_eq!(m.icon.as_ref().unwrap().to_str(), Some("media/icon.png"));
/// assert!(m.preview.is_none());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaDecl {
    /// Square, shown in the package page header and on shelf cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<PathBuf>,
    /// Wide, heads the package page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner: Option<PathBuf>,
    /// The link preview — its own option, because a banner's
    /// proportions do not fit a link card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<PathBuf>,
}

impl MediaDecl {
    /// `true` when no image is declared.
    pub fn is_empty(&self) -> bool {
        self.icon.is_none() && self.banner.is_none() && self.preview.is_none()
    }

    /// Every declared image as `(field name, path)`, for the validator
    /// and for the gate that later opens the files.
    pub fn declared(&self) -> Vec<(&'static str, &Path)> {
        let mut out: Vec<(&'static str, &Path)> = Vec::new();
        for (field, value) in [
            ("icon", &self.icon),
            ("banner", &self.banner),
            ("preview", &self.preview),
        ] {
            if let Some(path) = value {
                out.push((field, path.as_path()));
            }
        }
        out
    }
}

/// `[navigation]` — where a documentation asks its own pages to stand,
/// and the order it asks a person to read them in.
///
/// Three statements: which pages are listed first, what the folders of
/// the page tree are called (PROP-057 `##NAV-PINNED`), and the learning
/// path (`##NAV-CHAPTERS`). None of them moves `pages`, which keeps the
/// order the layer law already gave it (PROP-048 `##THE-LAYER-LAW`): the
/// pinning is a correction to where two pages stand, and the path is a
/// second order for a second audience, carried beside the list rather
/// than replacing it. Two orders for two readers is the decision
/// (`##NAV-CHAPTERS-DECISION`) — a corpus ordered for machines by
/// mutation frequency, and for people by what one has to learn first.
///
/// ```
/// use vibe_core::manifest::NavigationDecl;
///
/// let n: NavigationDecl = toml::from_str(r#"
///     pinned = ["start/what-vibevm-is", "start/index"]
///
///     [[section]]
///     id = "start"
///     title = "Start"
///
///     [[chapter]]
///     id = "start"
///     title = "Getting started"
///     pages = ["start/what-vibevm-is", "start/index"]
/// "#).unwrap();
/// assert_eq!(n.pinned, vec!["start/what-vibevm-is", "start/index"]);
/// assert_eq!(n.sections[0].title, "Start");
/// assert_eq!(n.chapters[0].pages.as_deref().unwrap().len(), 2);
/// assert!(!n.chapters[0].appendix);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationDecl {
    /// The document paths listed first, in the order given — a page's
    /// address under the spec root WITHOUT its extension, so that the
    /// pin survives a projection into another format.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pinned: Vec<String>,
    /// `[[navigation.section]]` — one row per top-level folder of the
    /// page tree.
    #[serde(default, rename = "section", skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<NavigationSectionDecl>,
    /// `[[navigation.chapter]]` — the learning path, in the order a
    /// reader walks it. Empty for a package that declares none, which is
    /// every documentation written before the rows existed and which the
    /// reader shows exactly as it did before.
    #[serde(default, rename = "chapter", skip_serializing_if = "Vec::is_empty")]
    pub chapters: Vec<NavigationChapterDecl>,
}

impl NavigationDecl {
    /// `true` when the table says nothing at all.
    pub fn is_empty(&self) -> bool {
        self.pinned.is_empty() && self.sections.is_empty() && self.chapters.is_empty()
    }
}

/// `[[navigation.chapter]]` — one chapter of the learning path.
///
/// The id is the chapter's identity and never shown, for the reason a
/// section carries one: a translation names the same chapters as its
/// source and shows other words for them
/// (PROP-057 `##NAV-CHAPTERS-TRANSLATION`).
///
/// `pages` is an `Option` because absence and emptiness are different
/// facts here and the rule turns on the difference: a source edition
/// declares the pages a chapter holds, and a translation declares a
/// chapter's name and NO pages at all. A `pages = []` in a translation
/// is still a translation listing pages, which is refused; a row with no
/// `pages` key is the legal shape.
///
/// ```
/// use vibe_core::manifest::NavigationChapterDecl;
///
/// let source: NavigationChapterDecl = toml::from_str(r#"
///     id = "reference"
///     title = "Appendices"
///     pages = ["reference/commands", "glossary/index"]
///     appendix = true
/// "#).unwrap();
/// assert_eq!(source.pages.as_deref().unwrap()[0], "reference/commands");
/// assert!(source.appendix);
///
/// // A translation names the chapter and leaves the path to its source.
/// let adapted: NavigationChapterDecl =
///     toml::from_str("id = \"reference\"\ntitle = \"Приложения\"\n").unwrap();
/// assert!(adapted.pages.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationChapterDecl {
    /// The chapter's identity inside the package — used once, shown
    /// never.
    pub id: String,
    /// What the contents shows over the chapter's pages, in this
    /// package's own language.
    pub title: String,
    /// The document paths the chapter holds, in reading order, spelled
    /// as a pin spells them. Absent in a translation, which takes the
    /// path of the documentation it adapts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<String>>,
    /// `true` for a chapter of pages a reader looks things up in rather
    /// than reads through — the reference tables, the errors, the
    /// glossary.
    #[serde(default, skip_serializing_if = "is_false")]
    pub appendix: bool,
}

/// `[glossary]` — the page a documentation defines its own terms on
/// (PROP-057 `##GLOSSARY-DECLARED`).
///
/// One field, and the reason it exists at all is that the path used to be
/// a CONVENTION: the style linter looked for `glossary/index.xml` and
/// nothing in the manifest said so. A convention is invisible to every
/// other documentation and to every tool that did not read that one
/// constant, so the glossary became a thing a package declares — and
/// everything done with terms, from the style checks to the reader's
/// cards, is done with the declared glossary or not at all
/// (`##GLOSSARY-CARD-DECISION`).
///
/// The page is spelled as a pin and a chapter row spell one: a document
/// path under the spec root, without the extension, because one document
/// is served as three projections.
///
/// ```
/// use vibe_core::manifest::GlossaryDecl;
///
/// let g: GlossaryDecl = toml::from_str("page = \"glossary/index\"\n").unwrap();
/// assert_eq!(g.page, "glossary/index");
/// // A neighbour's field is not this table's, and is refused as one.
/// assert!(toml::from_str::<GlossaryDecl>("page = \"g/i\"\ntitle = \"Terms\"\n").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlossaryDecl {
    /// The document path of the glossary page.
    pub page: String,
}

/// `[[navigation.section]]` — one folder of the page tree under the name
/// a reader sees.
///
/// The id is the folder as the page paths spell it, never the title, so
/// an adaptation names the same sections as its source and shows other
/// words for them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationSectionDecl {
    /// The first path segment of the pages it holds — `start`.
    pub id: String,
    /// What the navigation shows, in the package's own language.
    pub title: String,
}

/// `true` for a well-formed pinned document path: relative, spelled with
/// forward slashes, no extension, no `.` or `..` segment, no empty
/// segment.
///
/// A form check and not an existence check, for the same reason
/// `[media]` refuses only a shape that could point outside the package:
/// whether the page is there is a question about a tree, and the gate is
/// where a tree is read (PROP-057 `##NAV-PINNED`).
pub(crate) fn pinned_path_form_is_valid(value: &str) -> bool {
    if value.is_empty() || value.starts_with('/') || value.contains('\\') {
        return false;
    }
    if value.chars().any(char::is_whitespace) {
        return false;
    }
    let mut segments = value.split('/').peekable();
    let mut any = false;
    while let Some(segment) = segments.next() {
        if segment.is_empty() || segment == "." || segment == ".." {
            return false;
        }
        // The extension is the pipeline's business: a pinned path names
        // a DOCUMENT, and the same document is served as three
        // projections under three extensions.
        if segments.peek().is_none() && segment.contains('.') {
            return false;
        }
        any = true;
    }
    any
}

/// `true` for a chapter id: any name that is not blank.
///
/// Weaker than a section's on purpose. A section id IS a folder of the
/// page tree and has to spell one; a chapter is a unit of a lesson plan
/// that has no folder behind it — a chapter may gather pages from four
/// folders, and two chapters may draw on one. What the id must do is be
/// there, so a translation has something to name the chapter by
/// (PROP-057 `##NAV-CHAPTERS-TRANSLATION`).
pub(crate) fn chapter_id_form_is_valid(value: &str) -> bool {
    !value.trim().is_empty()
}

/// `true` for a section id: exactly one path segment, which is what a
/// top-level folder of the page tree is.
pub(crate) fn section_id_form_is_valid(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('/')
        && !value.contains('\\')
        && !value.chars().any(char::is_whitespace)
        && value != "."
        && value != ".."
}

/// The upper bound on `abstract`, in characters (PROP-057
/// `##CARD-DESCRIPTION-AND-ABSTRACT`: «bounded at about a thousand
/// characters»). Counted in `char`s, not bytes, so a Russian adaptation
/// is not silently allowed half the room an English source gets.
pub const ABSTRACT_LIMIT: usize = 1000;

/// `true` for a well-formed subject coordinate: `<group>/<name>`, with
/// no version, no `kind:` prefix and no whitespace.
///
/// Deliberately a form check and not a resolution: the subject of the
/// core documentation is the host's own project coordinate, which no
/// registry can hand back (PROP-057 `##REL-HOST-SUBJECT`).
pub(crate) fn coordinate_form_is_valid(value: &str) -> bool {
    if value.chars().any(char::is_whitespace) || value.contains(':') || value.contains('@') {
        return false;
    }
    value
        .split_once('/')
        .is_some_and(|(group, name)| !group.is_empty() && !name.is_empty() && !name.contains('/'))
}

/// `true` when `value` parses as a semver constraint — the same parser
/// a dependency's version constraint goes through.
pub(crate) fn version_constraint_is_valid(value: &str) -> bool {
    !value.trim().is_empty() && semver::VersionReq::parse(value.trim()).is_ok()
}

/// `true` when `path` stays inside the package: relative, no root, no
/// `..` component, and not empty.
pub(crate) fn media_path_is_inside_package(path: &Path) -> bool {
    use std::path::Component;
    if path.as_os_str().is_empty() || path.is_absolute() {
        return false;
    }
    path.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        && path.components().any(|c| matches!(c, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_form_accepts_group_and_name_only() {
        assert!(coordinate_form_is_valid("org.vibevm.core/vibevm"));
        assert!(coordinate_form_is_valid(
            "org.vibevm.world/multi-user-planning"
        ));
        // A versioned or kind-prefixed pkgref is a different grammar and
        // is refused here so the edge stays a coordinate.
        assert!(!coordinate_form_is_valid("org.vibevm.core/vibevm@1.0.0"));
        assert!(!coordinate_form_is_valid("doc:org.vibevm.core/vibevm"));
        assert!(!coordinate_form_is_valid("vibevm"));
        assert!(!coordinate_form_is_valid("org.vibevm.core/"));
        assert!(!coordinate_form_is_valid("/vibevm"));
        assert!(!coordinate_form_is_valid("org.vibevm.core/a/b"));
        assert!(!coordinate_form_is_valid("org.vibevm core/vibevm"));
    }

    #[test]
    fn version_constraint_takes_what_a_dependency_takes() {
        for good in ["^1.0", "=0.3.0", ">=1, <2", "0.1.0"] {
            assert!(
                version_constraint_is_valid(good),
                "`{good}` is a constraint"
            );
        }
        for bad in ["", "   ", "latest", "^^1"] {
            assert!(!version_constraint_is_valid(bad), "`{bad}` is not");
        }
    }

    #[test]
    fn media_paths_may_not_leave_the_package() {
        assert!(media_path_is_inside_package(Path::new("media/icon.png")));
        assert!(media_path_is_inside_package(Path::new("./media/icon.png")));
        assert!(!media_path_is_inside_package(Path::new("")));
        assert!(!media_path_is_inside_package(Path::new(
            "../other/icon.png"
        )));
        assert!(!media_path_is_inside_package(Path::new("/etc/icon.png")));
        assert!(!media_path_is_inside_package(Path::new("./")));
    }

    #[test]
    fn declared_lists_only_the_images_that_are_there() {
        let media = MediaDecl {
            icon: Some(PathBuf::from("media/icon.png")),
            banner: None,
            preview: Some(PathBuf::from("media/preview.png")),
        };
        let names: Vec<&str> = media.declared().iter().map(|(field, _)| *field).collect();
        assert_eq!(names, vec!["icon", "preview"]);
        assert!(!media.is_empty());
        assert!(MediaDecl::default().is_empty());
    }
}
