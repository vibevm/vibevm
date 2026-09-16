//! Package identity metadata.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#git-source");

use super::super::purl::Purl;
use super::{Authorship, Materialization, PackageFormat, PublishPosture, is_false};
use crate::manifest::SpecFormat;
use crate::package_ref::{Group, PackageKind};
use serde::{Deserialize, Serialize};

/// `[package]` — the identity of a publishable artifact.
///
/// A `vibe.toml` carrying this table is a package; one carrying `[project]`
/// is a plain consumer. The two are mutually exclusive — see
/// [`Manifest::validate`](crate::manifest::Manifest::validate).
///
/// ```
/// use vibe_core::manifest::PackageMeta;
/// use vibe_core::PackageKind;
///
/// let p: PackageMeta = toml::from_str(r#"
///     name = "wal"
///     group = "org.vibevm"
///     kind = "feat"
///     version = "0.1.0"
/// "#).unwrap();
/// assert_eq!(p.name, "wal");
/// assert_eq!(p.kind, PackageKind::Feat);
/// assert!(p.publish.is_default()); // `publish` defaults to true
/// assert!(p.materialization.is_default()); // defaults to `copy`
/// assert!(!p.bridge); // not a bridge by default
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageMeta {
    pub name: String,
    /// Reverse-FQDN namespace qualifier (PROP-008 §2.1) — mandatory. With
    /// `name` it forms the package's identity; `name` is unique within a
    /// `group`. `kind` is metadata, not part of identity (PROP-008 §2.2).
    pub group: Group,
    pub kind: PackageKind,
    pub version: semver::Version,
    /// `[package].epoch` — the manifest-format epoch this file is authored
    /// against (PROP-044 §6.2). **Absence is not `epoch = 1`**: a manifest
    /// with no `epoch` is in the distinct *pre-epoch* state and is read for
    /// all time by the frozen pre-epoch reader — today's parse — never by a
    /// later epoch's reader that merely assumes the first. New manifests are
    /// authored with `epoch = 1` by `vibe init`; existing ones stay as they
    /// are until a codemod wave rewrites them, and `vibe check` reports the
    /// absence as info so the pre-epoch population stays countable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epoch: Option<u32>,
    /// `[package].spec_format` — the PROP-045 materialisation setting for a
    /// package-rooted consumer node: a dev checkout whose ROOT manifest is
    /// this package pins its dependency materialisation exactly as a
    /// `[project]` does (the role-equipotence law, PROP-024
    /// ##MANIFEST-ROLES-ARE-EQUIPOTENT).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_format: Option<SpecFormat>,
    /// `[package].frozen` — the PROP-044 §2a immutability flag. Absence
    /// (`false`) means the version is a **snapshot**: its content may
    /// still flow under the same version string, and a hash mismatch is
    /// NEWS, not alarm. `true` means **frozen**: the bytes are immutable,
    /// and a hash mismatch is ALARM. The transition is one-way —
    /// unfreezing is forbidden; further work on the package is a new
    /// version line. The flag lives INSIDE the hashed content, so the
    /// version is self-describing even offline and every registry
    /// serving these bytes agrees with it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub frozen: bool,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// `[package].title` — the human-readable display name: what the
    /// site's shelves, the language selector, the page heading and the
    /// catalogue show. Uniqueness is not checked and identity stays the
    /// coordinate; the publisher is shown beside it, so two `VibeVM
    /// Manual`s never become one thing (PROP-057 `##CARD-TITLE`).
    /// REQUIRED for `kind = "doc"`, optional everywhere else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// `[package].abstract` — the four answers a reader needs before
    /// opening anything: what it covers, for whom, what it assumes
    /// known, what it leaves out. `description` stays the one-line
    /// subtitle beside it; this is the paragraph, bounded at
    /// [`ABSTRACT_LIMIT`] characters, and the entry page inserts it
    /// rather than restating it (PROP-057
    /// `##CARD-DESCRIPTION-AND-ABSTRACT`). REQUIRED for `kind = "doc"`.
    #[serde(default, rename = "abstract", skip_serializing_if = "Option::is_none")]
    pub abstract_text: Option<String>,
    /// `[package].authorship` — who wrote the prose this documentation
    /// carries: `human`, `ai` or `mixed` (PROP-057 `##CARD-AUTHORSHIP`).
    /// A field of documentation packages, so `validate` refuses it in
    /// every other kind; absent means unknown, and a reader filtering a
    /// shelf by authorship sees such a package in neither named group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorship: Option<Authorship>,
    /// `[package].lang` — **refused, and read only so the refusal can
    /// say why.**
    ///
    /// A documentation package does have a language, and an author will
    /// reach for this key: the campaign plan that preceded PROP-057
    /// named it. But the manifest already had a field for «what
    /// language is this written in» — `[i18n].canonical` of PROP-003 —
    /// and two fields for one fact is how they drift. So the key parses
    /// and `validate` refuses it by name, pointing at the one that
    /// works (PROP-057 `##LOC-LANGUAGE-FIELD`). It is never serialised:
    /// no valid manifest carries it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    /// PURL of the upstream library this package documents
    /// (PROP-003 §2.5.6). Optional; when set, ties the package's
    /// version to a specific upstream artefact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub describes: Option<Purl>,
    /// Publish posture — whether `vibe workspace publish` ships this node,
    /// and to which registries. PROP-007 §2.7. Default `true` (published
    /// into every configured registry).
    #[serde(default, skip_serializing_if = "PublishPosture::is_default")]
    pub publish: PublishPosture,
    /// How this package is materialised on disk (PROP-022 §2.1). Default
    /// `copy` (the vendored full copy); `hardlink` shares unchanged
    /// files by link; `in-place` is a git-native, project-local clone for
    /// giant repos. Skipped from the serialized form when default.
    #[serde(default, skip_serializing_if = "Materialization::is_default")]
    pub materialization: Materialization,
    /// `[package].bridge` — `true` marks this package as a bridge: a wrapper
    /// a maintainer publishes around someone else's repository (PROP-023
    /// §2.1). It does **not** change `kind` or identity; it is metadata that
    /// records the content as stewarded-not-authored and surfaces provenance.
    /// Default `false`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub bridge: bool,
    /// `[package].format` — the PROP-035 §3 package format. `simple` (the
    /// default) is carried whole, directive-free; `normal` opts into the
    /// contract/source split and the spec compiler, so a `static`-linked
    /// `normal` package is compiled to its `#use`-reachable, `#source`-merged
    /// closure (PROP-035 §7/§8) rather than concatenated verbatim. Default
    /// `simple` — a forgotten `format` fails safe (over-load, visibly working),
    /// never silent (PROP-035 §3, owner decision).
    #[serde(default, skip_serializing_if = "PackageFormat::is_default")]
    pub format: PackageFormat,
}
