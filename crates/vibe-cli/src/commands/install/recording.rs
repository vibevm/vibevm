//! Recording and reporting for `vibe install` — merging resolved roots
//! into `vibe.toml` / `vibe.lock`, building the locked-package entries,
//! and emitting the plan / outcome envelopes.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use anyhow::Result;
use serde::Serialize;
use vibe_core::manifest::{LockedPackage, Lockfile, Manifest, SourceKind};
use vibe_core::{PackageRef, VersionSpec};
use vibe_workspace::install::{InstallOutcome, ResolvedDep};

use crate::output;

use super::planning::Fetched;

/// Merge new root pkgrefs into `lockfile.meta.root_dependencies`,
/// deduplicating on `(group, name)` (idempotent re-installs don't grow
/// the list). Existing entries for the same `(group, name)` are
/// overwritten by the new pkgref so a constraint change in
/// `vibe install` updates the recorded root constraint.
pub(super) fn merge_root_dependencies(lockfile: &mut Lockfile, roots: &[PackageRef]) {
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

/// Convert a CLI-supplied root into the form that lands on disk in
/// `vibe.toml` `[requires].packages`. Three cases:
///
/// 1. `--exact` set → always `=<resolved-version>`, ignoring whatever
///    constraint the user typed (matches npm `--save-exact` —
///    operator wants exact pinning, not the default).
/// 2. CLI had no version (`flow:wal` → `VersionSpec::Latest`) → write
///    caret based on the resolved version (`^0.1.0`). Same default as
///    Cargo `cargo add`, npm `npm install`, Poetry `poetry add`.
/// 3. CLI had an explicit constraint (`@^0.1`, `@=0.2.0`, `@~0.3.1`,
///    `@>=0.2, <1.0`, …) → preserve it verbatim. The user already
///    declared their intent; we don't second-guess.
pub(super) fn finalize_pkgref_for_manifest(
    cli_pkgref: &PackageRef,
    resolved_version: &semver::Version,
    exact: bool,
) -> PackageRef {
    let version = if exact {
        let req = semver::VersionReq::parse(&format!("={resolved_version}"))
            .expect("`=<version>` always parses as VersionReq");
        VersionSpec::Req(req)
    } else if matches!(cli_pkgref.version, VersionSpec::Latest) {
        let req = semver::VersionReq::parse(&format!("^{resolved_version}"))
            .expect("`^<version>` always parses as VersionReq");
        VersionSpec::Req(req)
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

/// Merge new root pkgrefs into `manifest.requires.packages`, same
/// dedup discipline as `merge_root_dependencies`. Returns `true` if
/// any entry was added or changed — caller writes the manifest only
/// when the in-memory shape actually diverged from disk.
///
/// Skips pkgrefs that are already declared as a git-source in
/// `manifest.requires.git_packages` — those were recorded earlier via
/// `apply_git_source_flag` (M1.15) and writing them again as
/// registry-resolved would create a `(group, name)` duplicate that
/// `try_from = "RequiresWire"` rejects on the next parse.
pub(super) fn merge_manifest_requires(manifest: &mut Manifest, roots: &[PackageRef]) -> bool {
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

/// Build a [`LockedPackage`] from a fetched node. The lockfile records the
/// resolution provenance; the materialised footprint is the `vibedeps/`
/// slot — deterministic from `(kind, name, version)` — so `files_written`
/// stays empty and the `NN-` `boot_snippet` filename is retired.
pub(super) fn locked_package_from_fetched(f: &Fetched, language: Option<&str>) -> LockedPackage {
    let c = &f.cached;
    let source_kind = if c.overridden {
        SourceKind::Override
    } else if c.is_path_source {
        SourceKind::Path
    } else if c.is_git_source {
        SourceKind::Git
    } else {
        SourceKind::Registry
    };
    LockedPackage {
        kind: c.package_meta().kind,
        group: c.resolved.group.clone(),
        name: c.resolved.name.clone(),
        version: c.resolved.version.clone(),
        registry: c.registry_name.clone(),
        source_url: c.source_uri.clone(),
        source_ref: c.source_ref.clone(),
        resolved_commit: c.resolved_commit.clone(),
        content_hash: c.content_hash.clone(),
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
    }
}

pub(super) fn present_resolution(ctx: &output::Context, resolution: &[ResolvedDep]) {
    if ctx.is_json() {
        #[derive(Serialize)]
        struct PlanEntry {
            package: String,
            version: String,
        }
        let payload: Vec<PlanEntry> = resolution
            .iter()
            .map(|d| PlanEntry {
                package: format!("{}/{}", d.group, d.name),
                version: d.version.to_string(),
            })
            .collect();
        let _ = ctx.emit_json(&serde_json::json!({
            "command": "install:plan",
            "packages": payload,
        }));
        return;
    }
    if ctx.is_quiet() {
        return;
    }
    ctx.heading(&format!(
        "\nMaterialising {} package{} into vibedeps/:",
        resolution.len(),
        if resolution.len() == 1 { "" } else { "s" },
    ));
    for d in resolution {
        println!("  {}/{}@{}", d.group, d.name, d.version);
    }
    println!();
}

pub(super) fn emit_report(ctx: &output::Context, outcome: &InstallOutcome) -> Result<()> {
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "install",
            "materialised": outcome.materialised,
            "skipped": outcome.skipped,
            "pruned": outcome.pruned,
            "nodes_regenerated": outcome.nodes_regenerated,
        }))?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe install: {} package{} materialised",
            outcome.materialised.len(),
            if outcome.materialised.len() == 1 {
                ""
            } else {
                "s"
            },
        ));
        return Ok(());
    }
    ctx.summary(&format!(
        "\nMaterialised {} package{} into vibedeps/; regenerated boot artifacts for {} node{}.",
        outcome.materialised.len(),
        if outcome.materialised.len() == 1 {
            ""
        } else {
            "s"
        },
        outcome.nodes_regenerated.len(),
        if outcome.nodes_regenerated.len() == 1 {
            ""
        } else {
            "s"
        },
    ));
    if !outcome.skipped.is_empty() {
        ctx.step(&format!(
            "{} slot{} already present — re-copy skipped (PROP-011 §2.3)",
            outcome.skipped.len(),
            if outcome.skipped.len() == 1 { "" } else { "s" },
        ));
    }
    if !outcome.pruned.is_empty() {
        ctx.step(&format!(
            "pruned {} stale vibedeps/ slot{}",
            outcome.pruned.len(),
            if outcome.pruned.len() == 1 { "" } else { "s" },
        ));
    }
    Ok(())
}

/// Report the PROP-011 §2.2 fast path — `vibe.lock` was fresh, so no
/// resolution ran. Kept distinct from [`emit_report`] so the operator can
/// tell a no-op `vibe install` from one that materialised packages.
pub(super) fn emit_fresh_report(ctx: &output::Context, nodes_regenerated: &[String]) -> Result<()> {
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "install",
            "unchanged": true,
            "nodes_regenerated": nodes_regenerated,
        }))?;
        return Ok(());
    }
    ctx.summary(&format!(
        "vibe install: vibe.lock unchanged — nothing to re-resolve ({} node{} up to date)",
        nodes_regenerated.len(),
        if nodes_regenerated.len() == 1 {
            ""
        } else {
            "s"
        },
    ));
    Ok(())
}
