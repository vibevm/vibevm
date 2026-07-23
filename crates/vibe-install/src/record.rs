//! Recording toolkit — pin construction and the merge discipline for
//! `vibe.toml` / `vibe.lock`. Pure functions over core types; the
//! plan/apply phases drive them, and `vibe update` reuses the pin
//! helper.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use vibe_core::manifest::{GitPackageDep, LockedPackage, Lockfile, Manifest, SourceKind};
use vibe_core::{ContentHash, PackageName, PackageRef, SourceUrl, VersionSpec};
use vibe_resolver::ResolvedNode;

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

/// Merge new root pkgrefs into `manifest.requires.packages`, same
/// dedup discipline as [`merge_root_dependencies`]. Returns `true` if
/// any entry was added or changed — caller writes the manifest only
/// when the in-memory shape actually diverged from disk.
///
/// Skips pkgrefs that are already declared as a git-source in
/// `manifest.requires.git_packages` — those were recorded earlier via
/// the `--git` declaration path and writing them again as
/// registry-resolved would create a `(group, name)` duplicate that
/// `try_from = "RequiresWire"` rejects on the next parse.
pub fn merge_manifest_requires(manifest: &mut Manifest, roots: &[PackageRef]) -> bool {
    let mut changed = false;
    for r in roots {
        if manifest
            .requires
            .git_packages
            .iter()
            .any(|g| Some(&g.group) == r.group.as_ref() && g.name == r.name)
        {
            // Already declared as git-source — leave untouched.
            continue;
        }
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
pub(crate) fn locked_package_from_fetched(f: &Fetched, language: Option<&str>) -> LockedPackage {
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
        registry: c.registry_name.clone(),
        source_url: SourceUrl::new(c.source_uri.clone()),
        source_ref: c.source_ref.clone(),
        resolved_commit: c.resolved_commit.clone(),
        content_hash: ContentHash::from_validated(c.content_hash.clone()),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies: f.meta.dependencies.clone(),
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
