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

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

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
            reason: "expected a [project] table — this manifest is not a plain project"
                .to_string(),
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
mod tests {
    use super::*;
    use crate::package_ref::PackageKind;

    #[test]
    fn new_project_is_valid_and_roundtrips() {
        let m = Manifest::new_project("demo", "0.0.1");
        m.validate().unwrap();
        assert!(!m.is_package());
        assert!(!m.is_workspace_root());
        let rendered = toml::to_string_pretty(&m).unwrap();
        let back = Manifest::parse_str(&rendered).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn minimal_project_parses() {
        let m = Manifest::parse_str("[project]\nname = \"demo\"\nversion = \"0.0.1\"\n").unwrap();
        assert_eq!(m.require_project().unwrap().name, "demo");
        assert!(m.package.is_none());
        assert!(m.registries.is_empty());
    }

    #[test]
    fn full_project_parses() {
        let raw = r#"
[project]
name = "my-client"
version = "0.0.1"
authors = ["Oleg <oleg@example.com>"]

[requires.packages]
"flow:wal" = "^0.3"
"stack:rust-cli" = "^0.1.0"

[active]
stack = "rust-cli"

[llm]
default_provider = "anthropic"
default_model = "claude-sonnet-4-7"

[[registry]]
name = "vibespecs"
url = "https://github.com/vibespecs"

[[registry]]
name = "corporate"
url = "git@internal:packages"
naming = "name"

[[mirror]]
of = "vibespecs"
url = "https://mirror.internal/vibespecs"
priority = 1

[[override]]
pkgref = "flow:wal"
source_url = "git@mycompany:forks/wal"
ref = "my-fix"
reason = "pending upstream PR"
"#;
        let m = Manifest::parse_str(raw).unwrap();
        assert_eq!(m.requires.packages.len(), 2);
        assert_eq!(m.registries.len(), 2);
        assert_eq!(m.primary_registry().unwrap().name, "vibespecs");
        assert_eq!(m.registry_by_name("corporate").unwrap().url, "git@internal:packages");
        assert_eq!(m.mirrors.len(), 1);
        assert_eq!(m.overrides.len(), 1);
        let back = Manifest::parse_str(&toml::to_string_pretty(&m).unwrap()).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn package_manifest_parses() {
        let raw = r#"
[package]
name = "wal"
kind = "flow"
version = "0.3.0"
license = "EULA"
description = "Write-Ahead Log discipline"

[compatibility]
min_vibe_version = "0.1.0"

[boot_snippet]
source = "boot/10-flow-wal.md"
category = "flow"

[provides]
capabilities = ["discipline:wal@0.3.0"]

[requires.packages]
"flow:atomic-commits" = "^0.1"
"#;
        let m = Manifest::parse_str(raw).unwrap();
        let pkg = m.require_package().unwrap();
        assert_eq!(pkg.name, "wal");
        assert_eq!(pkg.kind, PackageKind::Flow);
        assert!(pkg.publish.is_default());
        assert_eq!(
            m.boot_snippet.as_ref().unwrap().source.to_string_lossy(),
            "boot/10-flow-wal.md"
        );
        assert_eq!(m.provides.capabilities.len(), 1);
        assert_eq!(m.requires.packages.len(), 1);
        assert_eq!(m.as_package_ref().unwrap().name, "wal");
        let back = Manifest::parse_str(&toml::to_string_pretty(&m).unwrap()).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn workspace_root_with_members_parses() {
        let raw = r#"
[project]
name = "monorepo"
version = "0.0.1"

[workspace]
members = ["packages/flow-wal", "packages/feat-auth", "packages/stack-*"]
"#;
        let m = Manifest::parse_str(raw).unwrap();
        assert!(m.is_workspace_root());
        assert_eq!(m.workspace.as_ref().unwrap().members.len(), 3);
    }

    #[test]
    fn workspace_versions_parse_and_round_trip() {
        let raw = r#"
[project]
name = "mono"
version = "0.0.1"

[workspace]
members = ["packages/a"]

[workspace.versions]
core = "0.0.1"
ui = "^0.3"
"#;
        let m = Manifest::parse_str(raw).unwrap();
        let ws = m.workspace.as_ref().unwrap();
        assert_eq!(ws.versions.get("core").map(String::as_str), Some("0.0.1"));
        assert_eq!(ws.versions.get("ui").map(String::as_str), Some("^0.3"));
        let back = Manifest::parse_str(&toml::to_string_pretty(&m).unwrap()).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn root_package_composes_workspace_and_package() {
        // cargo-style: the root crate is itself publishable. PROP-007 §2.9.
        let raw = r#"
[package]
name = "umbrella"
kind = "stack"
version = "0.1.0"

[workspace]
members = ["packages/core"]
"#;
        let m = Manifest::parse_str(raw).unwrap();
        assert!(m.is_package());
        assert!(m.is_workspace_root());
    }

    #[test]
    fn virtual_workspace_root_parses() {
        // [workspace] alone — a pure coordinator, neither project nor package.
        let m = Manifest::parse_str("[workspace]\nmembers = [\"a\", \"b\"]\n").unwrap();
        assert!(m.is_workspace_root());
        assert!(!m.is_package());
        assert!(m.project.is_none());
    }

    #[test]
    fn origin_marker_parses() {
        let raw = r#"
[package]
name = "wal"
kind = "flow"
version = "0.3.0"

[origin]
upstream = "https://github.com/you/monorepo"
path = "packages/flow-wal"
commit = "abc123"
generated_by = "vibe 0.1.0"
generated_at = "2026-05-20T00:00:00Z"
"#;
        let m = Manifest::parse_str(raw).unwrap();
        let o = m.origin.as_ref().unwrap();
        assert_eq!(o.path, "packages/flow-wal");
        assert_eq!(o.commit.as_deref(), Some("abc123"));
        let back = Manifest::parse_str(&toml::to_string_pretty(&m).unwrap()).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn rejects_project_and_package_together() {
        let raw = r#"
[project]
name = "demo"
version = "0.0.1"

[package]
name = "demo"
kind = "flow"
version = "0.0.1"
"#;
        let err = Manifest::parse_str(raw).unwrap_err();
        assert!(err.to_string().contains("mutually exclusive"), "{err}");
    }

    #[test]
    fn rejects_no_role_section() {
        let err = Manifest::parse_str("[active]\nstack = \"rust\"\n").unwrap_err();
        assert!(err.to_string().contains("declares no role"), "{err}");
    }

    #[test]
    fn rejects_package_role_section_without_package() {
        let raw = r#"
[project]
name = "demo"
version = "0.0.1"

[boot_snippet]
source = "boot/x.md"
"#;
        let err = Manifest::parse_str(raw).unwrap_err();
        assert!(err.to_string().contains("[boot_snippet]"), "{err}");
        assert!(err.to_string().contains("without a [package]"), "{err}");
    }

    #[test]
    fn require_package_and_project_error_clearly() {
        let proj = Manifest::new_project("demo", "0.0.1");
        assert!(proj.require_package().is_err());
        assert!(proj.require_project().is_ok());
    }

    #[test]
    fn conditional_deps_parse() {
        let raw = r#"
[package]
name = "x"
kind = "flow"
version = "0.1.0"

[target."context(stack:rust)".dependencies]
packages = { "flow:rust-best-practices" = "^0.1" }
"#;
        let m = Manifest::parse_str(raw).unwrap();
        assert_eq!(m.conditional_deps.len(), 1);
        let t = m.conditional_deps.get("context(stack:rust)").unwrap();
        assert_eq!(t.dependencies.packages.len(), 1);
    }

    #[test]
    fn rejects_unknown_top_level_section() {
        let raw = r#"
[project]
name = "demo"
version = "0.0.1"

[mystery]
value = 1
"#;
        assert!(toml::from_str::<Manifest>(raw).is_err());
    }

    #[test]
    fn mirrors_for_filters_and_sorts() {
        let raw = r#"
[project]
name = "demo"
version = "0.1.0"

[[registry]]
name = "vibespecs"
url = "git@host:org"

[[mirror]]
of = "vibespecs"
url = "https://a"
priority = 2

[[mirror]]
of = "vibespecs"
url = "https://b"
priority = 1

[[mirror]]
of = "*"
url = "https://catchall"
priority = 99
"#;
        let m = Manifest::parse_str(raw).unwrap();
        let ms = m.mirrors_for("vibespecs");
        assert_eq!(ms.len(), 3);
        assert_eq!(ms[0].url, "https://b");
        assert_eq!(ms[1].url, "https://a");
        assert_eq!(ms[2].url, "https://catchall");
    }

    #[test]
    fn write_and_read_roundtrip_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vibe.toml");
        let mut m = Manifest::new_project("disk-demo", "0.1.0");
        m.registries.push(RegistrySection {
            name: "vibespecs".into(),
            url: "https://github.com/vibespecs".into(),
            r#ref: "main".into(),
            naming: super::super::NamingConvention::KindName,
            auth: super::super::AuthKind::None,
            token_env: None,
        });
        m.write(&path).unwrap();
        let back = Manifest::read(&path).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn boot_section_parses_and_round_trips() {
        let raw = r#"
[project]
name = "demo"
version = "0.1.0"

[boot]
default_link = "dynamic"
"#;
        let m = Manifest::parse_str(raw).unwrap();
        assert_eq!(m.boot.default_link, Some(LinkType::Dynamic));
        let back = Manifest::parse_str(&toml::to_string_pretty(&m).unwrap()).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn boot_section_absent_is_empty_and_not_emitted() {
        let m = Manifest::new_project("demo", "0.1.0");
        assert!(m.boot.is_empty());
        let rendered = toml::to_string_pretty(&m).unwrap();
        assert!(!rendered.contains("[boot]"), "{rendered}");
    }

    #[test]
    fn boot_section_is_consumer_side_allowed_without_package() {
        // [boot] is not a package-role section — valid on a plain project.
        let raw = r#"
[project]
name = "demo"
version = "0.1.0"

[boot]
default_link = "inline"
"#;
        Manifest::parse_str(raw).unwrap();
    }

    #[test]
    fn boot_section_rejects_unknown_field() {
        let raw = r#"
[project]
name = "demo"
version = "0.1.0"

[boot]
mystery = "x"
"#;
        assert!(toml::from_str::<Manifest>(raw).is_err());
    }
}
