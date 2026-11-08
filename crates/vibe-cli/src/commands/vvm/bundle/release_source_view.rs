//! `vibe self source show|set|reset` and `vibe self releases`
//! (`PROP-019#release-sources`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#release-sources");

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Serialize;
use specmark::spec;
use vibe_core::user_config::{ReleaseSourceKind, SelfConfig, UserConfig};

use super::super::model::{Kind, Origin};
use super::super::{VvmEnv, output, provenance};
use super::api::ApiClient;
use super::download::{Downloader, HttpDownloader};
use super::release_source::{self, EffectiveReleaseSource, SourceOrigin};
use super::releases::Releases;
use crate::cli::{VvmReleasesArgs, VvmSourceCommand, VvmSourceSetArgs};

/// How many releases `self releases` shows without `--all`.
const RECENT: usize = 10;

#[derive(Serialize)]
struct SourceView {
    protocol: &'static str,
    source: &'static str,
    location: String,
    from: String,
    row: Option<RowView>,
}

#[derive(Serialize)]
struct RowView {
    prefix: String,
    target: String,
    layer: &'static str,
}

fn view(effective: &EffectiveReleaseSource) -> SourceView {
    SourceView {
        protocol: "vibe-self-source/1",
        source: effective.source.kind(),
        location: effective.source.location(),
        from: effective.origin.describe(),
        row: effective.row().map(|row| RowView {
            prefix: row.key.what().to_owned(),
            target: row.target.display(),
            layer: row.from.layer.label(),
        }),
    }
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#release-sources")]
pub(in crate::commands::vvm) fn run_release_source_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    command: Option<VvmSourceCommand>,
) -> Result<()> {
    // No subcommand keeps its meaning: the running source tree.
    let Some(command) = command else {
        return provenance::run_source_cmd(ctx, env);
    };
    match command {
        VvmSourceCommand::Show => show(ctx, &release_source::active()?),
        VvmSourceCommand::Set(args) => set(ctx, env, args),
        VvmSourceCommand::Reset => store(ctx, SelfConfig::default()),
    }
}

fn show(ctx: &output::Context, effective: &EffectiveReleaseSource) -> Result<()> {
    let shown = view(effective);
    if ctx.is_json() {
        return ctx.emit_json(&shown);
    }
    let mut text = format!(
        "release source: {} {}\n  from  {}",
        shown.source, shown.location, shown.from
    );
    match &shown.row {
        Some(row) => text.push_str(&format!(
            "\n  remap {} → {} ({} layer; `vibe remap explain` shows it)",
            row.prefix, row.target, row.layer
        )),
        None if effective.source.kind() == "gitverse" => {
            text.push_str("\n  read through the GitVerse REST API")
        }
        None => {}
    }
    ctx.summary(&text);
    Ok(())
}

fn set(ctx: &output::Context, env: &VvmEnv, args: VvmSourceSetArgs) -> Result<()> {
    let kind: ReleaseSourceKind = args.kind.parse()?;
    // A relative `--path` is the directory the user means from here.
    let path = args.path.map(|path| match (&env.cwd, path.is_absolute()) {
        (Some(cwd), false) => cwd.join(path),
        _ => path,
    });
    let table = match kind {
        ReleaseSourceKind::Github => SelfConfig {
            source: Some(kind),
            ..SelfConfig::default()
        },
        _ => SelfConfig {
            source: Some(kind),
            url: args.url,
            api: args.api,
            path,
        },
    };
    store(ctx, table)
}

/// Validate the new `[self]` table against the rest of the config, then
/// write it; an unusable source is refused and nothing is written.
fn store(ctx: &output::Context, table: SelfConfig) -> Result<()> {
    let path: PathBuf = UserConfig::default_path().context(
        "no user settings directory resolves (violates spec://org.vibevm.core/vibevm/common/PROP-019#release-sources; \
         fix: set VIBE_SETTINGS or make the user home resolvable)",
    )?;
    let mut config = UserConfig::load_from(&path)?;
    config.self_source = table;
    let resolved = release_source::resolve(None, &config, Some(&path))?;
    config
        .save_to(&path)
        .with_context(|| format!("writing `{}`", path.display()))?;
    if matches!(
        release_source::active(),
        Ok(EffectiveReleaseSource {
            origin: SourceOrigin::Environment,
            ..
        })
    ) {
        ctx.hint(&format!(
            "${} is set in this environment and still wins over `[self]`",
            release_source::RELEASE_ORIGIN_ENV
        ));
    }
    show(ctx, &resolved)
}

#[derive(Serialize)]
struct ReleasesView {
    protocol: &'static str,
    source: SourceView,
    releases: Vec<ReleaseRow>,
    truncated: bool,
}

#[derive(Serialize)]
struct ReleaseRow {
    version: String,
    installed: bool,
    active: bool,
    running: bool,
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#release-sources")]
pub(in crate::commands::vvm) fn run_releases_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    args: VvmReleasesArgs,
) -> Result<()> {
    let effective = release_source::active()?;
    let downloader = HttpDownloader::for_invocation()?;
    let api = ApiClient::new(downloader.admission())?;
    let releases = Releases::new(&effective, &api, vibe_core::remap::active())?;
    if env.offline
        && !matches!(
            effective.source,
            release_source::ReleaseSource::Localdir { .. }
        )
    {
        env.refuse_offline("self:releases", &effective.source.location())?;
    }
    let versions = releases.list()?;
    let store = env.store()?;
    let state = store.load_state()?;
    let tag = |record: &super::super::model::InstallRecord| {
        (record.origin == Origin::Binary && record.kind == Kind::Tag).then(|| record.id.clone())
    };
    let installed: Vec<String> = state.installs.iter().filter_map(tag).collect();
    let active = store.active()?.as_ref().and_then(tag);
    let running = provenance::running_record(&store)?.as_ref().and_then(tag);
    let total = versions.len();
    let shown = if args.all { total } else { RECENT.min(total) };
    let rows: Vec<ReleaseRow> = versions
        .iter()
        .take(shown)
        .map(|version| {
            let version = version.to_string();
            ReleaseRow {
                installed: installed.contains(&version),
                active: active.as_deref() == Some(version.as_str()),
                running: running.as_deref() == Some(version.as_str()),
                version,
            }
        })
        .collect();
    let document = ReleasesView {
        protocol: "vibe-self-releases/1",
        source: view(&effective),
        releases: rows,
        truncated: shown < total,
    };
    if ctx.is_json() {
        return ctx.emit_json(&document);
    }
    let mut text = format!(
        "releases at {} {} ({})",
        document.source.source, document.source.location, document.source.from
    );
    if document.releases.is_empty() {
        text.push_str("\n  none");
    }
    for row in &document.releases {
        let mut marks = Vec::new();
        if row.active {
            marks.push("active");
        }
        if row.running {
            marks.push("running");
        }
        if row.installed && !row.active {
            marks.push("installed");
        }
        let mark = if row.active { '*' } else { ' ' };
        text.push_str(&format!("\n{mark} {}", row.version));
        if !marks.is_empty() {
            text.push_str(&format!("  ({})", marks.join(", ")));
        }
    }
    if document.truncated {
        text.push_str(&format!(
            "\n  … {} more; `--all` lists every release",
            total - shown
        ));
    }
    ctx.summary(&text);
    Ok(())
}
