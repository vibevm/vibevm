//! `vibe.toml` — the unified manifest document.
//!
//! Schema: `VIBEVM-SPEC.md` §7, [PROP-007](../../../spec/modules/vibe-workspace/PROP-007-workspace.md).
//!
//! # One file, variable roles
//!
//! Every node in a vibevm project — a plain consumer project, a workspace
//! member, a published package, a workspace coordinator — carries a single
//! `vibe.toml`. The node's role is expressed by *which sections are present*,
//! the cargo model where one `Cargo.toml` carries `[package]` and/or
//! `[workspace]`:
//!
//! - `[project]` — a non-publishable consumer node.
//! - `[package]` — a publishable artifact.
//! - `[workspace]` — coordinates member packages.
//! - `[origin]` — provenance marker, written by `vibe workspace publish` into
//!   the published copy of a workspace member.
//!
//! `[project]` and `[package]` are mutually exclusive. `[workspace]` composes
//! with `[project]`, with `[package]` (a cargo-style root package), or with
//! neither (a virtual workspace root — just a coordinator). Consumer-side
//! configuration (`[requires]`, `[[registry]]`, `[[mirror]]`, `[[override]]`,
//! `[active]`, `[llm]`, `[i18n]`, `[boot]`) may appear on any node. Package-role
//! sections (`[writes]`, `[provides]`, `[[requires_any]]`, `[obsoletes]`,
//! `[conflicts]`, `[compatibility]`, `[boot_snippet]`, `[features]`,
//! `[target.*]`) are meaningful only alongside `[package]`.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-007#unified-manifest");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specmark::spec;

use crate::error::{Error, Result};
use crate::package_ref::PackageRef;

use super::i18n::I18nDecl;
use super::package::{
    BootSnippet, Compatibility, ConditionalTarget, ConflictsList, FeaturesTable, LinkType,
    Obsoletes, PackageMeta, Provides, Requires, RequiresAny,
};
use super::project::{
    ActiveSection, LlmSection, MirrorSection, OverrideSection, ProjectSection, RegistrySection,
};
use super::{read_toml, write_toml};

/// The unified `vibe.toml` document. See the module docs for the role model.
///
/// ```
/// use vibe_core::manifest::Manifest;
///
/// let m = Manifest::parse_str(
///     "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
/// ).unwrap();
/// m.validate().unwrap(); // the project XOR package rule holds
/// assert_eq!(m.require_package().unwrap().name, "wal");
/// ```
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-007#unified-manifest",
    r = 1
)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// `[project]` — identity of a non-publishable consumer node. Mutually
    /// exclusive with `package`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectSection>,

    /// `[package]` — identity of a publishable artifact. Mutually exclusive
    /// with `project`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<PackageMeta>,

    /// `[workspace]` — declares member packages this node coordinates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceSection>,

    /// `[origin]` — provenance marker on a published workspace-member copy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<OriginSection>,

    /// `[requires]` — packages and capabilities this node depends on.
    #[serde(default, skip_serializing_if = "Requires::is_empty")]
    pub requires: Requires,

    /// `[[requires_any]]` — disjunctive requirements (package-role).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires_any: Vec<RequiresAny>,

    /// `[provides]` — capabilities this package advertises (package-role).
    #[serde(default, skip_serializing_if = "Provides::is_empty")]
    pub provides: Provides,

    /// `[obsoletes]` — packages this one supersedes (package-role).
    #[serde(default, skip_serializing_if = "Obsoletes::is_empty")]
    pub obsoletes: Obsoletes,

    /// `[conflicts]` — packages that cannot coexist with this one (package-role).
    #[serde(default, skip_serializing_if = "ConflictsList::is_empty")]
    pub conflicts: ConflictsList,

    /// `[compatibility]` — minimum vibe version, required kinds (package-role).
    #[serde(default, skip_serializing_if = "Compatibility::is_empty")]
    pub compatibility: Compatibility,

    /// `[boot_snippet]` — boot snippet this package contributes (package-role).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_snippet: Option<BootSnippet>,

    /// `[features]` — conditionally-activated components (package-role).
    #[serde(default, skip_serializing_if = "FeaturesTable::is_empty")]
    pub features: FeaturesTable,

    /// `[target."<predicate>"]` — conditional dependencies (package-role).
    #[serde(default, rename = "target", skip_serializing_if = "BTreeMap::is_empty")]
    pub conditional_deps: BTreeMap<String, ConditionalTarget>,

    /// `[active]` — the active stack for `vibe build`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<ActiveSection>,

    /// `[llm]` — LLM provider configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm: Option<LlmSection>,

    /// `[[registry]]` — priority-ordered registry list.
    #[serde(default, rename = "registry", skip_serializing_if = "Vec::is_empty")]
    pub registries: Vec<RegistrySection>,

    /// `[[mirror]]` — transparent fallback URLs per registry.
    #[serde(default, rename = "mirror", skip_serializing_if = "Vec::is_empty")]
    pub mirrors: Vec<MirrorSection>,

    /// `[[override]]` — pkgref pins that bypass the registry layer.
    #[serde(default, rename = "override", skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<OverrideSection>,

    /// `[i18n]` — project-level language preference (PROP-003 §2.7).
    #[serde(default, skip_serializing_if = "I18nDecl::is_default")]
    pub i18n: I18nDecl,

    /// `[boot]` — workspace-wide loading settings (PROP-009 §2.6).
    #[serde(default, skip_serializing_if = "BootSection::is_empty")]
    pub boot: BootSection,
}

/// `[workspace]` — declares the member packages a node coordinates.
///
/// PROP-007 §2.1. A `[workspace]` node owns the single `vibe.lock` at the
/// absolute root of the workspace tree; members bubble commands up to it.
/// Nested workspaces are permitted — a member may itself carry `[workspace]`.
///
/// ```
/// use vibe_core::manifest::WorkspaceSection;
///
/// let w: WorkspaceSection = toml::from_str(r#"
///     members = ["packages/flow-wal", "packages/stack-rust"]
/// "#).unwrap();
/// assert_eq!(w.members.len(), 2);
/// assert!(w.versions.is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceSection {
    /// Member directories, relative to this manifest. Glob patterns are
    /// permitted (`packages/*`). Each member is a directory carrying its
    /// own `vibe.toml`. Membership is explicit — there is no auto-discovery.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<String>,

    /// `[workspace.versions]` — named version-constraint placeholders
    /// (PROP-007 §2.6). A member references one with `version.var = "name"`
    /// in `[requires.packages]`; the placeholder is resolved bottom-up
    /// against the enclosing-workspace chain, nearest first. Maps a name to
    /// a constraint string such as `"0.0.1"` or `"^0.3"`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub versions: BTreeMap<String, String>,
}

/// `[origin]` — provenance marker written into a published workspace-member
/// copy by `vibe workspace publish`. PROP-007 §2.8.
///
/// Lets a consumer (and the registry explorer) trace a published package
/// repository back to the monorepo it was generated from.
///
/// ```
/// use vibe_core::manifest::OriginSection;
///
/// let o: OriginSection = toml::from_str(r#"
///     upstream = "https://github.com/me/monorepo"
///     path = "packages/flow-wal"
///     generated_by = "vibe 0.1.0"
///     generated_at = "2026-05-21T12:00:00Z"
/// "#).unwrap();
/// assert_eq!(o.path, "packages/flow-wal");
/// assert!(o.commit.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OriginSection {
    /// URL of the source monorepo this copy was generated from.
    pub upstream: String,
    /// Path of the package directory within that monorepo.
    pub path: String,
    /// Commit of the monorepo at generation time — present when the
    /// monorepo was a git repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// Tool identity that generated the copy — e.g. `vibe 0.1.0`.
    pub generated_by: String,
    /// ISO-8601 timestamp of generation.
    pub generated_at: String,
}

/// `[boot]` — workspace-wide loading settings (PROP-009 §2.6).
///
/// Consumer-side: may appear on any node, with or without a `[package]`
/// table. For v1 it carries only a default inclusion type — the fallback
/// `link` for dependencies that declare none of their own. Room to grow;
/// nothing further is defined yet.
///
/// ```
/// use vibe_core::manifest::{BootSection, LinkType};
///
/// let b: BootSection = toml::from_str(r#"default_link = "dynamic""#).unwrap();
/// assert_eq!(b.default_link, Some(LinkType::Dynamic));
/// assert!(BootSection::default().is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootSection {
    /// Default inclusion type for dependencies that declare no `link` of
    /// their own. Absent → the PROP-009 §2.4 default, [`LinkType::Static`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_link: Option<LinkType>,
}

impl BootSection {
    /// `true` when the table carries nothing — lets the serializer skip it
    /// on a manifest that sets no loading options.
    pub fn is_empty(&self) -> bool {
        self.default_link.is_none()
    }
}

impl Manifest {
    pub const FILENAME: &'static str = "vibe.toml";

    /// Read and validate a `vibe.toml` from disk.
    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        let m: Manifest = read_toml(path)?;
        m.validate()?;
        Ok(m)
    }

    /// Parse and validate a `vibe.toml` from an in-memory string. Used by
    /// callers that obtain manifest bytes without a filesystem path — e.g.
    /// the registry reading a manifest out of a fetched package tree.
    pub fn parse_str(text: &str) -> Result<Self> {
        let m: Manifest = toml::from_str(text).map_err(|source| Error::ParseToml {
            path: PathBuf::from(Self::FILENAME),
            source,
        })?;
        m.validate()?;
        Ok(m)
    }

    /// Write the manifest to disk, preserving operator comments where the
    /// existing file carries any.
    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        write_toml(path, self)
    }

    /// A bare consumer-project manifest — `[project]` only.
    pub fn new_project(name: impl Into<String>, version: impl Into<String>) -> Self {
        Manifest {
            project: Some(ProjectSection {
                name: name.into(),
                version: version.into(),
                authors: Vec::new(),
            }),
            ..Manifest::default()
        }
    }

    /// A bare publishable-package manifest — `[package]` only.
    pub fn new_package(meta: PackageMeta) -> Self {
        Manifest {
            package: Some(meta),
            ..Manifest::default()
        }
    }

    /// Enforce the role rules: `[project]` ⊕ `[package]`; at least one role
    /// section present; package-role sections require `[package]`.
    pub fn validate(&self) -> Result<()> {
        let has_project = self.project.is_some();
        let has_package = self.package.is_some();
        let has_workspace = self.workspace.is_some();

        if has_project && has_package {
            return Err(Error::InvalidManifest {
                reason: "[project] and [package] are mutually exclusive — a node is \
                         either a plain project or a publishable package, not both"
                    .to_string(),
            });
        }
        if !has_project && !has_package && !has_workspace {
            return Err(Error::InvalidManifest {
                reason: "manifest declares no role — it must carry [project], [package], \
                         or [workspace]"
                    .to_string(),
            });
        }

        if !has_package {
            let mut offenders: Vec<&str> = Vec::new();
            if self.boot_snippet.is_some() {
                offenders.push("[boot_snippet]");
            }
            if !self.provides.is_empty() {
                offenders.push("[provides]");
            }
            if !self.requires_any.is_empty() {
                offenders.push("[[requires_any]]");
            }
            if !self.obsoletes.is_empty() {
                offenders.push("[obsoletes]");
            }
            if !self.conflicts.is_empty() {
                offenders.push("[conflicts]");
            }
            if !self.compatibility.is_empty() {
                offenders.push("[compatibility]");
            }
            if !self.features.is_empty() {
                offenders.push("[features]");
            }
            if !self.conditional_deps.is_empty() {
                offenders.push("[target]");
            }
            if !offenders.is_empty() {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "package-role section(s) {} present without a [package] table",
                        offenders.join(", ")
                    ),
                });
            }
        }
        Ok(())
    }

    /// `true` iff this node is a publishable package.
    pub fn is_package(&self) -> bool {
        self.package.is_some()
    }

    /// `true` iff this node coordinates a workspace.
    pub fn is_workspace_root(&self) -> bool {
        self.workspace.is_some()
    }

    /// The `[package]` table, or a descriptive error if this manifest is not
    /// a publishable package. Use at call sites that require a package —
    /// the registry, the publisher.
    pub fn require_package(&self) -> Result<&PackageMeta> {
        self.package.as_ref().ok_or_else(|| Error::InvalidManifest {
            reason: "expected a [package] table — this manifest is not a publishable package"
                .to_string(),
        })
    }

    /// The `[project]` table, or a descriptive error if this manifest is not
    /// a plain project.
    pub fn require_project(&self) -> Result<&ProjectSection> {
        self.project.as_ref().ok_or_else(|| Error::InvalidManifest {
            reason: "expected a [project] table — this manifest is not a plain project".to_string(),
        })
    }

    /// A `PackageRef` pinning this package to its exact version. Errors if
    /// the manifest is not a package.
    pub fn as_package_ref(&self) -> Result<PackageRef> {
        self.require_package()?.as_package_ref()
    }

    /// The first configured registry, if any.
    pub fn primary_registry(&self) -> Option<&RegistrySection> {
        self.registries.first()
    }

    /// Registry with the given local name, if any.
    pub fn registry_by_name(&self, name: &str) -> Option<&RegistrySection> {
        self.registries.iter().find(|r| r.name == name)
    }

    /// Mirror entries targeting the given registry name, plus any `"*"`
    /// wildcards, sorted by priority ascending.
    pub fn mirrors_for<'a>(&'a self, registry_name: &str) -> Vec<&'a MirrorSection> {
        let mut v: Vec<&'a MirrorSection> = self
            .mirrors
            .iter()
            .filter(|m| m.of == registry_name || m.of == "*")
            .collect();
        v.sort_by_key(|m| m.priority);
        v
    }
}

#[cfg(test)]
#[path = "document/tests.rs"]
mod tests;
