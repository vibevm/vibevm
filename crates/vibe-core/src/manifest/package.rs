//! Package-role sections of a `vibe.toml` manifest.
//!
//! A node whose `vibe.toml` carries a `[package]` table is a **publishable
//! artifact**. The types in this module are the building blocks of that role —
//! identity (`[package]`), declared writes, the capability vocabulary
//! (`[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` /
//! `[conflicts]`), `[features]`, and conditional dependencies. They are
//! assembled into the unified [`Manifest`](super::Manifest) document; this
//! module owns no file I/O.
//!
//! Schema: `VIBEVM-SPEC.md` §7.3. The capability-based dependency vocabulary
//! (`[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` /
//! `[conflicts]`) is defined in [PROP-002 §2.9](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#capability).
//!
//! `[requires.packages]` is a TOML table — each key a bare `<kind>:<name>`
//! pkgref, each value either a version-constraint string (registry-resolved)
//! or an inline-table (registry-resolved with options, or a git-source
//! declaration per PROP-002 §2.4.1). There is no legacy array form. An
//! inline-table value may also carry a `link` field — the dependency's
//! inclusion type (PROP-009 §2.4).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#git-source");

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specmark::spec;

use crate::error::Result;
use crate::package_ref::{Group, PackageKind, PackageRef, VersionSpec};

use super::purl::Purl;

mod binary;
mod deps;
mod features;
mod hooks;
mod mcp_server;
mod skill;
mod weak_deps;
mod when;
mod wire;

pub use binary::BinaryDecl;
pub use deps::{GitPackageDep, GitRefKind, PathPackageDep, VarRegistryDep};
pub use features::FeaturesTable;
pub use hooks::HooksDecl;
pub use mcp_server::{MCP_ARG_VARS, McpServerDecl};
pub use skill::SkillDecl;
pub use weak_deps::{Recommends, Suggests};
pub use when::WhenCondition;

/// `[package]` — the identity of a publishable artifact.
///
/// A `vibe.toml` carrying this table is a package; one carrying `[project]`
/// is a plain consumer. The two are mutually exclusive — see
/// [`Manifest::validate`](super::Manifest::validate).
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
/// assert!(p.materialization.is_default()); // defaults to `snapshot`
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
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
    /// `snapshot` (the vendored full copy); `hardlink` shares unchanged
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
}

/// `skip_serializing_if` helper for boolean fields that default to `false`.
fn is_false(b: &bool) -> bool {
    !*b
}

impl PackageMeta {
    /// Produce a `PackageRef` pinning this package to its exact version.
    pub fn as_package_ref(&self) -> Result<PackageRef> {
        // Structural `=` pin — `VersionReq::parse("={version}")` rejects
        // build metadata, which never participates in pinning anyway.
        let req = semver::VersionReq {
            comparators: vec![semver::Comparator {
                op: semver::Op::Exact,
                major: self.version.major,
                minor: Some(self.version.minor),
                patch: Some(self.version.patch),
                pre: self.version.pre.clone(),
            }],
        };
        PackageRef::new(
            Some(self.kind),
            Some(self.group.clone()),
            self.name.clone(),
            VersionSpec::Req(req),
        )
    }
}

/// `[package].publish` — whether and where `vibe workspace publish` ships a
/// node. PROP-007 §2.7. Cargo's `publish` shape: a bool or a registry list.
///
/// ```
/// use vibe_core::manifest::PublishPosture;
///
/// let everywhere = PublishPosture::default(); // `publish = true`
/// assert!(everywhere.is_default());
/// assert!(everywhere.includes("vibespecs"));
///
/// let only = PublishPosture::Registries(vec!["vibespecs".into()]);
/// assert!(only.includes("vibespecs"));
/// assert!(!only.includes("other"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PublishPosture {
    /// `publish = true` — published into every configured registry.
    /// `publish = false` — never published (workspace-internal).
    All(bool),
    /// `publish = ["vibespecs", ...]` — published only into these named
    /// registries.
    Registries(Vec<String>),
}

impl Default for PublishPosture {
    fn default() -> Self {
        PublishPosture::All(true)
    }
}

impl PublishPosture {
    /// `true` for the default posture (`publish = true`) — lets the
    /// serializer skip the field on unchanged manifests.
    pub fn is_default(&self) -> bool {
        matches!(self, PublishPosture::All(true))
    }

    /// `true` iff this node is never published (`publish = false`).
    pub fn is_never(&self) -> bool {
        matches!(self, PublishPosture::All(false))
    }

    /// `true` iff this node should be published into the registry with the
    /// given local name.
    pub fn includes(&self, registry_name: &str) -> bool {
        match self {
            PublishPosture::All(all) => *all,
            PublishPosture::Registries(names) => names.iter().any(|n| n == registry_name),
        }
    }
}

/// `[package].materialization` — how a package's content is placed into its
/// `vibedeps/` slot (PROP-022 §2.1). Three modes along two axes of "big":
/// `hardlink` shares bytes for few-but-large files; `in-place` avoids any
/// per-file tree walk for repos with millions of files.
///
/// ```
/// use vibe_core::manifest::Materialization;
///
/// // Default is the vendored full copy.
/// assert_eq!(Materialization::default(), Materialization::Snapshot);
/// assert!(Materialization::default().is_default());
///
/// // The wire form is kebab-case (`in-place`).
/// let m: Materialization = toml::from_str(r#"m = "in-place""#)
///     .map(|t: toml::value::Table| t["m"].clone().try_into().unwrap())
///     .unwrap();
/// assert_eq!(m, Materialization::InPlace);
/// assert!(!m.is_default());
///
/// // `in-place` identity is the git commit, not a content hash (PROP-022 §2.5).
/// assert!(Materialization::InPlace.is_in_place());
/// assert!(!Materialization::Snapshot.is_in_place());
/// ```
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-022#modes",
    r = 1
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Materialization {
    /// The default: live-git cache → `.git`-stripped snapshot → full copy
    /// into the slot. Identified by `content_hash`, vendored into the
    /// project's git (PROP-022 §2.2).
    #[default]
    Snapshot,
    /// Per-file hardlink from the cached snapshot, copy on change, copy
    /// fallback on cross-volume / unsupported filesystems (PROP-022 §2.3).
    /// For packages big in bytes but modest in file count.
    Hardlink,
    /// Git-native, project-local clone landed directly in the slot, managed
    /// in place by git; identity is `resolved_commit`, not `content_hash`
    /// (PROP-022 §2.4/§2.5). For repos with millions of files.
    InPlace,
}

impl Materialization {
    /// `true` for the default mode (`snapshot`) — lets the serializer skip
    /// the field on a manifest that does not set it.
    pub fn is_default(&self) -> bool {
        matches!(self, Materialization::Snapshot)
    }

    /// `true` iff this mode is `in-place` — the git-managed, commit-identified
    /// mode whose slot is not vendored and whose destructive operations are
    /// guarded (PROP-022 §2.4/§2.6/§2.7).
    pub fn is_in_place(&self) -> bool {
        matches!(self, Materialization::InPlace)
    }
}

/// `[compatibility]` — optional gates on the consuming toolchain: a minimum
/// `vibe` version and the package kinds this one needs present.
///
/// ```
/// use vibe_core::manifest::Compatibility;
///
/// let c: Compatibility = toml::from_str(r#"
///     min_vibe_version = "0.2"
///     requires_kinds = ["stack"]
/// "#).unwrap();
/// assert_eq!(c.min_vibe_version.as_deref(), Some("0.2"));
/// assert!(!c.is_empty());
/// assert!(Compatibility::default().is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Compatibility {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_vibe_version: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires_kinds: Vec<PackageKind>,
}

impl Compatibility {
    pub fn is_empty(&self) -> bool {
        self.min_vibe_version.is_none() && self.requires_kinds.is_empty()
    }
}

/// Inclusion type for a dependency's boot contribution — the point on the
/// static/dynamic-linking spectrum at which `vibe` resolves it (PROP-009
/// §2.4).
///
/// Set by the consumer on a `[requires.packages]` entry (`link = "…"`); a
/// package may suggest a default on its own `[boot_snippet]`; a workspace
/// may set a fallback in `[boot].default_link`. Absent everywhere, the
/// type is [`LinkType::Dynamic`].
///
/// ```
/// use vibe_core::manifest::LinkType;
///
/// // Absent everywhere, a dependency links dynamically (PROP-009 §2.4); a
/// // consumer overrides it per-dep with `link = "static"`.
/// assert_eq!(LinkType::default(), LinkType::Dynamic);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    /// Compiled verbatim into the generated `STATIC.md`, read first — the
    /// statically-linked priority lane (AOT). Duplicates the text on disk,
    /// so used deliberately, for critical disciplines and top-level skills.
    Static,
    /// The default. `vibe` resolves the contribution to a concrete path in
    /// the generated `INDEX.md`; the agent reads it **dynamically, on
    /// demand**. An optional `when` condition (PROP-009 §2.6) gates that
    /// read — context-gated / conditional loading.
    #[default]
    Dynamic,
    /// This package **and its entire transitive closure** are pulled
    /// `static` (PROP-035 §12 / PROP-034 §2.1). A consumer-side property of
    /// the edge — the same package can be pulled `static-transitive` by one
    /// consumer and `dynamic` by another. Resolved to `static` at emission,
    /// with the mode propagated across the closure by `bootgen`.
    #[serde(rename = "static-transitive")]
    StaticTransitive,
}

/// Ordering band for a package's boot snippet within the computed boot
/// sequence (PROP-009 §2.5). Replaces the author-chosen `NN-` numeric
/// prefix, which cannot survive a workspace's combined namespace.
///
/// `vibe` composes the sequence `foundation` → the node's own boot →
/// dependency boot → `user-override`.
///
/// ```
/// use vibe_core::manifest::BootCategory;
///
/// // The categories a boot snippet can declare; the wire form is the
/// // kebab-case name (e.g. `category = "user-override"`), shown on
/// // `BootSnippet` below. `flow` / `stack` / `tool` / `app` are package
/// // contributions and sort into the dependency band.
/// let categories = [
///     BootCategory::Foundation,
///     BootCategory::Flow,
///     BootCategory::Stack,
///     BootCategory::Tool,
///     BootCategory::App,
///     BootCategory::UserOverride,
/// ];
/// assert_eq!(categories.len(), 6);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BootCategory {
    /// Project-wide foundation — conventions, the four rules, technology
    /// choices. Composed first.
    Foundation,
    /// A `flow` package's discipline contribution.
    Flow,
    /// A `stack` package's technology contribution.
    Stack,
    /// A `tool` package's contribution — an installed tool's boot snippet.
    Tool,
    /// An `app` package's contribution.
    App,
    /// User-owned overrides — composed last, so they win.
    UserOverride,
}

/// An operating system a [`WhenCondition::Os`] gate can name. The values
/// match Rust's `std::env::consts::OS`.
///
/// ```
/// use vibe_core::manifest::TargetOs;
///
/// assert_eq!(TargetOs::Linux.as_str(), "linux");
/// assert_eq!(TargetOs::Windows.to_string(), "windows");
/// // `current()` maps the host to a variant, or `None` off the set.
/// let _ = TargetOs::current();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetOs {
    /// Microsoft Windows — `std::env::consts::OS == "windows"`.
    Windows,
    /// Apple macOS — `std::env::consts::OS == "macos"`.
    Macos,
    /// Linux — `std::env::consts::OS == "linux"`.
    Linux,
}

impl TargetOs {
    /// The canonical lowercase name — a `std::env::consts::OS` value.
    pub fn as_str(self) -> &'static str {
        match self {
            TargetOs::Windows => "windows",
            TargetOs::Macos => "macos",
            TargetOs::Linux => "linux",
        }
    }

    /// The operating system this process runs on, when vibevm recognises
    /// it — `None` on a platform outside the supported set.
    pub fn current() -> Option<Self> {
        match std::env::consts::OS {
            "windows" => Some(TargetOs::Windows),
            "macos" => Some(TargetOs::Macos),
            "linux" => Some(TargetOs::Linux),
            _ => None,
        }
    }
}

impl std::fmt::Display for TargetOs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `[boot_snippet]` — the boot contribution a package ships (package-role).
///
/// ```
/// use vibe_core::manifest::{BootSnippet, BootCategory, LinkType};
///
/// let b: BootSnippet = toml::from_str(r#"
///     source = "boot/10-flow-wal.md"
///     category = "flow"
///     link = "dynamic"
/// "#).unwrap();
/// assert_eq!(b.source.to_str(), Some("boot/10-flow-wal.md"));
/// assert_eq!(b.category, Some(BootCategory::Flow));
/// assert_eq!(b.link, Some(LinkType::Dynamic));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootSnippet {
    /// Path to the source file inside the package directory, e.g.
    /// `boot/10-flow-wal.md`.
    pub source: PathBuf,
    /// Ordering category for the computed boot sequence (PROP-009 §2.5).
    /// Absent on pre-PROP-009 manifests; the computed-view engine derives
    /// a fallback from the package kind when this is `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<BootCategory>,
    /// Suggested default inclusion type (PROP-009 §2.4). Only a hint — the
    /// consumer's own `link` declaration always wins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<LinkType>,
    /// Activation condition (PROP-009 §2.4 / §2.6). When set, the snippet
    /// is a conditional contribution: the computed-view engine renders it
    /// as a `dynamic` `INDEX.md` entry — regardless of `link` — carrying
    /// this condition, and the agent reads the file at boot only when it
    /// holds. For v1 the only condition is an OS match (`when = "os:linux"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<WhenCondition>,
}

mod capabilities;
pub use capabilities::{
    ConditionalTarget, ConflictsList, Obsoletes, Provides, Requires, RequiresAny,
};

#[cfg(test)]
#[path = "package/tests.rs"]
mod tests;
