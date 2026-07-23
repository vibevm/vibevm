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
//! Package identity is the tuple `(group, name, version, content_hash)`.
//! `source_url` is informational — it records where the content came from
//! on this particular install. Mirror-switching, host-migration, and
//! override pins all change `source_url` without changing identity; the
//! integrity check keys off `content_hash`. This is the property whose
//! absence trapped Nix on GitHub (PROP-002 §1).
//!
//! One lockfile lives at the absolute root of a workspace (PROP-007 §2.4) —
//! members never carry their own.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#lockfile");

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::content_hash::ContentHash;
use crate::error::Result;
use crate::package_ref::{Group, PackageKind, PackageName, PackageRef, VersionSpec};
use crate::provenance::{SourceUrl, TraceId};

use super::{Materialization, read_toml, write_toml};

/// The current — and only supported — lockfile schema version.
///
/// vibevm is pre-release and breaks lockfile compatibility freely. A
/// `vibe.lock` whose `schema_version` is not exactly this value is rejected
/// by [`Lockfile::read`]; the next `vibe install` regenerates it.
///
/// History (for the record only — earlier versions are not read):
/// `1` M0/M1.1 · `2` per-package registries · `3` PROP-003 features ·
/// `4` PROP-007 workspace path-source (`source_kind = "path"`) ·
/// `5` PROP-008 qualified naming (the per-package `group` field).
pub const CURRENT_SCHEMA_VERSION: u32 = 5;

fn is_false(b: &bool) -> bool {
    !*b
}

/// Top-level `vibe.lock` structure.
///
/// The on-disk TOML uses `[[package]]` array-of-tables. Serde's default
/// behavior flattens this when the field is named `package` and typed as
/// `Vec<LockedPackage>`.
///
/// ```
/// use vibe_core::manifest::Lockfile;
///
/// // A fresh lockfile carries no packages and the current schema.
/// let lf = Lockfile::empty("vibe 0.1.0", "2026-01-01T00:00:00Z");
/// assert!(lf.packages.is_empty());
/// assert_eq!(lf.meta.schema_version, vibe_core::manifest::CURRENT_SCHEMA_VERSION);
/// // Lookups are by the (group, name) identity; nothing is locked yet.
/// let group = vibe_core::Group::parse("org.vibevm").unwrap();
/// assert!(lf.find(&group, "wal").is_none());
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#lockfile")]
#[specmark::spec(implements = "spec://vibevm/VIBEVM-SPEC#lockfile-schema")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lockfile {
    pub meta: LockfileMeta,

    #[serde(default, rename = "package", skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<LockedPackage>,
}

/// `[meta]` — the lockfile's provenance header: who and when generated it,
/// the schema version, and the resolution-wide records (solver, roots,
/// language chain, active features, virtual capabilities).
///
/// ```
/// use vibe_core::manifest::{LockfileMeta, CURRENT_SCHEMA_VERSION};
///
/// let m: LockfileMeta = toml::from_str(r#"
///     generated_by = "vibe 0.1.0"
///     generated_at = "2026-05-21T12:00:00Z"
///     schema_version = 5
///     solver = "resolvo-0.x"
/// "#).unwrap();
/// assert_eq!(m.schema_version, CURRENT_SCHEMA_VERSION);
/// assert_eq!(m.solver.as_deref(), Some("resolvo-0.x"));
/// ```
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
    /// `<group>/<name>/<feature-name>`, scoped per package. Empty on v2
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
///
/// ```
/// use vibe_core::manifest::VirtualCapabilityRecord;
///
/// let v: VirtualCapabilityRecord = toml::from_str(r#"
///     name = "interface:llm-coordinator"
///     emitter = "anthropic:claude-opus-4-8"
///     trace_id = "build-2026-05-21-abc"
///     emitted_at = "2026-05-21T12:00:00Z"
/// "#).unwrap();
/// assert_eq!(v.name, "interface:llm-coordinator");
/// assert_eq!(v.trace_id, "build-2026-05-21-abc");
/// ```
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
    pub trace_id: TraceId,
    /// ISO-8601 timestamp.
    pub emitted_at: String,
}

/// Discriminator for `LockedPackage.source_kind` — which resolution path
/// produced the entry. Maps onto the short-circuit branches in
/// `MultiRegistryResolver::resolve`: `[[override]]` > path-source >
/// git-source > registry-walk. PROP-002 §2.4.1, PROP-007 §2.5.
///
/// ```
/// use vibe_core::manifest::{LockedPackage, SourceKind};
///
/// // The wire form is the lowercase name on a `[[package]].source_kind`:
/// let p: LockedPackage = toml::from_str(r#"
///     kind = "flow"
///     name = "wal"
///     group = "org.vibevm"
///     version = "0.1.0"
///     source_url = "packages/flow-wal"
///     content_hash = "sha256:abc"
///     source_kind = "path"
/// "#).unwrap();
/// assert_eq!(p.source_kind, Some(SourceKind::Path));
/// ```
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
    /// Resolved from the source-linked embedded registry — the in-tree
    /// `packages/` of a source-installed `vibe` (PROP-030 §2). `source_url`
    /// carries the `file://` path into that tree. The entry is
    /// machine-local; the reproducibility guard (PROP-030 §5) keys on this
    /// variant.
    Embedded,
    /// Resolved from the project-local `packages/` — the in-tree `packages/`
    /// of the *current* project (PROP-030 §3.3). Unlike `Embedded` (which
    /// derives from a vibe install's source_path), `Local` is per-project
    /// and portable: every checkout of the project carries the same
    /// `packages/`, so a lock entry with this source_kind does NOT trip the
    /// reproducibility guard. `source_url` carries the `file://` path into
    /// `<project_root>/packages/...`. The reserved name from PROP-030 §3.2
    /// / §9 D2, now implemented for the project-packages feature.
    Local,
}

/// One installed package, as it appears in the lockfile.
///
/// ```
/// use vibe_core::manifest::{LockedPackage, SourceKind};
/// use vibe_core::PackageKind;
///
/// let p: LockedPackage = toml::from_str(r#"
///     kind = "flow"
///     name = "wal"
///     group = "org.vibevm"
///     version = "0.3.0"
///     registry = "vibespecs"
///     source_url = "git@gitverse.ru:vibespecs/flow-wal.git"
///     content_hash = "sha256:abc"
///     source_kind = "registry"
/// "#).unwrap();
/// assert_eq!(p.kind, PackageKind::Flow);
/// assert_eq!(p.name, "wal");
/// assert_eq!(p.source_kind, Some(SourceKind::Registry));
/// // Materialization defaults to `snapshot` when the field is absent, so
/// // every lockfile written before the field landed parses unchanged.
/// assert!(p.materialization.is_default());
/// // Identity is (group, name, version, content_hash); `as_package_ref`
/// // pins this exact installed version.
/// assert_eq!(p.as_package_ref().unwrap().qualified_name(), "org.vibevm/wal");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedPackage {
    pub kind: PackageKind,
    pub name: PackageName,
    /// Reverse-FQDN group (PROP-008 §2.1). With `name` it forms the
    /// package's `(group, name, version, content_hash)` identity; `kind`
    /// is metadata, not part of identity (PROP-008 §2.2 / §2.3).
    pub group: Group,
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
    pub source_url: SourceUrl,

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
    /// **identity** component of the (group, name, version, content_hash)
    /// tuple. Present in every lockfile version.
    pub content_hash: ContentHash,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_snippet: Option<String>,

    #[serde(default)]
    pub files_written: Vec<PathBuf>,

    /// Transitive dependencies as resolved by the solver at install time.
    /// Each entry is pinned to an exact version (`group/name@=version`).
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

    /// How this package's content was materialised on disk (PROP-022 §2.1).
    /// Recorded so uninstall / `reinstall --force` and the destructive guard
    /// (PROP-022 §2.6) know an `in-place` slot is a git-native, unversioned,
    /// non-vendored working tree that must not be deleted unconfirmed.
    /// Default `snapshot`; skipped from the serialized form when default, so
    /// every lockfile written before this field parses unchanged.
    #[serde(default, skip_serializing_if = "Materialization::is_default")]
    pub materialization: Materialization,
}

/// One subskill entry under a package's `subskills_active` list.
///
/// ```
/// use vibe_core::manifest::LockedSubskill;
///
/// let s: LockedSubskill = toml::from_str(r#"
///     path = "stack/rust"
///     delivery = "eager"
///     files_written = ["spec/boot/15-flow-wal-rust.md"]
/// "#).unwrap();
/// assert_eq!(s.path, "stack/rust");
/// assert_eq!(s.delivery, "eager");
/// assert_eq!(s.files_written.len(), 1);
/// ```
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
        // `find`/`find_mut`/`remove` below treat `(group, name)` as a
        // unique key — first match wins. A lockfile carrying duplicate
        // identities would make those lookups silently
        // position-dependent; the solver never emits duplicates, so a
        // duplicate here means a hand-edited or corrupted file.
        debug_assert!(
            lockfile
                .packages
                .iter()
                .map(|p| (&p.group, &p.name))
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == lockfile.packages.len(),
            "lockfile carries duplicate (group, name) identities"
        );
        Ok(lockfile)
    }

    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        write_toml(path, self)
    }

    /// Find an installed package by its `(group, name)` identity.
    pub fn find(&self, group: &Group, name: &str) -> Option<&LockedPackage> {
        self.packages
            .iter()
            .find(|p| p.group == *group && p.name == name)
    }

    pub fn find_mut(&mut self, group: &Group, name: &str) -> Option<&mut LockedPackage> {
        self.packages
            .iter_mut()
            .find(|p| p.group == *group && p.name == name)
    }

    /// Remove an installed package; returns the removed entry if present.
    pub fn remove(&mut self, group: &Group, name: &str) -> Option<LockedPackage> {
        let idx = self
            .packages
            .iter()
            .position(|p| p.group == *group && p.name == name)?;
        Some(self.packages.remove(idx))
    }
}

impl LockedPackage {
    /// Produce a `PackageRef` pinned to this exact installed version —
    /// fully qualified, carrying the package's `group` and `kind`.
    pub fn as_package_ref(&self) -> Result<PackageRef> {
        // The `=` pin built structurally rather than via a string
        // round-trip: `VersionReq::parse("={version}")` rejects
        // versions carrying build metadata (a req has no
        // build-metadata grammar), and build metadata never
        // participates in pinning anyway.
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

#[cfg(test)]
#[path = "lockfile/tests.rs"]
mod tests;
