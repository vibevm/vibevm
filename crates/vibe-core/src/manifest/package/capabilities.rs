//! The capability-based dependency vocabulary of a `[package]` —
//! `[provides]`, `[requires]`, `[[requires_any]]`, `[obsoletes]`,
//! `[conflicts]`, and the `[target."<predicate>"]` body. Split from
//! `package.rs` per the file-length budget (VIBEVM-SPEC §7.3,
//! PROP-002 §2.9). `Requires` round-trips through the sibling
//! `wire::RequiresWire` for its table form.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#git-source");

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::wire::RequiresWire;
use super::{GitPackageDep, LinkType, PathPackageDep, VarRegistryDep};
use crate::capability_ref::CapabilityRef;
use crate::package_ref::{Group, PackageRef};

/// `[provides]` — capabilities this package advertises.
///
/// ```
/// use vibe_core::manifest::Provides;
///
/// let p: Provides = toml::from_str(r#"capabilities = ["db:postgres@^15"]"#).unwrap();
/// assert_eq!(p.capabilities.len(), 1);
/// assert!(!p.is_empty());
/// assert!(Provides::default().is_empty());
/// ```
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
///
/// ```
/// use vibe_core::manifest::Requires;
///
/// let r: Requires = toml::from_str(r#"
///     capabilities = ["db:any@>=1.0"]
///     [packages]
///     "org.vibevm/wal" = "^0.3"
///     "org.vibevm/rust" = { version = "^2.0", link = "dynamic" }
/// "#).unwrap();
/// assert_eq!(r.packages.len(), 2);
/// assert_eq!(r.capabilities.len(), 1);
/// ```
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
pub(super) fn link_key(group: &Group, name: &str) -> String {
    format!("{group}/{name}")
}

/// `[[requires_any]]` — one entry per independent disjunction; `one_of` must
/// be satisfied by at least one of its alternatives.
///
/// ```
/// use vibe_core::manifest::RequiresAny;
///
/// let any: RequiresAny = toml::from_str(r#"one_of = ["feat:wal", "feat:journal"]"#).unwrap();
/// assert_eq!(any.one_of.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiresAny {
    pub one_of: Vec<PackageRef>,
}

/// `[obsoletes]` — packages this one supersedes.
///
/// ```
/// use vibe_core::manifest::Obsoletes;
///
/// let o: Obsoletes = toml::from_str(r#"packages = ["feat:old-wal"]"#).unwrap();
/// assert_eq!(o.packages.len(), 1);
/// assert!(!o.is_empty());
/// ```
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
///
/// ```
/// use vibe_core::manifest::ConflictsList;
///
/// let c: ConflictsList = toml::from_str(r#"packages = ["feat:rival-wal"]"#).unwrap();
/// assert!(!c.is_empty());
/// assert!(ConflictsList::default().is_empty());
/// ```
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
///
/// ```
/// use vibe_core::manifest::ConditionalTarget;
///
/// let t: ConditionalTarget = toml::from_str(r#"
///     [dependencies.packages]
///     "org.vibevm/wal" = "^0.3"
/// "#).unwrap();
/// assert_eq!(t.dependencies.packages.len(), 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionalTarget {
    #[serde(default, skip_serializing_if = "Requires::is_empty")]
    pub dependencies: Requires,
}
