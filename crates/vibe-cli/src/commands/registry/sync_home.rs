//! `vibe registry sync -g` — refresh the user home's registry cache: the
//! cached package clones of every registry in `~/.vibe/registry.toml`, and
//! each registry's index handshake.
//!
//! A user who installs applications globally has no `vibe.lock` to walk,
//! and until this existed the only way to see a release published within
//! the last hour was to wait or to delete `<home>/registries/`. The set
//! refreshed here is the one `vibe_app::from_global` resolves through —
//! the global file's registries, mirrors and overrides opened by the same
//! `MultiRegistryResolver::open` — so a refresh cannot miss a clone a
//! global install would read, nor touch one it would not.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#registry");

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use vibe_core::GlobalRegistryConfig;
use vibe_core::user_config::UserConfig;
use vibe_registry::multi_registry_resolver::RegistrySource;
use vibe_registry::{MultiRegistryResolver, RefreshCatalog, RegistryRefresh};

use crate::output;
use crate::output::language::{Cell, Glyph, List, Meta, Role, Row};

#[derive(Debug, Serialize)]
struct HomeSyncReport {
    ok: bool,
    command: &'static str,
    scope: &'static str,
    home: String,
    refreshed: Vec<RefreshedRegistry>,
    skipped: Vec<SkippedHomeEntry>,
}

/// One registry the refresh reached. `via` says how its catalog was
/// refreshed: `index` when the index answered its handshake, `git` when
/// the registry has no index and its tags are the catalog, `repository`
/// when a monorepo's one repository was fetched at its `ref`.
#[derive(Debug, Serialize)]
struct RefreshedRegistry {
    registry: String,
    #[serde(rename = "ref")]
    refname: String,
    via: &'static str,
    /// The clones brought to their newest release, in the project form's
    /// entry shape.
    clones: Vec<super::sync::RefreshedReportEntry>,
}

/// A registry, or one package of a registry, the refresh left alone.
#[derive(Debug, Serialize)]
struct SkippedHomeEntry {
    registry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    reason: String,
}

pub(super) fn run_sync_home(ctx: &output::Context, root_offline: bool) -> Result<()> {
    let user_config = UserConfig::load().context("loading the user config")?;
    if output::resolve_offline(root_offline, user_config.net.offline) {
        bail!(
            "`vibe registry sync -g` refreshes the registry cache from the network, \
             which the offline posture forbids; run it again without `--offline` or `VIBE_OFFLINE`."
        );
    }
    let home = vibe_core::settings::settings_dir().ok_or_else(|| {
        anyhow!(
            "the Vibe settings root is unavailable; set VIBE_SETTINGS or make the user home resolvable"
        )
    })?;
    let global = GlobalRegistryConfig::load().context("loading `~/.vibe/registry.toml`")?;
    // A registry the effective `[source_policy]` disables is not refreshed:
    // refreshing it is contacting it (PROP-073 `#SOURCE-POLICY`). It stays
    // declared and is reported as skipped, exactly like `enabled = false`.
    let policy = super::effective_source_policy(None)?;
    let (admitted, excluded): (Vec<_>, Vec<_>) = global
        .registries
        .iter()
        .cloned()
        .partition(|row| !policy.excludes_endpoint(&row.url));

    ctx.heading(&format!(
        "Refreshing the registry cache of {}",
        home.display()
    ));
    let task = ctx
        .progress()
        .task("Refreshing the user home's registry clones");
    let refreshed =
        MultiRegistryResolver::open_refreshing(&admitted, &global.mirrors, &global.overrides)
            .context("opening the user home's registries")
            .and_then(|mrr| {
                // A refresh that moves bytes is a transfer of this task
                // (`PROP-068#git`).
                let walked = vibe_registry::git_backend::transfers::observed(
                    Some(&task.progress()),
                    None,
                    None,
                    || mrr.refresh_cached_clones(),
                )
                .context("refreshing the cached package clones")?;
                Ok((walked, local_sources(&mrr)?))
            });
    let (walked, locals) = match refreshed {
        Ok(found) => {
            task.finish();
            found
        }
        Err(error) => {
            task.fail(error.to_string());
            return Err(error);
        }
    };

    let mut skipped: Vec<SkippedHomeEntry> = global
        .registries
        .iter()
        .filter(|r| !r.enabled)
        .map(|r| registry_skip(&r.name, "disabled (`enabled = false`)"))
        .collect();
    skipped.extend(excluded.iter().map(|r| {
        registry_skip(
            &r.name,
            vibe_registry::multi_registry_resolver::DISABLED_BY_POLICY_MARK,
        )
    }));
    skipped.extend(locals.iter().map(|name| {
        registry_skip(
            name,
            "a local directory registry is read in place and has no clone to refresh",
        )
    }));
    for refresh in &walked {
        skipped.extend(refresh.skipped.iter().map(|e| SkippedHomeEntry {
            registry: refresh.registry.clone(),
            group: Some(e.group.to_string()),
            name: Some(e.name.clone()),
            reason: e.reason.clone(),
        }));
    }
    let report = HomeSyncReport {
        ok: true,
        command: "registry:sync",
        scope: "user",
        home: home.display().to_string(),
        refreshed: walked.iter().map(refreshed_registry).collect(),
        skipped,
    };
    if ctx.is_json() {
        return ctx.emit_json(&report);
    }
    draw(ctx, &report);
    Ok(())
}

fn local_sources(mrr: &MultiRegistryResolver) -> Result<Vec<String>> {
    Ok(mrr
        .sources()?
        .iter()
        .filter_map(|source| match source {
            RegistrySource::Local(local) => Some(local.name.clone()),
            RegistrySource::Git(_) | RegistrySource::Monorepo(_) => None,
        })
        .collect())
}

fn registry_skip(registry: &str, reason: &str) -> SkippedHomeEntry {
    SkippedHomeEntry {
        registry: registry.to_string(),
        group: None,
        name: None,
        reason: reason.to_string(),
    }
}

fn refreshed_registry(refresh: &RegistryRefresh) -> RefreshedRegistry {
    RefreshedRegistry {
        registry: refresh.registry.clone(),
        refname: refresh.registry_ref.clone(),
        via: match refresh.catalog {
            RefreshCatalog::Index => "index",
            RefreshCatalog::Tags => "git",
            RefreshCatalog::Repository => "repository",
        },
        clones: refresh
            .clones
            .iter()
            .map(super::sync::RefreshedReportEntry::from_entry)
            .collect(),
    }
}

/// One row per registry with its clones as children, then the rows the
/// refresh left, then the trailer.
fn draw(ctx: &output::Context, report: &HomeSyncReport) {
    let mut rows: Vec<Row> = report
        .refreshed
        .iter()
        .map(|r| {
            let count = r.clones.len();
            let unit = if count == 1 { "clone" } else { "clones" };
            let (catalog, what) = match r.via {
                "index" => ("index answered", format!("{count} {unit}")),
                "repository" => ("monorepo fetched", format!("@ {}", r.refname)),
                _ => ("no index, tags read", format!("{count} {unit}")),
            };
            r.clones.iter().fold(
                Row::new(vec![
                    Cell::new(r.registry.clone(), Role::Accent),
                    Cell::text(what),
                ])
                .glyph(Glyph::Done, Role::Ok)
                .meta(vec![Meta::note(catalog)]),
                |row, c| row.child(format!("{}/{} @ {}", c.group, c.name, c.refname)),
            )
        })
        .collect();
    rows.extend(report.skipped.iter().map(|s| {
        let what = match (&s.group, &s.name) {
            (Some(group), Some(name)) => format!("{} {group}/{name}", s.registry),
            _ => s.registry.clone(),
        };
        Row::new(vec![Cell::text(what)])
            .glyph(Glyph::Skipped, Role::Dim)
            .dim(true)
            .meta(vec![Meta::note(s.reason.as_str())])
    }));
    if !rows.is_empty() {
        ctx.block(&List::new(rows).render());
    }
    let count = report.refreshed.len();
    let unit = if count == 1 { "registry" } else { "registries" };
    ctx.trailer(&format!("refreshed {count} {unit}"));
}
