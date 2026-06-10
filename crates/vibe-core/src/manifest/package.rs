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
use crate::error::{Error, Result};
use crate::package_ref::{Group, PackageKind, PackageRef, VersionSpec};

use super::project::AuthKind;
use super::purl::Purl;

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
        let req = semver::VersionReq::parse(&format!("={}", self.version))
            .expect("exact version string always parses as VersionReq");
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

/// A `[boot_snippet]` activation condition (PROP-009 §2.4 / §2.6).
///
/// A boot snippet carrying a `when` is a **conditional** contribution:
/// `vibe` renders it as a `dynamic` `INDEX.md` entry — irrespective of any
/// `link`, since a condition cannot be honoured by the verbatim `inline`
/// lane or a direct `static` read — and the agent reads the file at boot
/// only when the condition holds.
///
/// For v1 the sole condition is an operating-system match — enough to ship
/// OS-specific packages and subskills. The wire form is the string
/// `"os:<name>"`, `<name>` one of `windows` / `macos` / `linux`; it is
/// carried verbatim into the generated `INDEX.md`. The richer `[activation]`
/// probe vocabulary (PROP-003 §2.5) folds in when that engine is built;
/// `os:` is its first probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum WhenCondition {
    /// Activates only when the session's operating system is the named one.
    Os(TargetOs),
}

impl WhenCondition {
    /// `true` when this condition holds for the operating system the
    /// current process runs on.
    pub fn matches_current_os(self) -> bool {
        match self {
            WhenCondition::Os(os) => TargetOs::current() == Some(os),
        }
    }
}

impl std::fmt::Display for WhenCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WhenCondition::Os(os) => write!(f, "os:{os}"),
        }
    }
}

impl std::str::FromStr for WhenCondition {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let Some(os_name) = s.strip_prefix("os:") else {
            return Err(Error::BadWhenCondition {
                input: s.to_string(),
                reason: "unrecognised condition — expected `os:<name>`".to_string(),
            });
        };
        match os_name {
            "windows" => Ok(WhenCondition::Os(TargetOs::Windows)),
            "macos" => Ok(WhenCondition::Os(TargetOs::Macos)),
            "linux" => Ok(WhenCondition::Os(TargetOs::Linux)),
            other => Err(Error::BadWhenCondition {
                input: s.to_string(),
                reason: format!(
                    "unknown operating system `{other}` — expected `windows`, `macos`, or `linux`"
                ),
            }),
        }
    }
}

impl TryFrom<String> for WhenCondition {
    type Error = String;

    fn try_from(s: String) -> std::result::Result<Self, String> {
        s.parse().map_err(|e: Error| e.to_string())
    }
}

impl From<WhenCondition> for String {
    fn from(w: WhenCondition) -> String {
        w.to_string()
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

/// `[requires.packages.<pkgref>]` inline-table value when the package is
/// sourced from an arbitrary git repository instead of a registry.
///
/// Spec: PROP-002 §2.4.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitPackageDep {
    /// Optional `kind` prefix carried by the pkgref key (PROP-008 §2.4).
    pub kind: Option<PackageKind>,
    /// Reverse-FQDN group — a manifest pkgref is always qualified.
    pub group: Group,
    pub name: String,
    /// Full git URL of the single-package repository.
    pub url: String,
    /// Exactly one of `tag`, `branch`, `rev` — wire-grammar enforced.
    pub ref_kind: GitRefKind,
    /// Optional verification constraint. After resolving the package
    /// version from `ref_kind`, the constraint must be satisfied; mismatch
    /// is `VersionMismatch` at install time. `None` = accept whatever.
    pub version: Option<VersionSpec>,
    /// Per-source authentication regime (default `none`).
    pub auth: AuthKind,
    /// Env-var name when `auth = "token-env"`. `None` = derive from URL host.
    pub token_env: Option<String>,
}

/// Which kind of git ref the operator declared on a `[requires.packages.*]`
/// git-source entry. Exactly one of the three is required at parse time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitRefKind {
    Tag(String),
    Branch(String),
    Rev(String),
}

impl GitRefKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Tag(s) | Self::Branch(s) | Self::Rev(s) => s.as_str(),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Tag(_) => "tag",
            Self::Branch(_) => "branch",
            Self::Rev(_) => "rev",
        }
    }
}

/// A `[requires.packages.<pkgref>]` inline-table value pointing at a package
/// in a local directory — typically a sibling workspace member. PROP-007 §2.5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPackageDep {
    /// Optional `kind` prefix carried by the pkgref key (PROP-008 §2.4).
    pub kind: Option<PackageKind>,
    /// Reverse-FQDN group — a manifest pkgref is always qualified.
    pub group: Group,
    pub name: String,
    /// Path to the package directory, relative to the manifest that declares
    /// this dependency. Forward-slashed; portable across machines.
    pub path: String,
    /// Optional version constraint — the dual-form `{ path, version }`.
    /// `path` drives local development inside the workspace; `version` takes
    /// effect when the consuming node is itself published (the published copy
    /// references the registry version — an external consumer has no access
    /// to the local path). Required for any path-dep whose consumer is itself
    /// publishable; that is enforced at publish time, not here.
    pub version: Option<VersionSpec>,
}

/// A `[requires.packages.<pkgref>]` registry-resolved entry whose version is
/// a `[workspace.versions]` placeholder — `{ version.var = "core" }`. Carries
/// the unresolved placeholder name; `vibe-workspace` resolves it. PROP-007 §2.6.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarRegistryDep {
    /// Optional `kind` prefix carried by the pkgref key (PROP-008 §2.4).
    pub kind: Option<PackageKind>,
    /// Reverse-FQDN group — a manifest pkgref is always qualified.
    pub group: Group,
    pub name: String,
    /// The `[workspace.versions]` placeholder name this dependency references.
    pub var: String,
}

// ---------------------------------------------------------------------------
// Wire types for `Requires` — private; reached only via Serialize / Deserialize.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequiresWire {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    packages: BTreeMap<String, RequiresPackageEntryWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    capabilities: Vec<CapabilityRef>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum RequiresPackageEntryWire {
    /// Bare constraint string: `"^0.3"`, `"=1.0"`, `"*"`.
    Constraint(String),
    /// Inline-table: registry-resolved with options OR git-source.
    Inline(InlinePackageDepWire),
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct InlinePackageDepWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<VersionFieldWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    git: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rev: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    auth: Option<AuthKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token_env: Option<String>,
    /// Inclusion type (PROP-009 §2.4). Valid on every source kind; lifted
    /// into `Requires::links` by the `TryFrom` conversion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    link: Option<LinkType>,
}

/// The `version` field of an inline `[requires.packages]` entry — either a
/// concrete constraint string or a `[workspace.versions]` placeholder
/// reference (`version.var = "core"`).
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum VersionFieldWire {
    /// `version = "^0.3"` — a concrete constraint.
    Constraint(String),
    /// `version.var = "core"` — a `[workspace.versions]` placeholder.
    Var { var: String },
}

impl From<Requires> for RequiresWire {
    fn from(r: Requires) -> Self {
        let mut packages: BTreeMap<String, RequiresPackageEntryWire> = BTreeMap::new();
        for p in &r.packages {
            let key = wire_key(p.kind, p.group.as_ref(), &p.name);
            let link = p
                .group
                .as_ref()
                .and_then(|g| r.links.get(&link_key(g, &p.name)).copied());
            let constraint = version_spec_to_constraint_str(&p.version);
            // A registry dep carrying a declared `link` cannot use the
            // bare constraint-string form — it must round-trip as an
            // inline table so the `link` field has somewhere to live.
            let value = match link {
                Some(link) => RequiresPackageEntryWire::Inline(InlinePackageDepWire {
                    version: Some(VersionFieldWire::Constraint(constraint)),
                    link: Some(link),
                    ..Default::default()
                }),
                None => RequiresPackageEntryWire::Constraint(constraint),
            };
            packages.insert(key, value);
        }
        for g in &r.git_packages {
            let key = wire_key(g.kind, Some(&g.group), &g.name);
            let inline = InlinePackageDepWire {
                version: g
                    .version
                    .as_ref()
                    .map(|v| VersionFieldWire::Constraint(version_spec_to_constraint_str(v))),
                path: None,
                git: Some(g.url.clone()),
                tag: match &g.ref_kind {
                    GitRefKind::Tag(s) => Some(s.clone()),
                    _ => None,
                },
                branch: match &g.ref_kind {
                    GitRefKind::Branch(s) => Some(s.clone()),
                    _ => None,
                },
                rev: match &g.ref_kind {
                    GitRefKind::Rev(s) => Some(s.clone()),
                    _ => None,
                },
                auth: if g.auth == AuthKind::None {
                    None
                } else {
                    Some(g.auth)
                },
                token_env: g.token_env.clone(),
                link: r.links.get(&link_key(&g.group, &g.name)).copied(),
            };
            packages.insert(key, RequiresPackageEntryWire::Inline(inline));
        }
        for p in &r.path_packages {
            let key = wire_key(p.kind, Some(&p.group), &p.name);
            let inline = InlinePackageDepWire {
                version: p
                    .version
                    .as_ref()
                    .map(|v| VersionFieldWire::Constraint(version_spec_to_constraint_str(v))),
                path: Some(p.path.clone()),
                link: r.links.get(&link_key(&p.group, &p.name)).copied(),
                ..Default::default()
            };
            packages.insert(key, RequiresPackageEntryWire::Inline(inline));
        }
        for v in &r.var_packages {
            let key = wire_key(v.kind, Some(&v.group), &v.name);
            let inline = InlinePackageDepWire {
                version: Some(VersionFieldWire::Var { var: v.var.clone() }),
                link: r.links.get(&link_key(&v.group, &v.name)).copied(),
                ..Default::default()
            };
            packages.insert(key, RequiresPackageEntryWire::Inline(inline));
        }
        RequiresWire {
            packages,
            capabilities: r.capabilities,
        }
    }
}

impl TryFrom<RequiresWire> for Requires {
    type Error = String;

    fn try_from(w: RequiresWire) -> std::result::Result<Self, Self::Error> {
        let mut packages: Vec<PackageRef> = Vec::new();
        let mut git_packages: Vec<GitPackageDep> = Vec::new();
        let mut path_packages: Vec<PathPackageDep> = Vec::new();
        let mut var_packages: Vec<VarRegistryDep> = Vec::new();
        let mut links: BTreeMap<String, LinkType> = BTreeMap::new();
        for (key, entry) in w.packages {
            let (kind, group, name) = parse_pkgref_key(&key).map_err(|e| e.to_string())?;
            match entry {
                RequiresPackageEntryWire::Constraint(spec_str) => {
                    let version = VersionSpec::parse(&spec_str).map_err(|e| e.to_string())?;
                    packages.push(
                        PackageRef::new(kind, Some(group), name, version)
                            .map_err(|e| e.to_string())?,
                    );
                }
                RequiresPackageEntryWire::Inline(inline) => {
                    // Record the consumer's `link` declaration (PROP-009
                    // §2.4) before the source-kind dispatch — `link` is
                    // valid on every source kind. Every declared value is
                    // stored, an explicit `static` included: writing
                    // `link = "static"` overrides a workspace
                    // `[boot].default_link` / a package-suggested link, and
                    // that intent is lost if explicit `static` is dropped.
                    if let Some(link) = inline.link {
                        links.insert(link_key(&group, &name), link);
                    }
                    // Dispatch on source-kind: path wins over git wins over
                    // registry. A registry-resolved entry whose version is a
                    // `[workspace.versions]` placeholder is held in var_packages
                    // for the workspace loader to resolve. Each `inline_to_*`
                    // rejects fields belonging to a different source-kind.
                    if inline.path.is_some() {
                        path_packages.push(
                            inline_to_path_dep(kind, group, name, inline)
                                .map_err(|e| e.to_string())?,
                        );
                    } else if inline.git.is_some() {
                        git_packages.push(
                            inline_to_git_dep(kind, group, name, inline)
                                .map_err(|e| e.to_string())?,
                        );
                    } else if matches!(inline.version, Some(VersionFieldWire::Var { .. })) {
                        var_packages.push(
                            inline_to_var_dep(kind, group, name, inline)
                                .map_err(|e| e.to_string())?,
                        );
                    } else {
                        packages.push(
                            inline_to_registry_pkgref(kind, group, name, inline)
                                .map_err(|e| e.to_string())?,
                        );
                    }
                }
            }
        }
        // Defence-in-depth: one `(group, name)` cannot land in two buckets.
        // The wire form is a single TOML table with unique keys, so this is
        // unreachable from a valid manifest — kept against a future wire shape.
        let mut seen: std::collections::HashSet<(Group, String)> = std::collections::HashSet::new();
        for (group, name) in packages
            .iter()
            .filter_map(|p| p.group.clone().map(|g| (g, p.name.clone())))
            .chain(
                git_packages
                    .iter()
                    .map(|g| (g.group.clone(), g.name.clone())),
            )
            .chain(
                path_packages
                    .iter()
                    .map(|p| (p.group.clone(), p.name.clone())),
            )
            .chain(
                var_packages
                    .iter()
                    .map(|v| (v.group.clone(), v.name.clone())),
            )
        {
            let label = link_key(&group, &name);
            if !seen.insert((group, name)) {
                return Err(format!("dependency `{label}` declared more than once"));
            }
        }
        Ok(Requires {
            packages,
            capabilities: w.capabilities,
            git_packages,
            path_packages,
            var_packages,
            links,
        })
    }
}

fn parse_pkgref_key(key: &str) -> Result<(Option<PackageKind>, Group, String)> {
    if key.contains('@') {
        return Err(Error::BadDependencyDecl {
            input: key.to_string(),
            reason: "version constraint must be the value, not part of the key".to_string(),
        });
    }
    let pr = PackageRef::parse(key)?;
    let group = pr.group.ok_or_else(|| Error::BadDependencyDecl {
        input: key.to_string(),
        reason: "a manifest dependency must be group-qualified — write `<group>/<name>`"
            .to_string(),
    })?;
    Ok((pr.kind, group, pr.name))
}

fn inline_to_registry_pkgref(
    kind: Option<PackageKind>,
    group: Group,
    name: String,
    inline: InlinePackageDepWire,
) -> Result<PackageRef> {
    let key_for_err = link_key(&group, &name);
    if inline.tag.is_some() || inline.branch.is_some() || inline.rev.is_some() {
        return Err(Error::BadDependencyDecl {
            input: key_for_err,
            reason: "registry-resolved dep cannot specify `tag`/`branch`/`rev` without `git`"
                .to_string(),
        });
    }
    if inline.auth.is_some() || inline.token_env.is_some() {
        return Err(Error::BadDependencyDecl {
            input: key_for_err,
            reason: "registry-resolved dep cannot specify `auth`/`token_env` without `git`"
                .to_string(),
        });
    }
    let version = match inline.version {
        Some(VersionFieldWire::Constraint(s)) => VersionSpec::parse(&s)?,
        Some(VersionFieldWire::Var { .. }) => {
            unreachable!("a `version.var` entry is dispatched to var_packages")
        }
        None => VersionSpec::Latest,
    };
    PackageRef::new(kind, Some(group), name, version)
}

fn inline_to_git_dep(
    kind: Option<PackageKind>,
    group: Group,
    name: String,
    inline: InlinePackageDepWire,
) -> Result<GitPackageDep> {
    let key_for_err = link_key(&group, &name);
    let url = inline.git.expect("caller checked git is Some");
    let ref_kind = match (inline.tag, inline.branch, inline.rev) {
        (Some(t), None, None) => GitRefKind::Tag(t),
        (None, Some(b), None) => GitRefKind::Branch(b),
        (None, None, Some(r)) => GitRefKind::Rev(r),
        (None, None, None) => {
            return Err(Error::BadDependencyDecl {
                input: key_for_err,
                reason: "git-source requires exactly one of `tag`, `branch`, `rev`".to_string(),
            });
        }
        _ => {
            return Err(Error::BadDependencyDecl {
                input: key_for_err,
                reason: "git-source must specify exactly one of `tag`/`branch`/`rev`, not several"
                    .to_string(),
            });
        }
    };
    let version = constraint_only_version(&key_for_err, inline.version, "a git-source dependency")?;
    Ok(GitPackageDep {
        kind,
        group,
        name,
        url,
        ref_kind,
        version,
        auth: inline.auth.unwrap_or_default(),
        token_env: inline.token_env,
    })
}

fn inline_to_path_dep(
    kind: Option<PackageKind>,
    group: Group,
    name: String,
    inline: InlinePackageDepWire,
) -> Result<PathPackageDep> {
    let key_for_err = link_key(&group, &name);
    let path = inline.path.expect("caller checked path is Some");
    if inline.git.is_some()
        || inline.tag.is_some()
        || inline.branch.is_some()
        || inline.rev.is_some()
    {
        return Err(Error::BadDependencyDecl {
            input: key_for_err,
            reason: "path-source dep cannot also specify `git`/`tag`/`branch`/`rev`".to_string(),
        });
    }
    if inline.auth.is_some() || inline.token_env.is_some() {
        return Err(Error::BadDependencyDecl {
            input: key_for_err,
            reason: "path-source dep cannot specify `auth`/`token_env` — the source is local"
                .to_string(),
        });
    }
    let version =
        constraint_only_version(&key_for_err, inline.version, "a path-source dependency")?;
    Ok(PathPackageDep {
        kind,
        group,
        name,
        path,
        version,
    })
}

fn inline_to_var_dep(
    kind: Option<PackageKind>,
    group: Group,
    name: String,
    inline: InlinePackageDepWire,
) -> Result<VarRegistryDep> {
    let key_for_err = link_key(&group, &name);
    if inline.git.is_some()
        || inline.path.is_some()
        || inline.tag.is_some()
        || inline.branch.is_some()
        || inline.rev.is_some()
        || inline.auth.is_some()
        || inline.token_env.is_some()
    {
        return Err(Error::BadDependencyDecl {
            input: key_for_err,
            reason: "a `version.var` dependency is registry-resolved — it cannot carry \
                     `git`/`path`/`tag`/`branch`/`rev`/`auth`/`token_env`"
                .to_string(),
        });
    }
    let var = match inline.version {
        Some(VersionFieldWire::Var { var }) => var,
        _ => unreachable!("caller checked version is a Var"),
    };
    Ok(VarRegistryDep {
        kind,
        group,
        name,
        var,
    })
}

/// Extract an optional concrete [`VersionSpec`] from a wire `version` field,
/// rejecting a `version.var` placeholder — placeholders are supported only on
/// registry-resolved dependencies (PROP-007 §2.6), not on `source` declares.
fn constraint_only_version(
    key_for_err: &str,
    field: Option<VersionFieldWire>,
    source_kind: &str,
) -> Result<Option<VersionSpec>> {
    match field {
        None => Ok(None),
        Some(VersionFieldWire::Constraint(s)) => Ok(Some(VersionSpec::parse(&s)?)),
        Some(VersionFieldWire::Var { .. }) => Err(Error::BadDependencyDecl {
            input: key_for_err.to_string(),
            reason: format!(
                "`version.var` is supported only on registry-resolved dependencies, not on \
                 {source_kind}"
            ),
        }),
    }
}

/// The `[requires.packages]` table key for a dependency — the canonical
/// version-less pkgref `[<kind>:]<group>/<name>` (PROP-008 §2.4 / §2.6).
fn wire_key(kind: Option<PackageKind>, group: Option<&Group>, name: &str) -> String {
    let base = match group {
        Some(g) => format!("{g}/{name}"),
        None => name.to_string(),
    };
    match kind {
        Some(k) => format!("{k}:{base}"),
        None => base,
    }
}

/// The `Requires::links` map key — the kind-free `<group>/<name>` identity
/// (PROP-008 §2.2). Version-independent, so it survives the
/// `var_packages` → `packages` resolution the workspace loader performs.
fn link_key(group: &Group, name: &str) -> String {
    format!("{group}/{name}")
}

fn version_spec_to_constraint_str(spec: &VersionSpec) -> String {
    match spec {
        VersionSpec::Latest => "*".to_string(),
        VersionSpec::Req(req) => req.to_string(),
    }
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

/// `[features]` table — feature definitions per PROP-003 §2.4.
///
/// Each feature maps to a list of activation strings; the strings can
/// be other feature names, dep-references (`dep:foo`, `foo?/feat`), or
/// subskill-references (`subskill:<path>`). The TOML form is a mix of
/// flat string-list keys plus a nested `exclusive` table; we deserialise
/// both via a manual visitor so the public API stays clean.
///
/// ```toml
/// [features]
/// default = ["wal-protocol"]
/// wal-protocol = []
/// rust-stack = ["subskill:stack/rust"]
/// python-stack = ["subskill:stack/python"]
///
/// [features.exclusive]
/// stacks = ["rust-stack", "python-stack"]
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeaturesTable {
    /// `feature-name` → list of activation strings.
    pub features: BTreeMap<String, Vec<String>>,
    /// `[features.exclusive]` — at-most-one named groups.
    pub exclusive: BTreeMap<String, Vec<String>>,
}

impl FeaturesTable {
    pub fn is_empty(&self) -> bool {
        self.features.is_empty() && self.exclusive.is_empty()
    }

    /// Convenience — list of features active by default
    /// (the `default` feature's activation list, if present).
    pub fn defaults(&self) -> &[String] {
        self.features
            .get("default")
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Look up a feature's activation list.
    pub fn get(&self, name: &str) -> Option<&[String]> {
        self.features.get(name).map(|v| v.as_slice())
    }
}

impl Serialize for FeaturesTable {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut total = self.features.len();
        if !self.exclusive.is_empty() {
            total += 1;
        }
        let mut m = s.serialize_map(Some(total))?;
        for (k, v) in &self.features {
            m.serialize_entry(k, v)?;
        }
        if !self.exclusive.is_empty() {
            m.serialize_entry("exclusive", &self.exclusive)?;
        }
        m.end()
    }
}

impl<'de> Deserialize<'de> for FeaturesTable {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // Receive as a generic `BTreeMap<String, toml::Value>` then split
        // into features (string lists) and the special `exclusive` table.
        let raw: BTreeMap<String, toml::Value> = BTreeMap::deserialize(d)?;
        let mut features: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut exclusive: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (k, v) in raw {
            if k == "exclusive" {
                let table: BTreeMap<String, Vec<String>> =
                    v.try_into().map_err(serde::de::Error::custom)?;
                exclusive = table;
                continue;
            }
            let arr: Vec<String> = v.try_into().map_err(serde::de::Error::custom)?;
            features.insert(k, arr);
        }
        Ok(FeaturesTable {
            features,
            exclusive,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse a bare `Requires` from a TOML body whose top-level keys are
    /// `packages` / `capabilities` (i.e. the inside of a `[requires]` table).
    fn requires_from_toml(body: &str) -> Requires {
        toml::from_str(body).unwrap()
    }

    /// The canonical group every fixture package in these tests belongs to.
    fn org() -> Group {
        Group::parse("org.vibevm").unwrap()
    }

    #[test]
    fn publish_posture_default_is_all_true() {
        assert!(PublishPosture::default().is_default());
        assert!(!PublishPosture::default().is_never());
        assert!(PublishPosture::default().includes("anything"));
    }

    #[test]
    fn publish_posture_roundtrips_all_forms() {
        // `publish = false`
        let never: PublishPosture = toml::from_str("v = false").map(|w: Wrap| w.v).unwrap();
        assert!(never.is_never());
        assert!(!never.includes("vibespecs"));
        // `publish = true`
        let all: PublishPosture = toml::from_str("v = true").map(|w: Wrap| w.v).unwrap();
        assert!(all.is_default());
        // `publish = ["a", "b"]`
        let some: PublishPosture = toml::from_str("v = [\"a\", \"b\"]")
            .map(|w: Wrap| w.v)
            .unwrap();
        assert!(some.includes("a"));
        assert!(some.includes("b"));
        assert!(!some.includes("c"));
        assert!(!some.is_never());
    }

    #[derive(Deserialize)]
    struct Wrap {
        v: PublishPosture,
    }

    #[test]
    fn compatibility_is_empty() {
        assert!(Compatibility::default().is_empty());
        let c = Compatibility {
            min_vibe_version: Some("0.1.0".into()),
            requires_kinds: vec![],
        };
        assert!(!c.is_empty());
    }

    #[test]
    fn requires_map_bare_constraint_parses() {
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm/wal" = "^0.3"
"org.vibevm/auth" = "*"
"#,
        );
        assert_eq!(r.packages.len(), 2);
        assert!(r.git_packages.is_empty());
        // BTreeMap ordering: org.vibevm/auth < org.vibevm/wal alphabetically.
        assert_eq!(r.packages[0].qualified_name(), "org.vibevm/auth");
        assert_eq!(r.packages[1].qualified_name(), "org.vibevm/wal");
    }

    #[test]
    fn requires_inline_table_with_version_parses() {
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm/wal" = { version = "^0.3" }
"#,
        );
        assert_eq!(r.packages.len(), 1);
        assert_eq!(r.packages[0].qualified_name(), "org.vibevm/wal");
        assert!(r.git_packages.is_empty());
    }

    #[test]
    fn git_source_with_tag_parses() {
        // A kind-prefixed manifest key — the prefix is optional (PROP-008
        // §2.4) and, when present, is parsed onto the dependency.
        let r = requires_from_toml(
            r#"[packages]
"flow:org.vibevm/internal" = { git = "https://github.com/me/flow-internal", tag = "v0.1.0" }
"#,
        );
        assert!(r.packages.is_empty());
        assert_eq!(r.git_packages.len(), 1);
        let g = &r.git_packages[0];
        assert_eq!(g.kind, Some(PackageKind::Flow));
        assert_eq!(g.group, org());
        assert_eq!(g.name, "internal");
        assert_eq!(g.url, "https://github.com/me/flow-internal");
        assert!(matches!(&g.ref_kind, GitRefKind::Tag(t) if t == "v0.1.0"));
        assert_eq!(g.ref_kind.label(), "tag");
        assert!(g.version.is_none());
        assert_eq!(g.auth, AuthKind::None);
    }

    #[test]
    fn git_source_with_branch_and_rev_parse() {
        let b = requires_from_toml(
            r#"[packages]
"org.vibevm/experimental" = { git = "https://github.com/x/y", branch = "main" }
"#,
        );
        assert!(matches!(&b.git_packages[0].ref_kind, GitRefKind::Branch(s) if s == "main"));
        let v = requires_from_toml(
            r#"[packages]
"org.vibevm/fork" = { git = "https://github.com/x/y", rev = "abc12345" }
"#,
        );
        assert!(matches!(&v.git_packages[0].ref_kind, GitRefKind::Rev(s) if s == "abc12345"));
    }

    #[test]
    fn git_source_with_auth_and_version_parse() {
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm/secret" = { git = "https://gitlab.acme.example/x/y", tag = "v1.0", auth = "token-env", token_env = "MY_TOKEN", version = "^1.0" }
"#,
        );
        let g = &r.git_packages[0];
        assert_eq!(g.auth, AuthKind::TokenEnv);
        assert_eq!(g.token_env.as_deref(), Some("MY_TOKEN"));
        assert!(g.version.is_some());
    }

    #[test]
    fn git_source_rejects_no_ref_and_multiple_refs() {
        let no_ref = toml::from_str::<Requires>(
            r#"[packages]
"org.vibevm/bad" = { git = "https://x/y" }
"#,
        )
        .unwrap_err();
        assert!(no_ref.to_string().contains("requires exactly one of"));
        let multi = toml::from_str::<Requires>(
            r#"[packages]
"org.vibevm/bad" = { git = "https://x/y", tag = "v1", branch = "main" }
"#,
        )
        .unwrap_err();
        assert!(multi.to_string().contains("exactly one of"));
    }

    #[test]
    fn registry_inline_rejects_git_fields() {
        let err = toml::from_str::<Requires>(
            r#"[packages]
"org.vibevm/bad" = { version = "^0.3", tag = "v1" }
"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("without `git`"));
    }

    #[test]
    fn rejects_at_in_pkgref_key() {
        let err = toml::from_str::<Requires>(
            r#"[packages]
"org.vibevm/wal@^0.3" = "*"
"#,
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("must be the value, not part of the key")
        );
    }

    #[test]
    fn path_source_parses() {
        let r = requires_from_toml(
            r#"[packages]
"flow:org.vibevm/wal" = { path = "../flow-wal" }
"#,
        );
        assert!(r.packages.is_empty());
        assert!(r.git_packages.is_empty());
        assert_eq!(r.path_packages.len(), 1);
        let p = &r.path_packages[0];
        assert_eq!(p.kind, Some(PackageKind::Flow));
        assert_eq!(p.group, org());
        assert_eq!(p.name, "wal");
        assert_eq!(p.path, "../flow-wal");
        assert!(p.version.is_none());
    }

    #[test]
    fn path_source_dual_form_parses() {
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm/wal" = { path = "../flow-wal", version = "^0.1" }
"#,
        );
        assert_eq!(r.path_packages.len(), 1);
        assert!(r.path_packages[0].version.is_some());
    }

    #[test]
    fn path_source_rejects_git_alongside() {
        let err = toml::from_str::<Requires>(
            r#"[packages]
"org.vibevm/bad" = { path = "../x", git = "https://x/y" }
"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("cannot also specify"), "{err}");
    }

    #[test]
    fn path_source_round_trips() {
        let original = requires_from_toml(
            r#"[packages]
"org.vibevm/wal" = { path = "../flow-wal", version = "^0.1" }
"org.vibevm/auth" = { path = "../feat-auth" }
"#,
        );
        let rendered = toml::to_string_pretty(&original).unwrap();
        let back: Requires = toml::from_str(&rendered).unwrap();
        assert_eq!(original, back);
        assert_eq!(back.path_packages.len(), 2);
    }

    #[test]
    fn version_var_parses() {
        let r = requires_from_toml(
            r#"[packages]
"flow:org.vibevm/wal" = { version.var = "core" }
"#,
        );
        assert!(r.packages.is_empty());
        assert!(r.git_packages.is_empty());
        assert!(r.path_packages.is_empty());
        assert_eq!(r.var_packages.len(), 1);
        let v = &r.var_packages[0];
        assert_eq!(v.kind, Some(PackageKind::Flow));
        assert_eq!(v.group, org());
        assert_eq!(v.name, "wal");
        assert_eq!(v.var, "core");
    }

    #[test]
    fn version_var_round_trips() {
        let original = requires_from_toml(
            r#"[packages]
"org.vibevm/wal" = { version.var = "core" }
"org.vibevm/auth" = "^0.2"
"#,
        );
        let rendered = toml::to_string_pretty(&original).unwrap();
        let back: Requires = toml::from_str(&rendered).unwrap();
        assert_eq!(original, back);
        assert_eq!(back.var_packages.len(), 1);
        assert_eq!(back.packages.len(), 1);
    }

    #[test]
    fn version_var_rejected_on_git_source() {
        let err = toml::from_str::<Requires>(
            r#"[packages]
"org.vibevm/bad" = { git = "https://x/y", tag = "v1", version.var = "core" }
"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("version.var"), "{err}");
    }

    #[test]
    fn version_var_rejects_extra_fields() {
        let err = toml::from_str::<Requires>(
            r#"[packages]
"org.vibevm/bad" = { version.var = "core", tag = "v1" }
"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("registry-resolved"), "{err}");
    }

    #[test]
    fn requires_round_trips_through_serialize() {
        let original = requires_from_toml(
            r#"capabilities = ["db:any@>=1.0"]

[packages]
"flow:org.vibevm/internal" = { git = "https://github.com/me/flow-internal", tag = "v0.1.0", auth = "token-env", token_env = "MY" }
"org.vibevm/wal" = "^0.3"
"#,
        );
        let rendered = toml::to_string_pretty(&original).unwrap();
        let back: Requires = toml::from_str(&rendered).unwrap();
        assert_eq!(back.packages.len(), 1);
        assert_eq!(back.git_packages.len(), 1);
        assert_eq!(back.git_packages[0].name, "internal");
        assert_eq!(back.capabilities.len(), 1);
        assert_eq!(original, back);
    }

    #[test]
    fn package_meta_as_package_ref_pins_exact() {
        let meta = PackageMeta {
            name: "wal".into(),
            group: Group::parse("org.vibevm").unwrap(),
            kind: PackageKind::Flow,
            version: semver::Version::parse("0.3.0").unwrap(),
            authors: vec![],
            license: None,
            description: None,
            homepage: None,
            keywords: vec![],
            describes: None,
            publish: PublishPosture::default(),
        };
        let r = meta.as_package_ref().unwrap();
        assert_eq!(r.kind, Some(PackageKind::Flow));
        assert_eq!(r.group, Some(org()));
        assert_eq!(r.name, "wal");
        assert!(r.version.matches(&semver::Version::parse("0.3.0").unwrap()));
        assert!(!r.version.matches(&semver::Version::parse("0.3.1").unwrap()));
    }

    #[test]
    fn features_table_roundtrips() {
        let raw = r#"
default = ["wal-protocol"]
wal-protocol = []
rust-stack = ["subskill:stack/rust"]

[exclusive]
stacks = ["rust-stack", "python-stack"]
"#;
        let ft: FeaturesTable = toml::from_str(raw).unwrap();
        assert_eq!(ft.defaults(), &["wal-protocol".to_string()]);
        assert_eq!(ft.get("rust-stack").unwrap().len(), 1);
        assert_eq!(ft.exclusive.get("stacks").unwrap().len(), 2);
        let rendered = toml::to_string_pretty(&ft).unwrap();
        let back: FeaturesTable = toml::from_str(&rendered).unwrap();
        assert_eq!(ft, back);
    }

    // --- PROP-009 §2.4 / §2.5 — inclusion type + boot category ----------

    #[test]
    fn link_type_default_is_static() {
        assert_eq!(LinkType::default(), LinkType::Static);
    }

    #[test]
    fn requires_link_on_registry_dep_parses() {
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm/wal" = { version = "^0.3", link = "inline" }
"#,
        );
        assert_eq!(r.packages.len(), 1);
        assert_eq!(r.link_for(&org(), "wal"), LinkType::Inline);
    }

    #[test]
    fn requires_link_dynamic_parses() {
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm/rust" = { version = "^2.0", link = "dynamic" }
"#,
        );
        assert_eq!(r.link_for(&org(), "rust"), LinkType::Dynamic);
    }

    #[test]
    fn requires_link_absent_is_static() {
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm/wal" = "^0.3"
"#,
        );
        assert!(r.links.is_empty());
        assert_eq!(r.link_for(&org(), "wal"), LinkType::Static);
    }

    #[test]
    fn requires_explicit_static_link_is_stored() {
        // An explicit `link = "static"` is kept, not folded into "absent":
        // the loading-model precedence (PROP-009 §2.4) lets it override a
        // workspace default, so the explicit choice must survive — and it
        // survives a serialize round-trip as an inline table.
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm/wal" = { version = "^0.3", link = "static" }
"#,
        );
        assert_eq!(r.declared_link(&org(), "wal"), Some(LinkType::Static));
        assert_eq!(r.link_for(&org(), "wal"), LinkType::Static);
        let back: Requires = toml::from_str(&toml::to_string_pretty(&r).unwrap()).unwrap();
        assert_eq!(back.declared_link(&org(), "wal"), Some(LinkType::Static));
    }

    #[test]
    fn requires_declared_link_is_none_when_unspecified() {
        // A bare entry declares no `link` — `declared_link` is `None`,
        // while `link_for` applies the `static` default.
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm/wal" = "^0.3"
"#,
        );
        assert_eq!(r.declared_link(&org(), "wal"), None);
        assert_eq!(r.link_for(&org(), "wal"), LinkType::Static);
    }

    #[test]
    fn requires_link_on_git_source_parses() {
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm/internal" = { git = "https://github.com/me/flow-internal", tag = "v0.1.0", link = "dynamic" }
"#,
        );
        assert_eq!(r.git_packages.len(), 1);
        assert_eq!(r.link_for(&org(), "internal"), LinkType::Dynamic);
    }

    #[test]
    fn requires_link_on_path_source_parses() {
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm/wal" = { path = "../flow-wal", link = "inline" }
"#,
        );
        assert_eq!(r.path_packages.len(), 1);
        assert_eq!(r.link_for(&org(), "wal"), LinkType::Inline);
    }

    #[test]
    fn requires_link_on_var_dep_parses() {
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm/wal" = { version.var = "core", link = "dynamic" }
"#,
        );
        assert_eq!(r.var_packages.len(), 1);
        assert_eq!(r.link_for(&org(), "wal"), LinkType::Dynamic);
    }

    #[test]
    fn requires_link_rejects_unknown_value() {
        let err = toml::from_str::<Requires>(
            r#"[packages]
"org.vibevm/wal" = { version = "^0.3", link = "weird" }
"#,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("variant") || err.to_string().contains("link"),
            "{err}"
        );
    }

    #[test]
    fn requires_registry_link_renders_as_inline_table() {
        // A registry dep with a non-default link cannot use the bare-string
        // form — it must serialise as an inline table so `link` survives.
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm/wal" = { version = "^0.3", link = "inline" }
"#,
        );
        let rendered = toml::to_string_pretty(&r).unwrap();
        assert!(rendered.contains("link = \"inline\""), "{rendered}");
    }

    #[test]
    fn requires_link_round_trips_across_all_source_kinds() {
        let original = requires_from_toml(
            r#"[packages]
"org.vibevm/wal" = { version = "^0.3", link = "inline" }
"org.vibevm/internal" = { git = "https://github.com/me/flow-internal", tag = "v0.1.0", link = "dynamic" }
"org.vibevm/auth" = { path = "../feat-auth", link = "dynamic" }
"org.vibevm/rust" = { version.var = "core", link = "inline" }
"org.vibevm/plain" = "^0.1"
"#,
        );
        let rendered = toml::to_string_pretty(&original).unwrap();
        let back: Requires = toml::from_str(&rendered).unwrap();
        assert_eq!(original, back);
        // Four declared links survive; the bare entry stays implicitly static.
        assert_eq!(back.links.len(), 4);
        assert_eq!(back.link_for(&org(), "wal"), LinkType::Inline);
        assert_eq!(back.link_for(&org(), "internal"), LinkType::Dynamic);
        assert_eq!(back.link_for(&org(), "auth"), LinkType::Dynamic);
        assert_eq!(back.link_for(&org(), "rust"), LinkType::Inline);
        assert_eq!(back.link_for(&org(), "plain"), LinkType::Static);
    }

    #[test]
    fn boot_snippet_parses_category_and_link() {
        let bs: BootSnippet = toml::from_str(
            r#"source = "boot/10-flow-wal.md"
category = "flow"
link = "inline"
"#,
        )
        .unwrap();
        assert_eq!(bs.category, Some(BootCategory::Flow));
        assert_eq!(bs.link, Some(LinkType::Inline));
    }

    #[test]
    fn boot_snippet_minimal_form_parses() {
        // `source` is the only required field; `category`, `link`, and
        // `when` are optional and absent here.
        let bs: BootSnippet = toml::from_str("source = \"boot/10-flow-wal.md\"\n").unwrap();
        assert!(bs.category.is_none());
        assert!(bs.link.is_none());
        assert!(bs.when.is_none());
    }

    #[test]
    fn boot_category_user_override_is_kebab_case() {
        let bs: BootSnippet = toml::from_str(
            r#"source = "boot/90-user.md"
category = "user-override"
"#,
        )
        .unwrap();
        assert_eq!(bs.category, Some(BootCategory::UserOverride));
    }

    #[test]
    fn boot_snippet_round_trips_with_category_and_link() {
        let bs: BootSnippet = toml::from_str(
            r#"source = "boot/20-stack-rust.md"
category = "stack"
link = "dynamic"
"#,
        )
        .unwrap();
        let rendered = toml::to_string_pretty(&bs).unwrap();
        let back: BootSnippet = toml::from_str(&rendered).unwrap();
        assert_eq!(bs, back);
    }

    // --- PROP-009 §2.4 / §2.6 — the `when` OS gate ----------------------

    #[test]
    fn when_condition_parses_each_supported_os() {
        use std::str::FromStr;
        assert_eq!(
            WhenCondition::from_str("os:windows").unwrap(),
            WhenCondition::Os(TargetOs::Windows)
        );
        assert_eq!(
            WhenCondition::from_str("os:macos").unwrap(),
            WhenCondition::Os(TargetOs::Macos)
        );
        assert_eq!(
            WhenCondition::from_str("os:linux").unwrap(),
            WhenCondition::Os(TargetOs::Linux)
        );
    }

    #[test]
    fn when_condition_display_round_trips_through_from_str() {
        use std::str::FromStr;
        for cond in [
            WhenCondition::Os(TargetOs::Windows),
            WhenCondition::Os(TargetOs::Macos),
            WhenCondition::Os(TargetOs::Linux),
        ] {
            assert_eq!(WhenCondition::from_str(&cond.to_string()).unwrap(), cond);
        }
        assert_eq!(WhenCondition::Os(TargetOs::Linux).to_string(), "os:linux");
    }

    #[test]
    fn when_condition_rejects_an_unrecognised_prefix() {
        // The `os:` namespace is the only one v1 understands — a bare
        // probe name from the wider `[activation]` vocabulary is rejected
        // until that engine lands.
        let err = "rust".parse::<WhenCondition>().unwrap_err();
        assert!(err.to_string().contains("expected `os:<name>`"), "{err}");
    }

    #[test]
    fn when_condition_rejects_an_unknown_os() {
        let err = "os:winows".parse::<WhenCondition>().unwrap_err();
        // The diagnostic names the full condition and the bad OS.
        assert!(err.to_string().contains("os:winows"), "{err}");
        assert!(err.to_string().contains("winows"), "{err}");
    }

    #[test]
    fn when_condition_matches_the_running_os() {
        // The test process runs on one of the supported OSes (CI: linux,
        // dev: windows); the matching condition holds and a different one
        // does not.
        let current = TargetOs::current().expect("test host is a supported OS");
        assert!(WhenCondition::Os(current).matches_current_os());
        let other = match current {
            TargetOs::Linux => TargetOs::Windows,
            TargetOs::Windows | TargetOs::Macos => TargetOs::Linux,
        };
        assert!(!WhenCondition::Os(other).matches_current_os());
    }

    #[test]
    fn boot_snippet_parses_when() {
        let bs: BootSnippet = toml::from_str(
            r#"source = "boot/win.md"
when = "os:windows"
"#,
        )
        .unwrap();
        assert_eq!(bs.when, Some(WhenCondition::Os(TargetOs::Windows)));
    }

    #[test]
    fn boot_snippet_rejects_a_malformed_when() {
        let err = toml::from_str::<BootSnippet>(
            r#"source = "boot/win.md"
when = "os:plan9"
"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("plan9"), "{err}");
    }

    #[test]
    fn boot_snippet_round_trips_with_when() {
        let bs: BootSnippet = toml::from_str(
            r#"source = "boot/mac.md"
category = "stack"
when = "os:macos"
"#,
        )
        .unwrap();
        let rendered = toml::to_string_pretty(&bs).unwrap();
        assert!(rendered.contains("when = \"os:macos\""), "{rendered}");
        let back: BootSnippet = toml::from_str(&rendered).unwrap();
        assert_eq!(bs, back);
    }
}
