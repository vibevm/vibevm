//! Recording toolkit — pin construction and the merge discipline for
//! `vibe.toml` / `vibe.lock`. Pure functions over core types; the
//! plan/apply phases drive them, and `vibe update` reuses the pin
//! helper.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use vibe_core::lock_provenance::{FetchedFrom, LockProvenance};
use vibe_core::manifest::{GitPackageDep, LockedPackage, Lockfile, Manifest, SourceKind};
use vibe_core::{ContentHash, PackageName, PackageRef, SourceUrl, VersionSpec};
use vibe_resolver::ResolvedNode;
use vibe_workspace::install::ResolvedDep;

use crate::fetched::Fetched;

/// A structural requirement over `version` with the given operator.
/// Built via typed `semver::Comparator`, never a string round-trip:
/// `VersionReq::parse("={version}")` rejects versions carrying build
/// metadata (a req has no build-metadata grammar), and build metadata
/// never participates in constraint matching anyway.
fn req_with_op(op: semver::Op, version: &semver::Version) -> semver::VersionReq {
    semver::VersionReq {
        comparators: vec![semver::Comparator {
            op,
            major: version.major,
            minor: Some(version.minor),
            patch: Some(version.patch),
            pre: version.pre.clone(),
        }],
    }
}

/// Build a `<group>/<name>@=<exact-version>` pkgref for fetching the
/// version the solver chose, regardless of how the user originally
/// constrained the package.
pub fn exact_pinned_pkgref(node: &ResolvedNode) -> PackageRef {
    PackageRef {
        kind: None,
        group: Some(node.group.clone()),
        name: PackageName::from_validated(node.name.clone()),
        version: VersionSpec::Req(req_with_op(semver::Op::Exact, &node.version)),
    }
}

/// Convert a caller-supplied root into the form that lands on disk in
/// `vibe.toml` `[requires].packages`. Three cases:
///
/// 1. `exact` set → always `=<resolved-version>`, ignoring whatever
///    constraint the user typed (matches npm `--save-exact` —
///    operator wants exact pinning, not the default).
/// 2. No version given (`flow:wal` → `VersionSpec::Latest`) → write
///    caret based on the resolved version (`^0.1.0`). Same default as
///    Cargo `cargo add`, npm `npm install`, Poetry `poetry add`.
/// 3. An explicit constraint (`@^0.1`, `@=0.2.0`, `@~0.3.1`,
///    `@>=0.2, <1.0`, …) → preserved verbatim. The user already
///    declared their intent; we don't second-guess.
pub fn finalize_pkgref_for_manifest(
    cli_pkgref: &PackageRef,
    resolved_version: &semver::Version,
    exact: bool,
) -> PackageRef {
    let version = if exact {
        VersionSpec::Req(req_with_op(semver::Op::Exact, resolved_version))
    } else if matches!(cli_pkgref.version, VersionSpec::Latest) {
        VersionSpec::Req(req_with_op(semver::Op::Caret, resolved_version))
    } else {
        cli_pkgref.version.clone()
    };
    PackageRef {
        kind: cli_pkgref.kind,
        group: cli_pkgref.group.clone(),
        name: cli_pkgref.name.clone(),
        version,
    }
}

/// Merge new root pkgrefs into `lockfile.meta.root_dependencies`,
/// deduplicating on `(group, name)` (idempotent re-installs don't grow
/// the list). Existing entries for the same `(group, name)` are
/// overwritten by the new pkgref so a constraint change in
/// `vibe install` updates the recorded root constraint.
pub fn merge_root_dependencies(lockfile: &mut Lockfile, roots: &[PackageRef]) {
    for r in roots {
        let pos = lockfile
            .meta
            .root_dependencies
            .iter()
            .position(|existing| existing.group == r.group && existing.name == r.name);
        match pos {
            Some(i) => lockfile.meta.root_dependencies[i] = r.clone(),
            None => lockfile.meta.root_dependencies.push(r.clone()),
        }
    }
}

/// Replace the lock's compatibility snapshot with the complete unique root
/// projection, for both bare and explicit installs. Consumer manifests, rather
/// than this flattened snapshot, remain authoritative for future solving.
pub(crate) fn replace_root_dependencies(lockfile: &mut Lockfile, roots: &[PackageRef]) {
    lockfile.meta.root_dependencies = roots.to_vec();
}

/// Merge new root pkgrefs into `manifest.requires.packages`, same
/// dedup discipline as [`merge_root_dependencies`]. Returns `true` if
/// any entry was added or changed — caller writes the manifest only
/// when the in-memory shape actually diverged from disk.
///
/// Preserves existing git/path source declarations. For a registry requirement,
/// only its selected-node version variable is replaced by the explicit value;
/// leaving that variable would overwrite the new value during serialization.
pub fn merge_manifest_requires(manifest: &mut Manifest, roots: &[PackageRef]) -> bool {
    let mut changed = false;
    for r in roots {
        if manifest
            .requires
            .git_packages
            .iter()
            .any(|g| Some(&g.group) == r.group.as_ref() && g.name == r.name)
            || manifest
                .requires
                .path_packages
                .iter()
                .any(|p| Some(&p.group) == r.group.as_ref() && p.name == r.name)
        {
            // A version request does not change an authored source route.
            continue;
        }
        let before = manifest.requires.var_packages.len();
        manifest
            .requires
            .var_packages
            .retain(|v| Some(&v.group) != r.group.as_ref() || v.name != r.name);
        changed |= before != manifest.requires.var_packages.len();
        let pos = manifest
            .requires
            .packages
            .iter()
            .position(|existing| existing.group == r.group && existing.name == r.name);
        match pos {
            Some(i) => {
                if manifest.requires.packages[i] != *r {
                    manifest.requires.packages[i] = r.clone();
                    changed = true;
                }
            }
            None => {
                manifest.requires.packages.push(r.clone());
                changed = true;
            }
        }
    }
    changed
}

/// Project the complete post-edit declarations into the legacy unique-root
/// snapshot. Differing ranges retain the selected version, not a guessed
/// semver intersection. Current manifests remain the solve authority.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INSTALL-CONSUMER-RECORDING"
)]
pub(crate) fn workspace_root_snapshot(
    workspace: &vibe_workspace::Workspace,
    resolution: &[ResolvedDep],
) -> crate::error::Result<Vec<PackageRef>> {
    let mut groups: std::collections::BTreeMap<(vibe_core::Group, String), Vec<PackageRef>> =
        std::collections::BTreeMap::new();
    for (consumer, node) in workspace.iter_nodes() {
        let mut declarations = node.requires.packages.clone();
        for git in &node.requires.git_packages {
            declarations.push(PackageRef::new(
                git.kind,
                Some(git.group.clone()),
                git.name.clone(),
                git.version.clone().unwrap_or(VersionSpec::Latest),
            )?);
        }
        for path in &node.requires.path_packages {
            declarations.push(PackageRef::new(
                path.kind,
                Some(path.group.clone()),
                path.name.clone(),
                path.version.clone().unwrap_or(VersionSpec::Latest),
            )?);
        }
        for declaration in declarations {
            let group = declaration
                .group
                .clone()
                .ok_or_else(|| crate::Error::RootNotFetched {
                    pkgref: declaration.to_string(),
                })?;
            let selected = resolution
                .iter()
                .find(|p| p.group == group && p.name == declaration.name)
                .ok_or_else(|| crate::Error::RootNotFetched {
                    pkgref: declaration.to_string(),
                })?;
            let mismatch = if !declaration.version.matches(&selected.version) {
                Some(vibe_resolver::SolveError::VersionConflict {
                    package: declaration.qualified_name(),
                    existing: selected.version.to_string(),
                    new_constraint: declaration.version.to_string(),
                })
            } else if let Some(requested) = declaration.kind
                && requested != selected.kind
            {
                Some(vibe_resolver::SolveError::KindMismatch {
                    package: declaration.qualified_name(),
                    requested,
                    actual: selected.kind,
                })
            } else {
                None
            };
            if let Some(source) = mismatch {
                return Err(crate::Error::ConsumerConstraints {
                    declarations: vec![format!("{consumer}/vibe.toml: {declaration}")],
                    source,
                });
            }
            groups
                .entry((group, declaration.name.to_string()))
                .or_default()
                .push(declaration);
        }
    }
    let mut snapshot = Vec::new();
    for ((group, name), declarations) in groups {
        let Some(first) = declarations.first() else {
            continue;
        };
        let selected = resolution
            .iter()
            .find(|p| p.group == group && p.name == name)
            .ok_or_else(|| crate::Error::RootNotFetched {
                pkgref: first.to_string(),
            })?;
        if declarations.iter().all(|d| d == first) {
            snapshot.push(first.clone());
        } else {
            snapshot.push(PackageRef {
                kind: Some(selected.kind),
                group: Some(group),
                name: first.name.clone(),
                version: VersionSpec::Req(req_with_op(semver::Op::Exact, &selected.version)),
            });
        }
    }
    Ok(snapshot)
}

/// Record a `--git` source declaration into `manifest.requires`:
/// replace any prior git-source entry for the same `(group, name)`
/// (updating an existing declaration) and drop a conflicting
/// registry-resolved entry from `requires.packages`, since M1.15 forbids
/// a `(group, name)` collision between the two tables — the
/// `RequiresWire` deserialiser rejects it on the next parse. Pure: it
/// mutates in memory and the caller persists, exactly as
/// [`merge_manifest_requires`] does (the CLI writes before it resolves,
/// so a panic mid-resolve cannot strand the declaration off disk). This
/// is the manifest-mutation discipline the CLI used to own inline; the
/// CLI now translates the `--git*` flags into the [`GitPackageDep`] and
/// hands it here.
pub fn record_git_source(manifest: &mut Manifest, dep: GitPackageDep) {
    // Drop any prior registry-resolved entry for the same pkgref.
    manifest
        .requires
        .packages
        .retain(|p| !(p.group.as_ref() == Some(&dep.group) && p.name == dep.name));
    // Replace any prior git-source entry for the same pkgref (same shape
    // as updating an existing constraint).
    manifest
        .requires
        .git_packages
        .retain(|g| !(g.group == dep.group && g.name == dep.name));
    manifest.requires.git_packages.push(dep);
}

/// Build a [`LockedPackage`] from a fetched node. The lockfile records
/// the resolution provenance; the materialised footprint is the
/// `vibedeps/` slot — deterministic from `(kind, name, version)` — so
/// `files_written` stays empty and the `NN-` `boot_snippet` filename
/// is retired.
pub(crate) fn locked_package_from_fetched(
    f: &Fetched,
    resolved: &ResolvedDep,
    language: Option<&str>,
) -> LockedPackage {
    let c = &f.cached;
    let source_kind = if c.overridden {
        SourceKind::Override
    } else if c.is_path_source {
        SourceKind::Path
    } else if c.is_git_source {
        SourceKind::Git
    } else if c.is_local {
        SourceKind::Local
    } else if c.is_embedded {
        SourceKind::Embedded
    } else {
        SourceKind::Registry
    };
    LockedPackage {
        kind: c.package_meta().kind,
        group: c.resolved.group.clone(),
        name: PackageName::from_validated(c.resolved.name.clone()),
        version: c.resolved.version.clone(),
        bridge: c.package_meta().bridge,
        authors: c.package_meta().authors.clone(),
        registry: c.registry_name.clone(),
        source_url: SourceUrl::new(c.source_uri.clone()),
        source_ref: c.source_ref.clone(),
        resolved_commit: c.resolved_commit.clone(),
        content_hash: ContentHash::from_validated(c.content_hash.clone()),
        embedded_sources: f.embedded_sources.clone(),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies: f.meta.dependencies.clone(),
        // Filled once every row exists (`record_app_requirements`).
        required_by: Vec::new(),
        admitted_by: resolved.admitted_by.clone(),
        via_override: resolved.via_override.clone(),
        overridden: c.overridden,
        source_kind: Some(source_kind),
        via_redirect: c.via_redirect.clone(),
        features: f
            .feature_expansion
            .active_features
            .iter()
            .cloned()
            .collect(),
        subskills_active: Vec::new(),
        describes: c.package_meta().describes.as_ref().map(|p| p.to_string()),
        language: language.map(str::to_string),
        // Record the package's declared materialization so destructive ops
        // and the guard (PROP-022 §2.6) recognise an in-place slot.
        materialization: c.package_meta().materialization,
    }
}

/// Note on every lock row the locked application packages that require it
/// (PROP-065 `#app-dependencies`), read from the fetched manifests.
pub(crate) fn record_app_requirements(lockfile: &mut Lockfile, fetched: &[Fetched]) {
    lockfile.record_app_requirements(|row| {
        fetched
            .iter()
            .find(|f| f.cached.resolved.group == row.group && f.cached.resolved.name == row.name)
            .map(|f| &f.cached.manifest)
    });
}

/// Where the bytes of each locked package actually came from
/// (`PROP-066#provenance`).
///
/// `fetched_from` was to sit on the `[[package]]` row beside the canonical
/// `source_url`, on the understanding that an older vibe ignores a field
/// it does not know. It does not: every lockfile struct carries
/// `deny_unknown_fields`, so the extra key would make an older binary
/// unable to read the lock at all — the one file a project cannot afford
/// to make unreadable. The PROP's own fallback applies, and the fact goes
/// into the sidecar `vibe.lock.provenance`, which an older vibe never
/// opens, is regenerated on every install, and is resolved from by
/// nothing: deleting it loses a record and changes nothing about what is
/// installed. A run where every package came from its canonical origin
/// records nothing, and the sidecar is then removed rather than written
/// empty.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#provenance")]
pub(crate) fn fetch_provenance(fetched: &[Fetched]) -> LockProvenance {
    LockProvenance {
        packages: fetched
            .iter()
            .filter_map(|f| {
                Some(FetchedFrom {
                    package: format!(
                        "{}/{}",
                        f.cached.resolved.group.as_str(),
                        f.cached.resolved.name
                    ),
                    version: f.cached.resolved.version.to_string(),
                    fetched_from: f.cached.fetched_from.clone()?,
                    row: None,
                })
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_root_snapshot_drops_removed_dependencies() {
        let mut lockfile = Lockfile::empty("vibe test", "2026-08-29T00:00:00Z");
        lockfile.meta.root_dependencies = vec![
            "flow:org.example/kept@^1".parse().unwrap(),
            "flow:org.example/removed@^1".parse().unwrap(),
        ];
        let current = vec!["flow:org.example/kept@^1".parse().unwrap()];

        replace_root_dependencies(&mut lockfile, &current);

        assert_eq!(lockfile.meta.root_dependencies, current);
    }

    #[test]
    fn explicit_root_merge_preserves_other_declared_dependencies() {
        let mut lockfile = Lockfile::empty("vibe test", "2026-08-29T00:00:00Z");
        lockfile.meta.root_dependencies = vec!["flow:org.example/kept@^1".parse().unwrap()];
        let added: PackageRef = "flow:org.example/added@^2".parse().unwrap();

        merge_root_dependencies(&mut lockfile, std::slice::from_ref(&added));

        assert_eq!(lockfile.meta.root_dependencies.len(), 2);
        assert_eq!(lockfile.meta.root_dependencies[1], added);
    }
}
