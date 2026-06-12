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

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::capability_ref::CapabilityRef;
use crate::error::Result;
use crate::package_ref::{Group, PackageKind, PackageRef, VersionSpec};

use super::purl::Purl;

mod deps;
mod features;
mod when;
mod wire;

pub use deps::{GitPackageDep, GitRefKind, PathPackageDep, VarRegistryDep};
pub use features::FeaturesTable;
pub use when::WhenCondition;

use wire::RequiresWire;

/// `[package]` — the identity of a publishable artifact.
///
/// A `vibe.toml` carrying this table is a package; one carrying `[project]`
/// is a plain consumer. The two are mutually exclusive — see
/// [`Manifest::validate`](super::Manifest::validate).
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
/// type is [`LinkType::Static`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    /// Concatenated verbatim into the generated `INLINE.md`, read first —
    /// the emergency priority lane. Duplicates the text on disk, so used
    /// sparingly, for critical disciplines and top-level skills.
    Inline,
    /// The default. `vibe` resolves the contribution to a concrete path in
    /// the generated `INDEX.md`; the agent reads it directly.
    #[default]
    Static,
    /// `INDEX.md` carries an INCLUDE pointer the agent resolves at boot —
    /// supports conditional, context-gated loading.
    Dynamic,
}

/// Ordering band for a package's boot snippet within the computed boot
/// sequence (PROP-009 §2.5). Replaces the author-chosen `NN-` numeric
/// prefix, which cannot survive a workspace's combined namespace.
///
/// `vibe` composes the sequence `foundation` → the node's own boot →
/// dependency boot → `user-override`.
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
    /// User-owned overrides — composed last, so they win.
    UserOverride,
}

/// An operating system a [`WhenCondition::Os`] gate can name. The values
/// match Rust's `std::env::consts::OS`.
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

/// `[provides]` — capabilities this package advertises.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provides {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<CapabilityRef>,
}

impl Provides {
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
}

/// `[requires]` — concrete package pkgrefs plus capability requirements.
///
/// Wire form: `[requires.packages]` is a TOML table — each key a bare pkgref
/// (`<kind>:<name>` without `@version`), each value either:
///
/// - a constraint string (`"^0.3"`, `"=1.0"`, `"*"`) — registry-resolved, or
/// - an inline-table — registry-resolved with options (`{ version = "..." }`)
///   **or** a git-source dependency (`{ git = "...", tag = "..." }` etc.,
///   per PROP-002 §2.4.1).
///
/// `capabilities` carries abstract requirements satisfied by any provider.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "RequiresWire", try_from = "RequiresWire")]
pub struct Requires {
    /// Registry-resolved package dependencies.
    pub packages: Vec<PackageRef>,
    /// Abstract capability requirements (RPM-family `Requires:` semantics).
    pub capabilities: Vec<CapabilityRef>,
    /// Git-source package dependencies — one git repository = one package
    /// (PROP-002 §2.4.1). Stored separately from `packages` so code that
    /// iterates registry-resolved roots stays untouched; resolver and CLI
    /// code consult both when they need the full root set.
    pub git_packages: Vec<GitPackageDep>,
    /// Path-source package dependencies — a package living in a local
    /// directory, typically a sibling workspace member (PROP-007 §2.5).
    /// Its own bucket, for the same reason as `git_packages`.
    pub path_packages: Vec<PathPackageDep>,
    /// Registry-resolved dependencies whose version is a `[workspace.versions]`
    /// placeholder — `{ version.var = "core" }`. Unresolved at parse time;
    /// `vibe-workspace`'s loader resolves each against the recursive
    /// placeholder chain into a concrete `PackageRef` in `packages`. Empty
    /// once a workspace has finalised the manifest. PROP-007 §2.6.
    pub var_packages: Vec<VarRegistryDep>,
    /// Per-dependency inclusion type the consumer declared (PROP-009 §2.4),
    /// keyed by the `<group>/<name>` identity. Every declared `link` is
    /// stored, **including an explicit `static`** — so a consumer's explicit
    /// choice can be told apart from an absent one. The distinction is
    /// load-bearing: an explicit `link` overrides a workspace
    /// `[boot].default_link` and a package-suggested link, while an absent
    /// one yields to them. The key is version-independent, so it survives
    /// the `var_packages` → `packages` resolution the workspace loader
    /// performs. Read it through [`Requires::link_for`] (resolved, with the
    /// default applied) or [`Requires::declared_link`] (raw — distinguishes
    /// an absent declaration from an explicit one).
    pub links: BTreeMap<String, LinkType>,
}

impl Requires {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
            && self.capabilities.is_empty()
            && self.git_packages.is_empty()
            && self.path_packages.is_empty()
            && self.var_packages.is_empty()
            && self.links.is_empty()
    }

    /// Return every root dependency's `(group, name)` identity in a single
    /// iterator. Order: `packages`, `git_packages`, `path_packages`,
    /// `var_packages`. The group is `Option` only because a `packages`
    /// entry's `PackageRef` carries an optional group — a well-formed
    /// `[requires]` always qualifies it.
    pub fn iter_pkgrefs(&self) -> impl Iterator<Item = (Option<&Group>, &str)> {
        self.packages
            .iter()
            .map(|p| (p.group.as_ref(), p.name.as_str()))
            .chain(
                self.git_packages
                    .iter()
                    .map(|g| (Some(&g.group), g.name.as_str())),
            )
            .chain(
                self.path_packages
                    .iter()
                    .map(|p| (Some(&p.group), p.name.as_str())),
            )
            .chain(
                self.var_packages
                    .iter()
                    .map(|v| (Some(&v.group), v.name.as_str())),
            )
    }

    /// The inclusion type (PROP-009 §2.4) in effect for the `<group>/<name>`
    /// dependency in this `[requires]`, with the contract default applied —
    /// an absent declaration resolves to [`LinkType::Static`].
    pub fn link_for(&self, group: &Group, name: &str) -> LinkType {
        self.declared_link(group, name).unwrap_or_default()
    }

    /// The inclusion type the consumer **explicitly declared** for
    /// `<group>/<name>`, or `None` if it declared none. Unlike
    /// [`Requires::link_for`], an explicit `link = "static"` returns
    /// `Some(LinkType::Static)`, not `None`: the loading-model precedence
    /// (PROP-009 §2.4) lets an explicit declaration override a workspace
    /// `[boot].default_link` or a package-suggested link, and that
    /// distinction is lost if explicit `static` is folded into "absent".
    pub fn declared_link(&self, group: &Group, name: &str) -> Option<LinkType> {
        self.links.get(&link_key(group, name)).copied()
    }
}

/// The `Requires::links` map key — the kind-free `<group>/<name>` identity
/// (PROP-008 §2.2). Version-independent, so it survives the
/// `var_packages` → `packages` resolution the workspace loader performs.
fn link_key(group: &Group, name: &str) -> String {
    format!("{group}/{name}")
}

/// `[[requires_any]]` — one entry per independent disjunction; `one_of` must
/// be satisfied by at least one of its alternatives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiresAny {
    pub one_of: Vec<PackageRef>,
}

/// `[obsoletes]` — packages this one supersedes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Obsoletes {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<PackageRef>,
}

impl Obsoletes {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

/// `[conflicts]` — packages that cannot coexist with this one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictsList {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<PackageRef>,
}

impl ConflictsList {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

/// `[target."<predicate>"]` body — currently just `[dependencies]`,
/// shaped like `[requires]`. PROP-003 §2.6.1.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionalTarget {
    #[serde(default, skip_serializing_if = "Requires::is_empty")]
    pub dependencies: Requires,
}

#[cfg(test)]
#[path = "package/tests.rs"]
mod tests;
