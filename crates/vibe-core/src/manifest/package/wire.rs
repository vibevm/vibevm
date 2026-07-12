//! Wire (serde) intermediates for `[requires]` — the TOML table form of
//! PROP-002 §2.4.1 / PROP-007 §2.6 / PROP-009 §2.4, reached only through
//! `Requires`'s `Serialize` / `Deserialize` (`into` / `try_from`).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#git-source");

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::capability_ref::CapabilityRef;
use crate::error::{Error, Result};
use crate::manifest::project::AuthKind;
use crate::package_ref::{Group, PackageKind, PackageRef, VersionSpec};

use super::capabilities::{Requires, link_key};
use super::deps::{inline_to_git_dep, inline_to_path_dep, inline_to_var_dep};
use super::{GitPackageDep, GitRefKind, LinkType, PathPackageDep, VarRegistryDep};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RequiresWire {
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
pub(super) struct InlinePackageDepWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) version: Option<VersionFieldWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) git: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) rev: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) auth: Option<AuthKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) token_env: Option<String>,
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
pub(super) enum VersionFieldWire {
    /// `version = "^0.3"` — a concrete constraint.
    Constraint(String),
    /// `version.var = "core"` — a `[workspace.versions]` placeholder.
    Var { var: String },
}

impl From<Requires> for RequiresWire {
    fn from(r: Requires) -> Self {
        let mut packages: BTreeMap<String, RequiresPackageEntryWire> = BTreeMap::new();
        for p in &r.packages {
            let key = wire_key(p.kind, p.group.as_ref(), p.name.as_str());
            let link = p
                .group
                .as_ref()
                .and_then(|g| r.links.get(&link_key(g, p.name.as_str())).copied());
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
                    // The discriminating field is taken out of the wire form
                    // and passed by value, so the callee never re-checks an
                    // Option the dispatch already proved.
                    let mut inline = inline;
                    if let Some(path) = inline.path.take() {
                        path_packages.push(
                            inline_to_path_dep(kind, group, name, path, inline)
                                .map_err(|e| e.to_string())?,
                        );
                    } else if let Some(url) = inline.git.take() {
                        git_packages.push(
                            inline_to_git_dep(kind, group, name, url, inline)
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
            .filter_map(|p| p.group.clone().map(|g| (g, p.name.to_string())))
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
    Ok((pr.kind, group, pr.name.to_string()))
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

fn version_spec_to_constraint_str(spec: &VersionSpec) -> String {
    match spec {
        VersionSpec::Latest => "*".to_string(),
        VersionSpec::Req(req) => req.to_string(),
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
    fn requires_map_bare_constraint_parses() {
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm.world/wal" = "^0.3"
"org.vibevm/auth" = "*"
"#,
        );
        assert_eq!(r.packages.len(), 2);
        assert!(r.git_packages.is_empty());
        // BTreeMap ordering: org.vibevm/auth < org.vibevm.world/wal alphabetically.
        assert_eq!(r.packages[0].qualified_name(), "org.vibevm/auth");
        assert_eq!(r.packages[1].qualified_name(), "org.vibevm.world/wal");
    }

    #[test]
    fn requires_inline_table_with_version_parses() {
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm.world/wal" = { version = "^0.3" }
"#,
        );
        assert_eq!(r.packages.len(), 1);
        assert_eq!(r.packages[0].qualified_name(), "org.vibevm.world/wal");
        assert!(r.git_packages.is_empty());
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
"org.vibevm.world/wal@^0.3" = "*"
"#,
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("must be the value, not part of the key")
        );
    }

    #[test]
    fn requires_round_trips_through_serialize() {
        let original = requires_from_toml(
            r#"capabilities = ["db:any@>=1.0"]

[packages]
"flow:org.vibevm/internal" = { git = "https://github.com/me/flow-internal", tag = "v0.1.0", auth = "token-env", token_env = "MY" }
"org.vibevm.world/wal" = "^0.3"
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

    // --- PROP-009 §2.4 — the `link` inclusion type on wire entries ------

    #[test]
    fn requires_link_on_registry_dep_parses() {
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm.world/wal" = { version = "^0.3", link = "inline" }
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
"org.vibevm.world/wal" = "^0.3"
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
"org.vibevm.world/wal" = { version = "^0.3", link = "static" }
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
"org.vibevm.world/wal" = "^0.3"
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
"org.vibevm.world/wal" = { path = "../flow-wal", link = "inline" }
"#,
        );
        assert_eq!(r.path_packages.len(), 1);
        assert_eq!(r.link_for(&org(), "wal"), LinkType::Inline);
    }

    #[test]
    fn requires_link_on_var_dep_parses() {
        let r = requires_from_toml(
            r#"[packages]
"org.vibevm.world/wal" = { version.var = "core", link = "dynamic" }
"#,
        );
        assert_eq!(r.var_packages.len(), 1);
        assert_eq!(r.link_for(&org(), "wal"), LinkType::Dynamic);
    }

    #[test]
    fn requires_link_rejects_unknown_value() {
        let err = toml::from_str::<Requires>(
            r#"[packages]
"org.vibevm.world/wal" = { version = "^0.3", link = "weird" }
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
"org.vibevm.world/wal" = { version = "^0.3", link = "inline" }
"#,
        );
        let rendered = toml::to_string_pretty(&r).unwrap();
        assert!(rendered.contains("link = \"inline\""), "{rendered}");
    }

    #[test]
    fn requires_link_round_trips_across_all_source_kinds() {
        let original = requires_from_toml(
            r#"[packages]
"org.vibevm.world/wal" = { version = "^0.3", link = "inline" }
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
}
