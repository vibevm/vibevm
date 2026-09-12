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
