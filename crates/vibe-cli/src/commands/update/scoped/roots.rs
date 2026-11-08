//! How a scoped `vibe update` names its roots: the named packages qualified
//! against the lock and re-resolved at their declared constraint, or —
//! `--sources-only` — pinned to their exact locked versions.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::{PackageRef, VersionSpec};

use crate::commands::short_name;

/// `vibe update --sources-only`: the named locked packages (or every locked
/// package) pinned to their exact locked versions, so only source selection
/// can change.
pub(super) fn source_only_roots(
    packages: &[String],
    lockfile: &Lockfile,
) -> Result<Vec<PackageRef>> {
    let selected: Vec<&vibe_core::manifest::LockedPackage> = if packages.is_empty() {
        lockfile.packages.iter().collect()
    } else {
        let mut rows = Vec::with_capacity(packages.len());
        for raw in packages {
            let pkgref = PackageRef::parse(raw).with_context(|| format!("parsing `{raw}`"))?;
            let pkgref = short_name::qualify_locked(&pkgref, lockfile)?;
            let row = pkgref
                .group
                .as_ref()
                .and_then(|group| lockfile.find(group, &pkgref.name))
                .with_context(|| {
                    format!("package `{pkgref}` is not locked; `--sources-only` changes only locked sources")
                })?;
            rows.push(row);
        }
        rows
    };
    if selected.is_empty() {
        bail!("`vibe.lock` holds no package; `--sources-only` has nothing to refresh");
    }
    selected
        .into_iter()
        .map(|row| {
            let exact = semver::VersionReq::parse(&format!("={}", row.version))
                .with_context(|| format!("locked version of `{}/{}`", row.group, row.name))?;
            PackageRef::new(
                None,
                Some(row.group.clone()),
                row.name.to_string(),
                VersionSpec::Req(exact),
            )
            .with_context(|| format!("locked coordinate `{}/{}`", row.group, row.name))
        })
        .collect()
}

/// Each named package must already be installed; re-resolve it against its
/// original root constraint so a caret bumps within range.
///
/// `vibe update` refreshes an installed package, so a bare short name resolves
/// against `vibe.lock` alone — no index, no network — and a name not locked
/// fails here with a clear "not installed".
pub(super) fn qualify_roots(
    packages: &[String],
    manifest: &Manifest,
    lockfile: &Lockfile,
) -> Result<Vec<PackageRef>> {
    let mut roots: Vec<PackageRef> = Vec::with_capacity(packages.len());
    for raw in packages {
        let pkgref = PackageRef::parse(raw).with_context(|| format!("parsing `{raw}`"))?;
        let pkgref = short_name::qualify_locked(&pkgref, lockfile)?;
        let group = match pkgref.group.as_ref() {
            Some(g) => g.clone(),
            None => bail!(
                "`{pkgref}` resolved without a group — internal: `qualify_locked` should qualify"
            ),
        };
        if lockfile.find(&group, &pkgref.name).is_none() {
            bail!(
                "package `{group}/{}` is not installed — `vibe update` only refreshes installed \
                 packages; use `vibe install {group}/{}` to add it.",
                pkgref.name,
                pkgref.name,
            );
        }
        // The constraint to re-resolve against: the manifest `[requires]`
        // declaration is authoritative — the operator edits it to widen a
        // pin before updating — and the lockfile's `root_dependencies`
        // mirror is only the fallback.
        let constraint = manifest
            .requires
            .packages
            .iter()
            .find(|r| r.group.as_ref() == Some(&group) && r.name == pkgref.name)
            .or_else(|| {
                lockfile
                    .meta
                    .root_dependencies
                    .iter()
                    .find(|r| r.group.as_ref() == Some(&group) && r.name == pkgref.name)
            })
            .map(|r| r.version.clone())
            .unwrap_or(VersionSpec::Latest);
        roots.push(PackageRef::new(
            pkgref.kind,
            Some(group),
            pkgref.name,
            constraint,
        )?);
    }
    Ok(roots)
}
