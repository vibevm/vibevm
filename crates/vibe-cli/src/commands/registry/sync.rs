//! `vibe registry sync` — refresh the per-package clones referenced by
//! the lockfile, or, with `-g` or outside a project, the user home's
//! cached clones (`sync_home`).

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#registry");

use anyhow::{Context, Result};
use serde::Serialize;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_registry::{MultiRegistryResolver, RefreshedEntry, RefreshedVia};

use crate::cli::RegistrySyncArgs;
use crate::output;

use super::{SkippedReportEntry, resolve_project_root};

#[derive(Debug, Serialize)]
struct SyncReport {
    ok: bool,
    command: &'static str,
    refreshed: Vec<RefreshedReportEntry>,
    skipped: Vec<SkippedReportEntry>,
}

#[derive(Debug, Serialize)]
pub(super) struct RefreshedReportEntry {
    pub(super) group: String,
    pub(super) name: String,
    via: String, // "registry:<name>" or "override"
    #[serde(rename = "ref")]
    pub(super) refname: String,
}

impl RefreshedReportEntry {
    pub(super) fn from_entry(e: &RefreshedEntry) -> Self {
        RefreshedReportEntry {
            group: e.group.as_str().to_string(),
            name: e.name.clone(),
            via: match &e.via {
                RefreshedVia::Registry(n) => format!("registry:{n}"),
                RefreshedVia::Override => "override".to_string(),
            },
            refname: e.refname.clone(),
        }
    }
}

pub(super) fn run_sync(
    ctx: &output::Context,
    args: RegistrySyncArgs,
    root_offline: bool,
) -> Result<()> {
    // Outside a project there is no lockfile to walk, and the clones a
    // user can mean are the home's: the plain form refreshes those rather
    // than refusing, the way `-g` asks for them explicitly.
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(Manifest::FILENAME);
    if args.global || !manifest_path.exists() {
        return super::sync_home::run_sync_home(ctx, root_offline);
    }
    let manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    let lockfile_path = project_root.join(Lockfile::FILENAME);
    if !lockfile_path.exists() {
        ctx.summary(
            "vibe registry sync: no `vibe.lock` yet — nothing installed, nothing to refresh.",
        );
        if ctx.is_json() {
            ctx.emit_json(&SyncReport {
                ok: true,
                command: "registry:sync",
                refreshed: Vec::new(),
                skipped: Vec::new(),
            })?;
        }
        return Ok(());
    }
    let lockfile = Lockfile::read(&lockfile_path)
        .with_context(|| format!("reading `{}`", lockfile_path.display()))?;

    if lockfile.packages.is_empty() {
        ctx.summary("vibe registry sync: lockfile is empty — nothing to refresh.");
        if ctx.is_json() {
            ctx.emit_json(&SyncReport {
                ok: true,
                command: "registry:sync",
                refreshed: Vec::new(),
                skipped: Vec::new(),
            })?;
        }
        return Ok(());
    }

    let effective = super::effective_registries(&project_root, &manifest)?;
    if effective.registries.is_empty() {
        // Empty `[[registry]]` is legal (e.g., projects that only use
        // `--registry <path>` or `[[override]]`-only setups), but
        // `registry sync` has nothing to do without `[[registry]]`
        // entries to dispatch through. Override-only refresh would
        // need its own flag; for now, surface the situation.
        ctx.summary(
            "vibe registry sync: no `[[registry]]` entries in `vibe.toml`, \
             `.vibe/registry.local.toml` or `~/.vibe/registry.toml` — nothing to refresh.",
        );
        if ctx.is_json() {
            ctx.emit_json(&SyncReport {
                ok: true,
                command: "registry:sync",
                refreshed: Vec::new(),
                skipped: Vec::new(),
            })?;
        }
        return Ok(());
    }

    // Refreshing a registry is contacting it, so a registry the effective
    // `[source_policy]` disables is left out of the walk entirely; it stays
    // declared (PROP-073 `#SOURCE-POLICY`).
    let policy = super::effective_source_policy(Some(&manifest))?;
    let admitted: Vec<_> = effective
        .registries
        .iter()
        .filter(|row| !policy.excludes_endpoint(&row.url))
        .cloned()
        .collect();
    for row in effective
        .registries
        .iter()
        .filter(|row| policy.excludes_endpoint(&row.url))
    {
        if let Some(disabled) =
            vibe_registry::multi_registry_resolver::disabled_registry(&policy, row)
        {
            ctx.summary(&disabled.skip_note());
        }
    }

    let mrr = MultiRegistryResolver::open(&admitted, &effective.mirrors, &effective.overrides)
        .context("opening multi-registry resolver")?;

    ctx.heading(&format!(
        "Syncing {} package clone{} referenced by lockfile",
        lockfile.packages.len(),
        if lockfile.packages.len() == 1 {
            ""
        } else {
            "s"
        }
    ));

    let refresh = ctx.progress().task("Refreshing lockfile package clones");
    refresh.set_progress(0, Some(lockfile.packages.len() as u64), "packages");
    let report = match mrr
        .refresh_lockfile_clones(&lockfile)
        .context("refreshing per-package clones")
    {
        Ok(report) => report,
        Err(error) => {
            refresh.fail(error.to_string());
            return Err(error);
        }
    };
    refresh.set_progress(
        lockfile.packages.len() as u64,
        Some(lockfile.packages.len() as u64),
        "packages",
    );
    refresh.finish();

    let json_refreshed: Vec<RefreshedReportEntry> = report
        .refreshed
        .iter()
        .map(RefreshedReportEntry::from_entry)
        .collect();
    let json_skipped: Vec<SkippedReportEntry> = report
        .skipped
        .iter()
        .map(|e| SkippedReportEntry {
            group: e.group.as_str().to_string(),
            name: e.name.clone(),
            reason: e.reason.clone(),
        })
        .collect();

    if ctx.is_json() {
        ctx.emit_json(&SyncReport {
            ok: true,
            command: "registry:sync",
            refreshed: json_refreshed,
            skipped: json_skipped,
        })?;
        return Ok(());
    }

    if !report.refreshed.is_empty() {
        for e in &report.refreshed {
            let via_text = match &e.via {
                RefreshedVia::Registry(name) => format!("registry `{name}`"),
                RefreshedVia::Override => "override".to_string(),
            };
            ctx.step(&format!(
                "{}/{} @ {} via {}",
                e.group, e.name, e.refname, via_text
            ));
        }
    }
    if !report.skipped.is_empty() {
        for e in &report.skipped {
            ctx.skipped(&format!("{}/{}", e.group, e.name), &e.reason);
        }
    }

    ctx.summary(&format!(
        "\nvibe registry sync: {} refreshed, {} skipped.",
        report.refreshed.len(),
        report.skipped.len()
    ));
    Ok(())
}
