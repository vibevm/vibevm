//! `vibe-subskill.toml` — manifest for an optional content unit inside a
//! package.
//!
//! Spec: [PROP-003 §2.5](../../../spec/modules/vibe-resolver/PROP-003-dep-evolution.md#subskills).
//!
//! A subskill is the smallest activatable content unit inside a package.
//! It has its own manifest, files, and (optionally) nested subskills.
//! Activation rules and a delivery mode (eager / lazy-push / lazy-pull)
//! decide *whether* and *how* the subskill's content reaches the agent.

specmark::scope!("spec://vibevm/modules/vibe-resolver/PROP-003#subskills");

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::capability_ref::CapabilityRef;
use crate::error::Result;
use crate::package_ref::PackageRef;

use super::purl::Purl;
use super::{read_toml, write_toml};

/// Top-level `vibe-subskill.toml`.
///
/// ```
/// use vibe_core::manifest::SubskillManifest;
///
/// let m: SubskillManifest = toml::from_str(r#"
///     [subskill]
///     path = "stack/rust"
///     description = "Rust + sqlx guidance"
///     delivery = "lazy-push"
///
///     [activation]
///     if_files = ["**/Cargo.toml"]
/// "#).unwrap();
/// assert_eq!(m.subskill.path, "stack/rust");
/// // `lazy-push` is valid here because a description is present.
/// assert!(m.validation_findings().is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubskillManifest {
    pub subskill: SubskillMeta,

    #[serde(default)]
    pub activation: ActivationRules,

    /// Soft-preference: subskills that should also activate when this
    /// one is active (libsolv-Recommends-style).
    #[serde(default, skip_serializing_if = "SubskillRecommends::is_empty")]
    pub recommends: SubskillRecommends,

    /// Hard exclusion: subskills that must not activate alongside this
    /// one.
    #[serde(default, skip_serializing_if = "SubskillConflicts::is_empty")]
    pub conflicts: SubskillConflicts,

    #[serde(default, skip_serializing_if = "SubskillContent::is_empty")]
    pub content: SubskillContent,
}

/// `[subskill]` — identity and delivery metadata for one subskill.
///
/// ```
/// use vibe_core::manifest::{SubskillMeta, DeliveryMode};
///
/// let s: SubskillMeta = toml::from_str(r#"
///     path = "stack/rust"
///     delivery = "lazy-pull"
/// "#).unwrap();
/// assert_eq!(s.path, "stack/rust");
/// assert_eq!(s.delivery, DeliveryMode::LazyPull);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubskillMeta {
    /// Canonical addressable name within the parent package — e.g.
    /// `stack/rust`, `feature/atomic-only`. Forward-slash separated;
    /// matches the relative path under `subskills/`.
    pub path: String,

    /// Short one-line summary, surfaced in `vibe show subskills`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Natural-language activation trigger — load-bearing for
    /// `lazy-push` and `lazy-pull` modes. The agent matches this
    /// against the current task / files / conversation. `vibe review`
    /// scores it under "activation distinctiveness."
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// How this subskill reaches the agent. Default: `eager`
    /// (materialised at install time, current vibevm behaviour).
    #[serde(default)]
    pub delivery: DeliveryMode,

    /// PURL pinning this subskill to an upstream OSS package version.
    /// Optional; when set, the subskill applies only to projects that
    /// declare a matching upstream via `[package].describes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub describes: Option<Purl>,
}

/// Three delivery modes per PROP-003 §2.5.0.
///
/// ```
/// use vibe_core::manifest::DeliveryMode;
///
/// assert_eq!(DeliveryMode::default(), DeliveryMode::Eager);
/// assert_eq!(DeliveryMode::LazyPush.as_str(), "lazy-push");
/// // The lazy modes match against the agent's task, so they need a description.
/// assert!(DeliveryMode::LazyPull.requires_description());
/// assert!(!DeliveryMode::Eager.requires_description());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryMode {
    /// Materialise into the project tree at install time. The default —
    /// matches the existing vibevm behaviour for top-level package
    /// content.
    #[default]
    Eager,
    /// Pushed into the agent's MCP context when the agent's task
    /// description matches the subskill's `description`. Never written
    /// to disk by default.
    LazyPush,
    /// Fetched only on agent request through `vibe-mcp::read_subskill`.
    /// Never written to disk; never auto-loaded.
    LazyPull,
}

impl DeliveryMode {
    pub fn as_str(self) -> &'static str {
        match self {
            DeliveryMode::Eager => "eager",
            DeliveryMode::LazyPush => "lazy-push",
            DeliveryMode::LazyPull => "lazy-pull",
        }
    }

    pub fn requires_description(self) -> bool {
        matches!(self, DeliveryMode::LazyPush | DeliveryMode::LazyPull)
    }
}

/// `[activation]` — multi-channel activation rules.
///
/// All fields are optional; an empty `[activation]` block means the
/// subskill activates only manually (via parent `[features]` mapping).
///
/// ```
/// use vibe_core::manifest::ActivationRules;
///
/// let a: ActivationRules = toml::from_str(r#"
///     if_present = ["stack:rust"]
///     if_files = ["**/Cargo.toml"]
/// "#).unwrap();
/// assert_eq!(a.if_present, vec!["stack:rust"]);
/// assert!(a.allow_llm_emission); // defaults to true
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationRules {
    /// Capabilities or pkgrefs that must be present in the resolved
    /// graph for activation. Strings of the form `<kind>:<name>` or
    /// `capability:<...>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub if_present: Vec<String>,

    /// Interface tags that must be provided by some package in the
    /// graph. Strings of the form `interface:<...>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub if_provides: Vec<String>,

    /// Glob patterns that must match the project tree.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub if_files: Vec<String>,

    /// Commands that must resolve on the user's PATH. Implementation
    /// pending; manifest field reserved for forward compatibility.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub if_command: Vec<String>,

    /// Env vars that must be set. Implementation pending; manifest
    /// field reserved for forward compatibility.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub if_env: Vec<String>,

    /// Operating systems this subskill is scoped to — a session whose OS
    /// is not in the set does not activate it. Values are
    /// `std::env::consts::OS` names (`windows`, `macos`, `linux`). The OS
    /// probe is the same one the `[boot_snippet]` `when` gate ships
    /// end-to-end (PROP-009 §2.4); here it is reserved for forward
    /// compatibility, inert until the subskill activation engine lands.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub if_os: Vec<String>,

    /// Match if any package or the project carries a `describes` PURL
    /// of the same `<type>` as this subskill's `describes`. Off by
    /// default; set to `true` to opt in.
    #[serde(default, skip_serializing_if = "is_false")]
    pub if_describes_match: bool,

    /// BCP-47 language tags. Activates if the consumer's resolved
    /// language preference is in the set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub if_language: Vec<String>,

    /// Allow LLM-emitted virtual capabilities (Phase F) to influence
    /// this subskill's activation through `if_present` / `if_provides`.
    /// Default `true`; set false to refuse virtual emissions.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub allow_llm_emission: bool,
}

fn default_true() -> bool {
    true
}
fn is_true(b: &bool) -> bool {
    *b
}
fn is_false(b: &bool) -> bool {
    !*b
}

/// `[recommends]` — soft preference: subskills, packages, or capabilities
/// that should also activate alongside this one (libsolv-Recommends-style).
///
/// ```
/// use vibe_core::manifest::SubskillRecommends;
///
/// let r: SubskillRecommends = toml::from_str(r#"
///     subskills = ["feature/atomic-only"]
/// "#).unwrap();
/// assert_eq!(r.subskills, vec!["feature/atomic-only"]);
/// assert!(!r.is_empty());
/// assert!(SubskillRecommends::default().is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubskillRecommends {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subskills: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<PackageRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<CapabilityRef>,
}

impl SubskillRecommends {
    pub fn is_empty(&self) -> bool {
        self.subskills.is_empty() && self.packages.is_empty() && self.capabilities.is_empty()
    }
}

/// `[conflicts]` — hard exclusion: subskills or packages that must not
/// activate alongside this one.
///
/// ```
/// use vibe_core::manifest::SubskillConflicts;
///
/// let c: SubskillConflicts = toml::from_str(r#"
///     subskills = ["stack/python"]
/// "#).unwrap();
/// assert!(!c.is_empty());
/// assert!(SubskillConflicts::default().is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubskillConflicts {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subskills: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<PackageRef>,
}

impl SubskillConflicts {
    pub fn is_empty(&self) -> bool {
        self.subskills.is_empty() && self.packages.is_empty()
    }
}

/// `[content]` — the files a subskill ships, relative to its own root.
///
/// ```
/// use vibe_core::manifest::SubskillContent;
///
/// let c: SubskillContent = toml::from_str(r#"
///     files_written = ["spec/boot/15-flow-wal-rust.md"]
/// "#).unwrap();
/// assert_eq!(c.files_written.len(), 1);
/// assert!(SubskillContent::default().is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubskillContent {
    /// Files this subskill ships. Paths are relative to the subskill's
    /// own root (the directory containing `vibe-subskill.toml`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files_written: Vec<PathBuf>,
}

impl SubskillContent {
    pub fn is_empty(&self) -> bool {
        self.files_written.is_empty()
    }
}

impl SubskillManifest {
    pub const FILENAME: &'static str = "vibe-subskill.toml";

    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        read_toml(path)
    }

    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        write_toml(path, self)
    }

    /// Static validation per PROP-003 §2.10's "subskill structure"
    /// check. Returns a list of diagnostics (empty = valid).
    pub fn validation_findings(&self) -> Vec<String> {
        let mut findings = Vec::new();
        if self.subskill.delivery.requires_description() && self.subskill.description.is_none() {
            findings.push(format!(
                "subskill `{}`: delivery `{}` requires a non-empty `description`",
                self.subskill.path,
                self.subskill.delivery.as_str()
            ));
        }
        if self.subskill.path.is_empty() {
            findings.push("subskill: `path` field must be non-empty".to_string());
        }
        for s in &self.subskill.path.split('/').collect::<Vec<_>>() {
            if s.is_empty() {
                findings.push(format!(
                    "subskill `{}`: path segments must be non-empty (no leading/trailing/double `/`)",
                    self.subskill.path
                ));
                break;
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_FULL: &str = r#"
[subskill]
path = "stack/rust"
summary = "Rust-specific guidance"
description = "When working with WAL in a Rust project using sqlx."
delivery = "lazy-push"
describes = "pkg:cargo/sqlx@0.8.0"

[activation]
if_present = ["stack:rust"]
if_provides = ["interface:build-system"]
if_files = ["**/Cargo.toml"]
if_command = ["cargo"]
if_env = ["RUST_LOG"]
if_os = ["linux", "macos"]
if_describes_match = true
if_language = ["en", "ru"]
allow_llm_emission = true

[recommends]
subskills = ["feature/atomic-only"]

[conflicts]
subskills = ["stack/python"]

[content]
files_written = [
    "spec/flows/wal/rust-specific-protocol.md",
    "spec/boot/15-flow-wal-rust.md",
]
"#;

    #[test]
    fn parses_full() {
        let m: SubskillManifest = toml::from_str(FIXTURE_FULL).unwrap();
        assert_eq!(m.subskill.path, "stack/rust");
        assert_eq!(m.subskill.delivery, DeliveryMode::LazyPush);
        assert_eq!(
            m.subskill.describes.as_ref().map(|p| p.purl_type.as_str()),
            Some("cargo")
        );
        assert_eq!(m.activation.if_present, vec!["stack:rust"]);
        assert_eq!(m.activation.if_files, vec!["**/Cargo.toml"]);
        assert_eq!(m.activation.if_os, vec!["linux", "macos"]);
        assert!(m.activation.if_describes_match);
        assert_eq!(m.recommends.subskills, vec!["feature/atomic-only"]);
        assert_eq!(m.content.files_written.len(), 2);
    }

    #[test]
    fn delivery_default_is_eager() {
        let raw = r#"
[subskill]
path = "x"
"#;
        let m: SubskillManifest = toml::from_str(raw).unwrap();
        assert_eq!(m.subskill.delivery, DeliveryMode::Eager);
    }

    #[test]
    fn lazy_push_without_description_flagged() {
        let raw = r#"
[subskill]
path = "stack/rust"
delivery = "lazy-push"
"#;
        let m: SubskillManifest = toml::from_str(raw).unwrap();
        let findings = m.validation_findings();
        assert!(
            findings
                .iter()
                .any(|f| f.contains("requires a non-empty `description`"))
        );
    }

    #[test]
    fn eager_without_description_ok() {
        let raw = r#"
[subskill]
path = "stack/rust"
"#;
        let m: SubskillManifest = toml::from_str(raw).unwrap();
        assert!(m.validation_findings().is_empty());
    }

    #[test]
    fn rejects_unknown_top_level_field() {
        let raw = r#"
[subskill]
path = "x"

[bogus]
value = 1
"#;
        assert!(toml::from_str::<SubskillManifest>(raw).is_err());
    }

    #[test]
    fn round_trip_preserves_shape() {
        let m: SubskillManifest = toml::from_str(FIXTURE_FULL).unwrap();
        let rendered = toml::to_string_pretty(&m).unwrap();
        let back: SubskillManifest = toml::from_str(&rendered).unwrap();
        assert_eq!(m, back);
    }
}
