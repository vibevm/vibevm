//! Non-registry dependency declarations a `[requires.packages]` entry can
//! carry — git-source (PROP-002 §2.4.1), path-source (PROP-007 §2.5), and
//! `version.var` placeholder (PROP-007 §2.6) — plus their conversions from
//! the wire form.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#git-source");

use crate::error::{Error, Result};
use crate::manifest::project::AuthKind;
use crate::package_ref::{Group, PackageKind, VersionSpec};

use super::link_key;
use super::wire::{InlinePackageDepWire, VersionFieldWire};

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

pub(super) fn inline_to_git_dep(
    kind: Option<PackageKind>,
    group: Group,
    name: String,
    url: String,
    inline: InlinePackageDepWire,
) -> Result<GitPackageDep> {
    let key_for_err = link_key(&group, &name);
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

pub(super) fn inline_to_path_dep(
    kind: Option<PackageKind>,
    group: Group,
    name: String,
    path: String,
    inline: InlinePackageDepWire,
) -> Result<PathPackageDep> {
    let key_for_err = link_key(&group, &name);
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

pub(super) fn inline_to_var_dep(
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::manifest::Requires;

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
}
