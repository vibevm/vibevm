//! `vibe self source show|set|reset` and `vibe self releases`
//! (`PROP-019#release-sources`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#release-sources");

use std::path::PathBuf;

use anyhow::{Context, Result};
use specmark::spec;
use vibe_core::remap::Layer;
use vibe_core::user_config::{ReleaseSourceKind, SelfConfig, UserConfig};
use vibe_wire::generated::self_releases_report::{ReleaseRow, SelfReleasesReport};
use vibe_wire::generated::self_source_report::{
    ReleaseSourceRow, ReleaseSourceView, RemapLayerLabel, SelfSourceReport,
};

use super::super::model::{Kind, Origin};
use super::super::{VvmEnv, output, provenance};
use super::api::ApiClient;
use super::download::{Downloader, HttpDownloader};
use super::release_source::{self, EffectiveReleaseSource, SourceOrigin};
use super::releases::Releases;
use crate::cli::{VvmReleasesArgs, VvmSourceCommand, VvmSourceSetArgs};

/// How many releases `self releases` shows without `--all`.
const RECENT: usize = 10;

/// The JSON protocol name of the source view (`cli-self-source-report`),
/// carried inside the releases document too.
const SOURCE_PROTOCOL: &str = "vibe-self-source/1";

/// The JSON protocol name of the releases listing (`cli-self-releases-report`).
const RELEASES_PROTOCOL: &str = "vibe-self-releases/1";

/// The wire value of one remap layer; `Layer::label` is the same spelling
/// seen by a human, and matching on the layer keeps the two in step.
fn layer_label(layer: Layer) -> RemapLayerLabel {
    match layer {
        Layer::UserPackage => RemapLayerLabel::UserPackage,
        Layer::ProjectPackage => RemapLayerLabel::ProjectPackage,
        Layer::Local => RemapLayerLabel::Local,
        Layer::Invocation => RemapLayerLabel::Invocation,
    }
}

fn view(effective: &EffectiveReleaseSource) -> SelfSourceReport {
    ReleaseSourceView {
        protocol: SOURCE_PROTOCOL.to_owned(),
        source: effective.source.wire_kind(),
        location: effective.source.location(),
        from: effective.origin.describe(),
        row: effective.row().map(|row| ReleaseSourceRow {
            prefix: row.key.what().to_owned(),
            target: row.target.display(),
            layer: layer_label(row.from.layer),
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
        release_source::kind_label(shown.source.clone()),
        shown.location,
        shown.from
    );
    match &shown.row {
        Some(row) => text.push_str(&format!(
            "\n  remap {} → {} ({} layer; `vibe remap explain` shows it)",
            row.prefix,
            row.target,
            layer_text(&row.layer)
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
            install_root: None,
        },
    };
    store(ctx, table)
}

/// Validate the new `[self]` table against the rest of the config, then
/// write it; an unusable source is refused and nothing is written.
fn store(ctx: &output::Context, mut table: SelfConfig) -> Result<()> {
    let path: PathBuf = UserConfig::default_path().context(
        "no user settings directory resolves (violates spec://org.vibevm.core/vibevm/common/PROP-019#release-sources; \
         fix: set VIBE_SETTINGS or make the user home resolvable)",
    )?;
    let mut config = UserConfig::load_from(&path)?;
    // `[self] install_root` answers a different question — where vibe
    // installs itself (`PROP-019#install-root`) — and the release-source
    // editor is not allowed to forget it.
    table.install_root = config.self_source.install_root.clone();
    config.self_source = table;
    release_source::resolve(None, &config, Some(&path))?;
    config
        .save_to(&path)
        .with_context(|| format!("writing `{}`", path.display()))?;
    // What is in effect now is the user's file over the applied site
    // profile (`PROP-019#site-profile`): after a reset the site's `[self]`
    // decides again, and the report names it, not the built-in default.
    let site_path = vibe_core::settings::site_profile_path();
    let layers = crate::commands::vvm::site::ConfigLayers::load(
        site_path.as_deref(),
        dirs::home_dir().as_deref(),
        &mut Vec::new(),
        &mut Vec::new(),
    );
    let resolved = layers.release(None)?;
    if let (
        Some(site),
        SourceOrigin::SelfTable(Some(from)) | SourceOrigin::ArtifactsDrop(Some(from)),
    ) = (site_path.as_deref(), &resolved.origin)
        && from == site
    {
        ctx.hint(
            "the applied site profile decides the release source; `vibe self source set` \
             overrides it for this user, `vibe self apply --remove` removes the profile",
        );
    }
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

/// One remap layer spelled for a human — `Layer::label`'s four strings,
/// reached through the wire vocabulary so neither side can drift.
fn layer_text(layer: &RemapLayerLabel) -> &'static str {
    match layer {
        RemapLayerLabel::UserPackage => Layer::UserPackage.label(),
        RemapLayerLabel::ProjectPackage => Layer::ProjectPackage.label(),
        RemapLayerLabel::Local => Layer::Local.label(),
        RemapLayerLabel::Invocation => Layer::Invocation.label(),
    }
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
    let document = SelfReleasesReport {
        protocol: RELEASES_PROTOCOL.to_owned(),
        source: view(&effective),
        releases: rows,
        truncated: shown < total,
    };
    if ctx.is_json() {
        return ctx.emit_json(&document);
    }
    let mut text = format!(
        "releases at {} {} ({})",
        release_source::kind_label(document.source.source.clone()),
        document.source.location,
        document.source.from
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
