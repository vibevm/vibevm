//! Serde wire forms and conversions for the authored `[requires]` table.

use super::*;

impl From<Requires> for RequiresWire {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#OPTIONAL-DECLARATIONS"
    )]
    fn from(r: Requires) -> Self {
        let mut packages: BTreeMap<String, RequiresPackageEntryWire> = BTreeMap::new();
        for p in &r.packages {
            let key = wire_key(p.kind, p.group.as_ref(), p.name.as_str());
            let link = p
                .group
                .as_ref()
                .and_then(|g| r.links.get(&link_key(g, p.name.as_str())).copied());
            let (access, friend, exclude) = p.group.as_ref().map_or((None, None, None), |group| {
                visibility_wire(&r, group, p.name.as_str())
            });
            let configuration = p
                .group
                .as_ref()
                .and_then(|g| r.configurations.get(&link_key(g, p.name.as_str())))
                .cloned();
            let optional = p
                .group
                .as_ref()
                .and_then(|g| r.optional.get(&link_key(g, p.name.as_str())).copied());
            let registry = p
                .group
                .as_ref()
                .and_then(|g| r.registry_pins.get(&link_key(g, p.name.as_str())))
                .cloned();
            let constraint = version_spec_to_constraint_str(&p.version);
            // A registry dep carrying per-edge metadata needs the inline
            // form so every explicit declaration survives round-trip.
            let value = if registry.is_some()
                || link.is_some()
                || access.is_some()
                || friend.is_some()
                || exclude.is_some()
                || configuration.is_some()
                || optional.is_some()
            {
                RequiresPackageEntryWire::Inline(Box::new(InlinePackageDepWire {
                    version: Some(VersionFieldWire::Constraint(constraint)),
                    registry,
                    link,
                    access,
                    friend,
                    exclude,
                    configuration,
                    optional,
                    ..Default::default()
                }))
            } else {
                RequiresPackageEntryWire::Constraint(constraint)
            };
            packages.insert(key, value);
        }
        for g in &r.git_packages {
            let key = wire_key(g.kind, Some(&g.group), &g.name);
            let inline = InlinePackageDepWire {
                registry: None,
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
                access: r.accesses.get(&link_key(&g.group, &g.name)).copied(),
                friend: r.friend_flags.get(&link_key(&g.group, &g.name)).copied(),
                exclude: r.excludes.get(&link_key(&g.group, &g.name)).cloned(),
                configuration: r.configurations.get(&link_key(&g.group, &g.name)).cloned(),
                optional: r.optional.get(&link_key(&g.group, &g.name)).copied(),
            };
            packages.insert(key, RequiresPackageEntryWire::Inline(Box::new(inline)));
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
                access: r.accesses.get(&link_key(&p.group, &p.name)).copied(),
                friend: r.friend_flags.get(&link_key(&p.group, &p.name)).copied(),
                exclude: r.excludes.get(&link_key(&p.group, &p.name)).cloned(),
                configuration: r.configurations.get(&link_key(&p.group, &p.name)).cloned(),
                optional: r.optional.get(&link_key(&p.group, &p.name)).copied(),
                ..Default::default()
            };
            packages.insert(key, RequiresPackageEntryWire::Inline(Box::new(inline)));
        }
        for v in &r.var_packages {
            let key = wire_key(v.kind, Some(&v.group), &v.name);
            let inline = InlinePackageDepWire {
                version: Some(VersionFieldWire::Var { var: v.var.clone() }),
                registry: r.registry_pins.get(&link_key(&v.group, &v.name)).cloned(),
                link: r.links.get(&link_key(&v.group, &v.name)).copied(),
                access: r.accesses.get(&link_key(&v.group, &v.name)).copied(),
                friend: r.friend_flags.get(&link_key(&v.group, &v.name)).copied(),
                exclude: r.excludes.get(&link_key(&v.group, &v.name)).cloned(),
                configuration: r.configurations.get(&link_key(&v.group, &v.name)).cloned(),
                optional: r.optional.get(&link_key(&v.group, &v.name)).copied(),
                ..Default::default()
            };
            packages.insert(key, RequiresPackageEntryWire::Inline(Box::new(inline)));
        }
        RequiresWire {
            packages,
            capabilities: r.capabilities,
            remaps: r
                .remaps
                .iter()
                .map(|p| {
                    (
                        wire_key(p.kind, p.group.as_ref(), p.name.as_str()),
                        version_spec_to_constraint_str(&p.version),
                    )
                })
                .collect(),
        }
    }
}

impl TryFrom<RequiresWire> for Requires {
    type Error = String;

    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#OPTIONAL-DECLARATIONS"
    )]
    fn try_from(w: RequiresWire) -> std::result::Result<Self, Self::Error> {
        let mut packages: Vec<PackageRef> = Vec::new();
        let mut git_packages: Vec<GitPackageDep> = Vec::new();
        let mut path_packages: Vec<PathPackageDep> = Vec::new();
        let mut var_packages: Vec<VarRegistryDep> = Vec::new();
        let mut links: BTreeMap<String, LinkType> = BTreeMap::new();
        let mut accesses: BTreeMap<String, AccessLevel> = BTreeMap::new();
        let mut friend_flags: BTreeMap<String, bool> = BTreeMap::new();
        let mut excludes: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut configurations = BTreeMap::new();
        let mut optional = BTreeMap::new();
        let mut registry_pins = BTreeMap::new();
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
                    if let Some(pin) = &inline.registry {
                        if inline.path.is_some() || inline.git.is_some() {
                            return Err(format!(
                                "source_policy_source_conflict: registry pin on `{key}` cannot accompany path or git"
                            ));
                        }
                        registry_pins.insert(link_key(&group, &name), pin.clone());
                    }
                    if let Some(value) = inline.optional {
                        optional.insert(link_key(&group, &name), value);
                    }
                    if let Some(input) = &inline.configuration {
                        input
                            .validate_shape_at(
                                &crate::manifest::FieldPath::root()
                                    .table("requires")
                                    .table("packages")
                                    .table(&key)
                                    .table("configuration"),
                            )
                            .map_err(|e| format!("{}: {}", e.field, e.message))?;
                        configurations.insert(link_key(&group, &name), input.clone());
                    }
                    // Per-edge metadata is source-kind independent and keeps
                    // explicit defaults distinguishable from absent fields.
                    if let Some(link) = inline.link {
                        links.insert(link_key(&group, &name), link);
                    }
                    if let Some(access) = inline.access {
                        accesses.insert(link_key(&group, &name), access);
                    }
                    if let Some(friend) = inline.friend {
                        friend_flags.insert(link_key(&group, &name), friend);
                    }
                    if let Some(exclude) = inline.exclude.clone() {
                        excludes.insert(link_key(&group, &name), exclude);
                    }
                    // Source dispatch stays path, git, variable, registry.
                    let mut inline = *inline;
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
        let mut remaps: Vec<PackageRef> = Vec::new();
        for (key, constraint) in w.remaps {
            let (kind, group, name) = parse_pkgref_key(&key).map_err(|e| e.to_string())?;
            let version = VersionSpec::parse(&constraint).map_err(|e| e.to_string())?;
            remaps.push(
                PackageRef::new(kind, Some(group), name, version).map_err(|e| e.to_string())?,
            );
        }
        Ok(Requires {
            packages,
            registry_pins,
            capabilities: w.capabilities,
            remaps,
            git_packages,
            path_packages,
            var_packages,
            links,
            accesses,
            friend_flags,
            excludes,
            configurations,
            optional,
        })
    }
}

fn visibility_wire(
    requires: &Requires,
    group: &Group,
    name: &str,
) -> (Option<AccessLevel>, Option<bool>, Option<Vec<String>>) {
    let key = link_key(group, name);
    (
        requires.accesses.get(&key).copied(),
        requires.friend_flags.get(&key).copied(),
        requires.excludes.get(&key).cloned(),
    )
}

pub(in crate::manifest) fn parse_pkgref_key(
    key: &str,
) -> Result<(Option<PackageKind>, Group, String)> {
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
        // The written form of a channel is its own name; it is lowered to
        // an exact pin before the solve, never widened to a constraint
        // (PROP-075 `#CHANNEL-SYNTAX`).
        VersionSpec::Channel(name) => name.as_str().to_string(),
    }
}

#[cfg(test)]
mod tests;
