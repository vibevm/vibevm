//! `vibe.lock` — the project lockfile.
//!
//! Schema: `VIBEVM-SPEC.md` §7.4,
//! [PROP-002 §2.7](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#lockfile),
//! [PROP-007 §2.5](../../../spec/modules/vibe-workspace/PROP-007-workspace.md).
//!
//! # Schema version
//!
//! [`CURRENT_SCHEMA_VERSION`] is the one and only supported version.
//! vibevm is pre-release and breaks lockfile compatibility freely; there is
//! no migration path and none is needed. [`Lockfile::read`] rejects a
//! `vibe.lock` whose `schema_version` is anything else — the fix is always
//! to regenerate it with `vibe install`.
//!
//! # Identity vs. source_url
//!
//! Package identity is the tuple `(kind, name, version, content_hash)`.
//! `source_url` is informational — it records where the content came from
//! on this particular install. Mirror-switching, host-migration, and
//! override pins all change `source_url` without changing identity; the
//! integrity check keys off `content_hash`. This is the property whose
//! absence trapped Nix on GitHub (PROP-002 §1).
//!
//! One lockfile lives at the absolute root of a workspace (PROP-007 §2.4) —
//! members never carry their own.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::package_ref::{PackageKind, PackageRef, VersionSpec};

use super::{read_toml, write_toml};

/// The current — and only supported — lockfile schema version.
///
/// vibevm is pre-release and breaks lockfile compatibility freely. A
/// `vibe.lock` whose `schema_version` is not exactly this value is rejected
/// by [`Lockfile::read`]; the next `vibe install` regenerates it.
///
/// History (for the record only — earlier versions are not read):
/// `1` M0/M1.1 · `2` per-package registries · `3` PROP-003 features ·
/// `4` PROP-007 workspace path-source (`source_kind = "path"`).
pub const CURRENT_SCHEMA_VERSION: u32 = 4;

fn is_false(b: &bool) -> bool {
    !*b
}

/// Top-level `vibe.lock` structure.
///
/// The on-disk TOML uses `[[package]]` array-of-tables. Serde's default
/// behavior flattens this when the field is named `package` and typed as
/// `Vec<LockedPackage>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lockfile {
    pub meta: LockfileMeta,

    #[serde(
        default,
        rename = "package",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockfileMeta {
    pub generated_by: String,
    pub generated_at: String,

    /// Lockfile schema version — always [`CURRENT_SCHEMA_VERSION`]. A
    /// required field: a `vibe.lock` without it, or carrying any other
    /// value, is rejected by [`Lockfile::read`].
    pub schema_version: u32,

    /// Identity of the depsolver that produced this lockfile — e.g.
    /// `"resolvo-0.x"`. `None` for pre-resolver installs (Phase A
    /// straight-line) and for v1 lockfiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solver: Option<String>,

    /// Packages the user directly asked for (as opposed to transitive
    /// deps pulled in by the solver). `vibe uninstall <pkgref>` of a
    /// root dep prunes it and any transitives it uniquely reached;
    /// uninstalling a pure transitive is refused.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub root_dependencies: Vec<PackageRef>,

    /// Resolved language preference for this project — the chain
    /// produced by `[i18n].project_preference_chain()` at install
    /// time. First entry is primary, last is the canonical fallback.
    /// Empty/absent on v2 lockfiles or when no `[i18n]` block is
    /// declared at the project level.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub language_chain: Vec<String>,

    /// Full set of features active in this resolution. Each entry is
    /// `<kind>:<name>/<feature-name>`, scoped per package. Empty on v2
    /// lockfiles and when no features are configured.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_features: Vec<String>,

    /// Virtual capabilities emitted by an LLM during resolution
    /// (Phase F — post-M1.5). Each entry carries the capability name
    /// plus an audit trail tying it to the emitting model and trace
    /// ID.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub virtual_capabilities: Vec<VirtualCapabilityRecord>,
}

/// One LLM-emitted virtual capability record. PROP-003 §2.5.3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VirtualCapabilityRecord {
    /// Capability name, in the same namespace as static capabilities.
    /// Examples: `interface:llm-coordinator`, `capability:rust-tracing`.
    pub name: String,
    /// Provider/model identifier — `anthropic:claude-sonnet-4-6`.
    pub emitter: String,
    /// Trace ID of the LLM run that emitted this capability. Links into
    /// the `vibe build` audit log.
    pub trace_id: String,
    /// ISO-8601 timestamp.
    pub emitted_at: String,
}

/// Discriminator for `LockedPackage.source_kind` — which resolution path
/// produced the entry. Maps onto the short-circuit branches in
/// `MultiRegistryResolver::resolve`: `[[override]]` > path-source >
/// git-source > registry-walk. PROP-002 §2.4.1, PROP-007 §2.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    /// Resolved through the `[[registry]]` walk.
    Registry,
    /// A git-source declaration — `[requires.packages]` `{ git = … }`.
    Git,
    /// A `[[override]]` pin.
    Override,
    /// A path-source declaration — `[requires.packages]` `{ path = … }`,
    /// typically a sibling workspace member. `source_url` then carries the
    /// path relative to the workspace root, not a URL. PROP-007 §2.5.
    Path,
}

/// One installed package, as it appears in the lockfile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedPackage {
    pub kind: PackageKind,
    pub name: String,
    pub version: semver::Version,

    /// Name of the registry that served this package — matches a
    /// `[[registry]].name` in `vibe.toml`. `None` for local-directory
    /// registries (`--registry <path>`), override-resolved packages, and
    /// v1 lockfiles (which didn't record this).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,

    /// Where the content came from on the install that produced this
    /// entry. Informational — identity is `(kind, name, version,
    /// content_hash)`. A git URL for registry / git-source / override
    /// entries; for a path-source entry (`source_kind = "path"`) it is the
    /// member's path relative to the workspace root — portable, never an
    /// absolute path.
    pub source_url: String,

    /// Git ref the content was fetched at — typically the version tag
    /// (`v0.3.0`). `None` for non-git registries (`file://…`) and v1
    /// lockfiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,

    /// Commit the ref resolved to at install time. Lets a future
    /// `vibe check` detect silent tag rewrites against the same URL.
    /// `None` for non-git sources and v1 lockfiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_commit: Option<String>,

    /// `sha256:<hex>` content hash over the package tree. The
    /// **identity** component of the (kind, name, version, content_hash)
    /// tuple. Present in every lockfile version.
    pub content_hash: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_snippet: Option<String>,

    #[serde(default)]
    pub files_written: Vec<PathBuf>,

    /// Transitive dependencies as resolved by the solver at install time.
    /// Each entry is pinned to an exact version (`kind:name@=version`).
    /// Empty for pre-resolver installs and v1 lockfiles.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<PackageRef>,

    /// `true` iff this package was resolved through a `[[override]]` in
    /// `vibe.toml` rather than through the registry layer. Surfaces in
    /// `vibe list --overrides` and gates certain update paths.
    #[serde(default, skip_serializing_if = "is_false")]
    pub overridden: bool,

    /// Resolution path that produced this entry — `registry`, `git`,
    /// `override`, or `path`. `None` only on an entry that predates the
    /// field; fresh writes always set it. PROP-002 §2.4.1, PROP-007 §2.5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<SourceKind>,

    /// When this entry was resolved via a registry stub that redirected
    /// to an external URL (PROP-002 §2.4.2), records the **stub** URL
    /// here while `source_url` carries the **target** URL. `None` for
    /// non-redirected entries — the common case. Diagnostic / auditing
    /// only; `vibe show <pkgref>` and `vibe list --json` surface this
    /// to operators.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via_redirect: Option<String>,

    /// Features active for this package (PROP-003 §2.4). Empty for
    /// packages with no `[features]` table or where no features were
    /// requested.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,

    /// Subskills active for this package — each entry is the subskill's
    /// canonical path plus the resolved delivery mode (so reproducing
    /// a checkout produces the same on-disk shape).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subskills_active: Vec<LockedSubskill>,

    /// PURL pinning this package to an upstream library version, if
    /// the package's manifest carried `[package].describes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub describes: Option<String>,

    /// Language under which this package's content was materialised.
    /// `None` ⇒ inherits `[meta].language_chain[0]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

/// One subskill entry under a package's `subskills_active` list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedSubskill {
    /// Subskill canonical path within the parent package, e.g.
    /// `stack/rust`.
    pub path: String,
    /// Resolved delivery mode — `eager` / `lazy-push` / `lazy-pull`.
    pub delivery: String,
    /// PURL inherited or declared on the subskill, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub describes: Option<String>,
    /// Project-relative files this subskill *specifically* contributed
    /// — distinct from the package-level `files_written` aggregate.
    /// Empty for `delivery=lazy-pull` since those files are never
    /// materialised on disk; the per-subskill index lives in the
    /// lockfile so `vibe-mcp::read_subskill` can resolve them by
    /// loading from the package cache rather than the project tree.
    /// Empty for legacy `subskills_active` entries written before
    /// this field landed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files_written: Vec<PathBuf>,

    /// For `delivery=lazy-pull`: the files the subskill carries
    /// inside the package cache, relative to the subskill's own
    /// root (`<cache>/subskills/<path>/<...>`). Used by
    /// `vibe-mcp::read_subskill` to fetch on-demand without ever
    /// touching the project tree.
    /// Absent for `eager` / `lazy-push` deliveries (they
    /// materialise into the project, recorded under
    /// `files_written` above).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cache_files: Vec<PathBuf>,
}

impl Lockfile {
    pub const FILENAME: &'static str = "vibe.lock";

    pub fn empty(generated_by: impl Into<String>, generated_at: impl Into<String>) -> Self {
        Lockfile {
            meta: LockfileMeta {
                generated_by: generated_by.into(),
                generated_at: generated_at.into(),
                schema_version: CURRENT_SCHEMA_VERSION,
                solver: None,
                root_dependencies: Vec::new(),
                language_chain: Vec::new(),
                active_features: Vec::new(),
                virtual_capabilities: Vec::new(),
            },
            packages: Vec::new(),
        }
    }

    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        let lockfile: Lockfile = read_toml(path)?;
        if lockfile.meta.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(crate::error::Error::UnsupportedLockfile {
                found: lockfile.meta.schema_version,
                expected: CURRENT_SCHEMA_VERSION,
            });
        }
        Ok(lockfile)
    }

    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        write_toml(path, self)
    }

    /// Find an installed package by its `<kind>:<name>` identity.
    pub fn find(&self, kind: PackageKind, name: &str) -> Option<&LockedPackage> {
        self.packages
            .iter()
            .find(|p| p.kind == kind && p.name == name)
    }

    pub fn find_mut(&mut self, kind: PackageKind, name: &str) -> Option<&mut LockedPackage> {
        self.packages
            .iter_mut()
            .find(|p| p.kind == kind && p.name == name)
    }

    /// Remove an installed package; returns the removed entry if present.
    pub fn remove(&mut self, kind: PackageKind, name: &str) -> Option<LockedPackage> {
        let idx = self
            .packages
            .iter()
            .position(|p| p.kind == kind && p.name == name)?;
        Some(self.packages.remove(idx))
    }

}

impl LockedPackage {
    /// Produce a `PackageRef` pinned to this exact installed version.
    pub fn as_package_ref(&self) -> Result<PackageRef> {
        let req = semver::VersionReq::parse(&format!("={}", self.version))
            .expect("exact version always parses");
        PackageRef::new(self.kind, self.name.clone(), VersionSpec::Req(req))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
[meta]
generated_by = "vibe 0.1.0-dev"
generated_at = "2026-05-21T12:00:00Z"
schema_version = 4
solver = "resolvo-0.x"
root_dependencies = ["flow:wal", "stack:rust-cli"]

[[package]]
kind = "flow"
name = "wal"
version = "0.3.0"
registry = "vibespecs"
source_url = "git@gitverse.ru:vibespecs/flow-wal.git"
source_ref = "v0.3.0"
resolved_commit = "abc123def456"
content_hash = "sha256:abc"
source_kind = "registry"
boot_snippet = "10-flow-wal.md"
files_written = [
    "spec/flows/wal/WAL-PROTOCOL.md",
    "spec/boot/10-flow-wal.md",
]
dependencies = ["flow:atomic-commits@=0.1.0"]

[[package]]
kind = "stack"
name = "rust-cli"
version = "0.1.0"
registry = "vibespecs"
source_url = "git@gitverse.ru:vibespecs/stack-rust-cli.git"
source_ref = "v0.1.0"
resolved_commit = "999888777666"
content_hash = "sha256:def"
source_kind = "registry"
"#;

    #[test]
    fn parses_fully() {
        let lf: Lockfile = toml::from_str(FIXTURE).unwrap();
        assert_eq!(lf.meta.schema_version, 4);
        assert_eq!(lf.meta.solver.as_deref(), Some("resolvo-0.x"));
        assert_eq!(lf.meta.root_dependencies.len(), 2);
        assert_eq!(lf.packages.len(), 2);

        let wal = lf.find(PackageKind::Flow, "wal").unwrap();
        assert_eq!(wal.version.to_string(), "0.3.0");
        assert_eq!(wal.registry.as_deref(), Some("vibespecs"));
        assert_eq!(wal.source_url, "git@gitverse.ru:vibespecs/flow-wal.git");
        assert_eq!(wal.source_ref.as_deref(), Some("v0.3.0"));
        assert_eq!(wal.resolved_commit.as_deref(), Some("abc123def456"));
        assert_eq!(wal.dependencies.len(), 1);
        assert_eq!(wal.dependencies[0].qualified_name(), "flow:atomic-commits");
        assert_eq!(wal.source_kind, Some(SourceKind::Registry));
        assert!(!wal.overridden);
    }

    #[test]
    fn roundtrip() {
        let lf: Lockfile = toml::from_str(FIXTURE).unwrap();
        let rendered = toml::to_string_pretty(&lf).unwrap();
        let back: Lockfile = toml::from_str(&rendered).unwrap();
        assert_eq!(lf, back);
    }

    #[test]
    fn empty_lockfile_has_v4_defaults() {
        let lf = Lockfile::empty("vibe 0.1.0-dev", "2026-05-21T00:00:00Z");
        assert_eq!(lf.meta.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(CURRENT_SCHEMA_VERSION, 4);
        assert!(lf.meta.solver.is_none());
        assert!(lf.packages.is_empty());

        let rendered = toml::to_string_pretty(&lf).unwrap();
        let back: Lockfile = toml::from_str(&rendered).unwrap();
        assert_eq!(lf, back);
    }

    #[test]
    fn read_accepts_current_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vibe.lock");
        Lockfile::empty("vibe", "2026-05-21T00:00:00Z")
            .write(&path)
            .unwrap();
        let lf = Lockfile::read(&path).unwrap();
        assert_eq!(lf.meta.schema_version, 4);
    }

    #[test]
    fn read_rejects_non_current_version() {
        // A pre-v4 lockfile is rejected outright — no legacy reader, no
        // migration. The fix is to regenerate with `vibe install`.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vibe.lock");
        std::fs::write(
            &path,
            "[meta]\ngenerated_by = \"old\"\ngenerated_at = \"x\"\nschema_version = 3\n",
        )
        .unwrap();
        let err = Lockfile::read(&path).unwrap_err();
        assert!(
            matches!(
                err,
                crate::error::Error::UnsupportedLockfile {
                    found: 3,
                    expected: 4
                }
            ),
            "{err}"
        );
    }

    #[test]
    fn path_source_kind_round_trips() {
        // A path-source member: source_kind = "path", and source_url is the
        // workspace-root-relative path, not a URL. PROP-007 §2.5.
        let raw = r#"
[meta]
generated_by = "vibe"
generated_at = "2026-05-21T00:00:00Z"
schema_version = 4

[[package]]
kind = "flow"
name = "wal"
version = "0.1.0"
source_url = "packages/flow-wal"
content_hash = "sha256:abc"
source_kind = "path"
"#;
        let lf: Lockfile = toml::from_str(raw).unwrap();
        let wal = lf.find(PackageKind::Flow, "wal").unwrap();
        assert_eq!(wal.source_kind, Some(SourceKind::Path));
        assert_eq!(wal.source_url, "packages/flow-wal");
        let rendered = toml::to_string_pretty(&lf).unwrap();
        assert!(rendered.contains("source_kind = \"path\""));
        let back: Lockfile = toml::from_str(&rendered).unwrap();
        assert_eq!(lf, back);
    }

    #[test]
    fn rejects_missing_schema_version() {
        // schema_version is a required field — no default.
        let raw = "[meta]\ngenerated_by = \"vibe\"\ngenerated_at = \"x\"\n";
        assert!(toml::from_str::<Lockfile>(raw).is_err());
    }

    #[test]
    fn remove_drops_entry() {
        let mut lf: Lockfile = toml::from_str(FIXTURE).unwrap();
        assert_eq!(lf.packages.len(), 2);
        let removed = lf.remove(PackageKind::Flow, "wal").unwrap();
        assert_eq!(removed.name, "wal");
        assert_eq!(lf.packages.len(), 1);
        assert!(lf.find(PackageKind::Flow, "wal").is_none());
    }

    #[test]
    fn override_flag_round_trips() {
        let raw = r#"
[meta]
generated_by = "vibe 0.1.0-dev"
generated_at = "2026-05-21T00:00:00Z"
schema_version = 4

[[package]]
kind = "flow"
name = "wal"
version = "0.3.0"
source_url = "git@mycompany:forks/wal"
source_ref = "my-fix"
content_hash = "sha256:xyz"
source_kind = "override"
overridden = true
"#;
        let lf: Lockfile = toml::from_str(raw).unwrap();
        assert!(lf.packages[0].overridden);

        let rendered = toml::to_string_pretty(&lf).unwrap();
        assert!(rendered.contains("overridden = true"));
        // The false case (default) is skipped on serialize.
        let mut lf2 = lf.clone();
        lf2.packages[0].overridden = false;
        let rendered2 = toml::to_string_pretty(&lf2).unwrap();
        assert!(!rendered2.contains("overridden"));
    }

    #[test]
    fn rejects_unknown_package_field() {
        let raw = r#"
[meta]
generated_by = "vibe"
generated_at = "2026-05-21T00:00:00Z"
schema_version = 4

[[package]]
kind = "flow"
name = "wal"
version = "0.1.0"
source_url = "file:///x"
content_hash = "sha256:abc"
mystery = true
"#;
        assert!(toml::from_str::<Lockfile>(raw).is_err());
    }
}
