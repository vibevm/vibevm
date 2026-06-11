//! `vibe outdated` — show which installed packages have newer
//! versions available in their configured registry.
//!
//! Spec: [PROP-003 §M1.10](../../../../spec/research/PROP-004-tessl-comparative-research.md#outdated)
//! and [ROADMAP §M1.10](../../../../ROADMAP.md).
//!
//! Read-only: walks the lockfile, asks the resolver
//! `list_versions` per package, picks the highest non-prerelease
//! version, compares with the lockfile pin. Emits a status table
//! sorted by `<kind>:<name>`. JSON envelope under `--json` for CI
//! consumption.
//!
//! Out-of-scope here: `--upstream` mode that walks `describes`
//! PURLs against npm/pypi/cargo.io. That requires per-ecosystem
//! HTTP probes and is queued for a follow-up slice once the
//! threat-model question (PROP-004 §5.9) is settled.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#cli-surface");

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::{Group, PackageRef, VersionSpec};
use vibe_registry::MultiRegistryResolver;

use crate::cli::OutdatedArgs;
use crate::output;

#[derive(Debug, Serialize)]
struct OutdatedEntry {
    group: String,
    name: String,
    installed: String,
    latest: Option<String>,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct OutdatedReport {
    ok: bool,
    command: &'static str,
    project: String,
    packages: Vec<OutdatedEntry>,
    total: usize,
    update_available: usize,
}

pub fn run(ctx: &output::Context, args: OutdatedArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest = load_project_manifest(&project_root)?;
    let lockfile = load_lockfile(&project_root)?;

    if lockfile.packages.is_empty() {
        if ctx.is_json() {
            ctx.emit_json(&OutdatedReport {
                ok: true,
                command: "outdated",
                project: project_root.display().to_string(),
                packages: Vec::new(),
                total: 0,
                update_available: 0,
            })?;
            return Ok(());
        }
        ctx.summary("(no packages installed)");
        return Ok(());
    }

    if manifest.registries.is_empty() {
        bail!(
            "no registry configured. Add a `[[registry]]` entry to `vibe.toml` or run `vibe outdated` against a project that has one."
        );
    }
    let mrr =
        MultiRegistryResolver::open(&manifest.registries, &manifest.mirrors, &manifest.overrides)
            .context("opening multi-registry resolver")?
            .with_strict_auth(args.auth_required)
            .with_git_packages(manifest.requires.git_packages.clone());

    let mut entries: Vec<OutdatedEntry> = Vec::with_capacity(lockfile.packages.len());
    let mut update_available = 0usize;
    for p in &lockfile.packages {
        let installed = p.version.clone();
        let latest = match probe_latest(&mrr, &p.group, &p.name) {
            Ok(v) => v,
            Err(e) => {
                tracing::debug!(
                    target: "vibe_outdated",
                    package = %format!("{}/{}", p.group, p.name),
                    error = %e,
                    "could not probe latest version"
                );
                None
            }
        };
        let status = match &latest {
            Some(v) if v > &installed => {
                update_available += 1;
                "update available"
            }
            Some(_) => "up to date",
            None => "unknown",
        };
        entries.push(OutdatedEntry {
            group: p.group.to_string(),
            name: p.name.clone(),
            installed: installed.to_string(),
            latest: latest.map(|v| v.to_string()),
            status,
        });
    }
    entries.sort_by(|a, b| {
        (a.group.as_str(), a.name.as_str()).cmp(&(b.group.as_str(), b.name.as_str()))
    });

    if ctx.is_json() {
        ctx.emit_json(&OutdatedReport {
            ok: true,
            command: "outdated",
            project: project_root.display().to_string(),
            total: entries.len(),
            update_available,
            packages: entries,
        })?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe outdated: {update_available} of {} package{} have updates available",
            entries.len(),
            if entries.len() == 1 { "" } else { "s" },
        ));
        return Ok(());
    }

    if entries.is_empty() {
        ctx.summary("(no packages installed)");
        return Ok(());
    }

    println!(
        "GROUP                 NAME                          INSTALLED      LATEST         STATUS"
    );
    for e in &entries {
        println!(
            "{:<20}  {:<28}  {:<14}  {:<14}  {}",
            e.group,
            e.name,
            e.installed,
            e.latest.as_deref().unwrap_or("-"),
            e.status
        );
    }
    println!();
    ctx.summary(&format!(
        "{update_available} of {} package{} have updates available",
        entries.len(),
        if entries.len() == 1 { "" } else { "s" },
    ));
    Ok(())
}

fn probe_latest(
    mrr: &MultiRegistryResolver,
    group: &Group,
    name: &str,
) -> Result<Option<semver::Version>> {
    let pkgref = PackageRef::new(
        None,
        Some(group.clone()),
        name.to_string(),
        VersionSpec::Latest,
    )
    .with_context(|| format!("constructing pkgref for {group}/{name}"))?;
    match mrr.resolve(&pkgref) {
        Ok(res) => Ok(Some(res.resolved.version)),
        Err(_) => Ok(None),
    }
}

fn resolve_project_root(path: &Path) -> Result<std::path::PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

fn load_project_manifest(root: &Path) -> Result<Manifest> {
    let path = root.join(Manifest::FILENAME);
    Ok(Manifest::read(&path)?)
}

fn load_lockfile(root: &Path) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    if !path.exists() {
        Ok(Lockfile::empty(
            format!("vibe {}", env!("CARGO_PKG_VERSION")),
            super::init::current_timestamp_utc(),
        ))
    } else {
        Ok(Lockfile::read(&path)?)
    }
}
