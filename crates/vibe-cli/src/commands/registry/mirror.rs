//! `vibe registry mirror <source> --into <target>` — save a registry into a
//! `monorepo` or `localdir` registry, then keep it in sync (PROP-002
//! `#registry-mirror`).
//!
//! The run is `list → plan → fetch → publish`:
//!
//! 1. **list** the source's versions — from its index (`primary.jsonl`,
//!    with each version's content hash, frozen and yanked flags), from its
//!    own tree when it is itself a layout registry, or, without an index,
//!    from the tags of the packages `--package` names;
//! 2. **plan** each against the target's projection: absent → copy; the
//!    same content hash → unchanged (not even fetched); a different one →
//!    an alarm when the version is frozen, an update with
//!    `--update-snapshots`, otherwise reported and left;
//! 3. **fetch** what is to be copied through the operation's controlled
//!    transport, source policy and remap Locator, and verify the bytes
//!    against the hash the source's index records;
//! 4. **publish** every copied version, with its provenance and the
//!    source's yanks and tombstones, as ONE batch of the layout publisher —
//!    one commit and one push for a monorepo.
//!
//! An alarm stops the run before anything is written.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::manifest::{Manifest, RegistrySection};
use vibe_publish::git_publish::redact_credentials;
use vibe_publish::layout_publish::{
    BatchConfig, GitTree, LayoutRegistry, PublishItem, publish_batch, read_target,
};
use vibe_wire::generated::registry_mirror_report::{MirrorAction, RegistryMirrorReport};

use crate::cli::RegistryMirrorArgs;
use crate::output;

mod git_source;
mod history;
mod plan;
mod record;
mod source;
mod target;

use history::{history_bundles, mirror_record};
use plan::{Decision, plan_versions};
use source::SourceReader;
use target::resolve_target;

pub(super) fn run_mirror(
    ctx: &output::Context,
    args: RegistryMirrorArgs,
    root_offline: bool,
) -> Result<()> {
    let project_root = super::resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(Manifest::FILENAME);
    let manifest = if manifest_path.exists() {
        Manifest::read(&manifest_path)
            .with_context(|| format!("reading `{}`", manifest_path.display()))?
    } else {
        Manifest::default()
    };
    let effective = super::effective_registries(&project_root, &manifest)?;
    let source_reg = effective
        .registries
        .iter()
        .find(|r| r.name == args.source)
        .cloned()
        .ok_or_else(|| {
            anyhow!(
                "no `[[registry]]` named `{}` in `vibe.toml`, `.vibe/registry.local.toml` or \
                 `~/.vibe/registry.toml`; configured: {} \
                 (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror; \
                 fix: name a configured registry, or add one with `vibe registry add`)",
                args.source,
                names(&effective.registries)
            )
        })?;
    let target = resolve_target(&args, &effective.registries)?;
    let filters = Filters::parse(&args.packages, &args.groups)?;

    let operation = vibe_package_source::source_operation(
        &vibe_package_source::PackageSourceOptions::default(),
        &manifest,
        &project_root,
        vibe_package_source::deferred_user_registry(crate::output::seeds_default_registry()),
    )?;
    ctx.heading(&format!(
        "Mirroring registry `{}` ({}) → {}{}",
        source_reg.name,
        redact_credentials(&source_reg.url),
        target.shown,
        if args.dry_run { " [dry-run]" } else { "" }
    ));

    let progress = ctx.progress();
    let reader = SourceReader::open(
        &source_reg,
        &effective.mirrors,
        &operation,
        root_offline,
        &progress,
    )?;
    let listing =
        super::observed_stage(ctx, "Listing the source registry", || reader.list(&filters))?;
    let tree = super::observed_stage(ctx, "Reading the target registry", || {
        read_target(&target.layout).map_err(anyhow::Error::from)
    })?;
    let (target_index, target_state) =
        vibe_index::layout::load_or_project(tree.root(), &target.identity())
            .context("projecting the target registry's index")?;

    let fetch_root = tempfile::tempdir().context("creating a fetch directory")?;
    let fetching = ctx.progress().task("Fetching new versions from the source");
    let planned = plan_versions(
        &listing.versions,
        &target_index,
        args.update_snapshots,
        !args.dry_run,
        |version| reader.fetch(version, fetch_root.path()),
    );
    let planned = match planned {
        Ok(planned) => {
            fetching.finish();
            planned
        }
        Err(error) => {
            fetching.fail(error.to_string());
            return Err(error);
        }
    };

    let alarms = planned
        .iter()
        .filter(|p| matches!(p.decision, Decision::Alarm(_)))
        .count();
    let mut items = Vec::new();
    let mut copies = 0usize;
    for p in &planned {
        let replace = match p.decision {
            Decision::Copy => false,
            Decision::Update => true,
            _ => continue,
        };
        let Some(fetched) = &p.fetched else {
            if args.dry_run {
                // Decided without fetching: a dry run copies nothing.
                copies += 1;
                continue;
            }
            bail!(
                "internal: {}/{}@{} was planned for copying but not fetched",
                p.version.group,
                p.version.name,
                p.version.version
            );
        };
        items.push(PublishItem {
            group: p.version.group.as_str().to_string(),
            name: p.version.name.clone(),
            version: p.version.version.clone(),
            tree: fetched.tree.clone(),
            provenance: Some(fetched.provenance.clone()),
            git: fetched.git.as_ref().map(|g| GitTree {
                repo: g.store.clone(),
                reference: g.reference.clone(),
                tree: g.facts.tree.clone(),
            }),
            replace,
        });
    }
    let facts = source::carried_facts(&listing, &planned, &source_reg.name);
    let scratch = tempfile::tempdir().context("creating a scratch directory")?;
    let mut files = mirror_record(&source_reg, &planned, tree.root(), scratch.path())?;
    let bundles = if args.keep_history {
        history_bundles(
            ctx,
            &reader,
            &source_reg,
            &planned,
            tree.root(),
            scratch.path(),
        )?
    } else {
        Vec::new()
    };
    let bundle_count = bundles.len();
    files.extend(bundles);
    drop(tree);

    let unavailable = planned
        .iter()
        .filter(|p| matches!(p.decision, Decision::Unavailable(_)))
        .count();
    let mut report = RegistryMirrorReport {
        ok: alarms == 0 && unavailable == 0,
        command: "registry:mirror".to_owned(),
        source: source_reg.name.clone(),
        source_url: redact_credentials(&source_reg.url),
        listing: listing.kind.clone(),
        target: target.shown.clone(),
        target_kind: target.kind.clone(),
        versions: planned.iter().map(plan::report_row).collect(),
        facts_recorded: 0,
        index_refreshed: false,
        retried: false,
        dry_run: args.dry_run,
        commit: None,
        bundles: u32::try_from(bundle_count).unwrap_or(u32::MAX),
    };
    if alarms > 0 {
        render(ctx, &report)?;
        bail!(
            "vibe registry mirror: {alarms} version(s) of `{}` differ from the copy in the target \
             in a way that may not happen (a frozen version changed, or the bytes disagree with \
             the source's index); nothing was written \
             (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror; \
             fix: investigate the listed versions at the source — a frozen version must never \
             change — then re-run)",
            source_reg.name
        );
    }

    if args.dry_run {
        // Decided without writing: the facts the batch would record are the
        // publications plus the carried facts the target does not reflect.
        let new_facts = facts
            .iter()
            .filter(|fact| fact_is_new(fact, &target_index))
            .count();
        let recorded = copies + items.len() + new_facts;
        report.facts_recorded = u32::try_from(recorded).unwrap_or(u32::MAX);
        report.index_refreshed =
            recorded > 0 || target_state != vibe_core::layout_registry::IndexState::Current;
        return render(ctx, &report);
    }
    let batch = BatchConfig {
        target: target.layout.clone(),
        registry: LayoutRegistry {
            name: target.name.clone(),
            url: target.url.clone(),
        },
        message: Some(commit_message(&source_reg, &items)),
        items,
        facts,
        files,
        dry_run: false,
    };
    let outcome = super::observed_stage(ctx, "Writing into the target registry", || {
        publish_batch(&batch, ctx.progress().cancellation()).map_err(anyhow::Error::from)
    })?;
    report.facts_recorded = u32::try_from(outcome.facts_recorded).unwrap_or(u32::MAX);
    report.index_refreshed = outcome.index_refreshed;
    report.retried = outcome.retried;
    report.commit = outcome.commit.clone();
    for (row, item) in report.versions.iter_mut().filter_map(|row| {
        outcome
            .items
            .iter()
            .find(|i| {
                format!("{}/{}", i.group, i.name) == row.package
                    && i.version.to_string() == row.version
            })
            .map(|item| (row, item))
    }) {
        if let Some(tree) = &item.tree {
            row.tree = Some(tree.clone());
            row.tree_verified = Some(true);
        }
    }
    if unavailable > 0 {
        render(ctx, &report)?;
        bail!(
            "vibe registry mirror: the source `{}` could not serve {unavailable} version(s); \
             everything else was copied \
             (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror; \
             fix: check the listed versions at the source — a repository or tag the index names \
             is missing or denied — then re-run: only what is still missing is copied)",
            source_reg.name
        );
    }
    render(ctx, &report)
}

/// Whether a carried fact is news to the target (the batch applies the
/// same rule when it writes).
fn fact_is_new(fact: &vibe_index::journal::Event, target: &vibe_index::index::Index) -> bool {
    match fact {
        vibe_index::journal::Event::Yanked {
            group,
            name,
            version,
            ..
        } => !target
            .get(group, name)
            .and_then(|pkg| pkg.versions.iter().find(|v| &v.version == version))
            .is_some_and(|entry| entry.yanked),
        vibe_index::journal::Event::Buried { name, .. } => !target.tombstones.contains_key(name),
        _ => true,
    }
}

/// `--package` / `--group` filters.
#[derive(Debug, Default)]
pub(super) struct Filters {
    packages: Vec<(String, String)>,
    groups: Vec<String>,
}

impl Filters {
    fn parse(packages: &[String], groups: &[String]) -> Result<Self> {
        let mut parsed = Vec::new();
        for raw in packages {
            let (group, name) = raw.split_once('/').ok_or_else(|| {
                anyhow!(
                    "`--package {raw}` is not `<group>/<name>` \
                     (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror; \
                     fix: name the package with its group, e.g. `org.vibevm.world/wal`)"
                )
            })?;
            vibe_core::Group::parse(group)?;
            parsed.push((group.to_string(), name.to_string()));
        }
        for group in groups {
            vibe_core::Group::parse(group)?;
        }
        Ok(Filters {
            packages: parsed,
            groups: groups.to_vec(),
        })
    }

    pub(super) fn admits(&self, group: &str, name: &str) -> bool {
        let by_package =
            self.packages.is_empty() || self.packages.iter().any(|(g, n)| g == group && n == name);
        let by_group = self.groups.is_empty() || self.groups.iter().any(|g| g == group);
        by_package && by_group
    }

    pub(super) fn packages(&self) -> &[(String, String)] {
        &self.packages
    }
}

fn names(registries: &[RegistrySection]) -> String {
    if registries.is_empty() {
        return "(none)".to_string();
    }
    registries
        .iter()
        .map(|r| r.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn commit_message(source: &RegistrySection, items: &[PublishItem]) -> String {
    let what = match items {
        [] => "index".to_string(),
        [one] => format!("{}/{}@{}", one.group, one.name, one.version),
        many => format!("{} versions", many.len()),
    };
    format!(
        "mirror: {what} from `{}` ({})",
        source.name,
        redact_credentials(&source.url)
    )
}

fn render(ctx: &output::Context, report: &RegistryMirrorReport) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(report);
    }
    let count = |action: MirrorAction| {
        report
            .versions
            .iter()
            .filter(|v| v.action == action)
            .count()
    };
    for row in &report.versions {
        if row.action == MirrorAction::Unchanged {
            continue;
        }
        let verb = action_word(&row.action, report.dry_run);
        let note = row
            .note
            .as_deref()
            .map(|n| format!(" — {n}"))
            .unwrap_or_default();
        ctx.step(&format!("{}@{} {verb}{note}", row.package, row.version));
    }
    let commit = report
        .commit
        .as_deref()
        .map(|c| format!(", commit {c}"))
        .unwrap_or_default();
    ctx.summary(&format!(
        "\nvibe registry mirror: {} {}, {} {}, {} unchanged, {} snapshot(s) left as they are, \
         {} alarm(s), {} unavailable; {} fact(s) recorded{commit}.",
        count(MirrorAction::Copied),
        if report.dry_run { "to copy" } else { "copied" },
        count(MirrorAction::Updated),
        if report.dry_run {
            "to update"
        } else {
            "updated"
        },
        count(MirrorAction::Unchanged),
        count(MirrorAction::SnapshotDiffers),
        count(MirrorAction::Alarm),
        count(MirrorAction::Unavailable),
        report.facts_recorded,
    ));
    Ok(())
}

fn action_word(action: &MirrorAction, dry_run: bool) -> &'static str {
    match (action, dry_run) {
        (MirrorAction::Copied, true) => "would be copied",
        (MirrorAction::Copied, false) => "copied",
        (MirrorAction::Updated, true) => "would be updated (snapshot moved at the source)",
        (MirrorAction::Updated, false) => "updated (snapshot moved at the source)",
        (MirrorAction::Unchanged, _) => "unchanged",
        (MirrorAction::SnapshotDiffers, _) => {
            "differs at the source (snapshot; left as it is — pass --update-snapshots)"
        }
        (MirrorAction::Alarm, _) => "ALARM",
        (MirrorAction::Unavailable, _) => "unavailable at the source",
    }
}

/// The fetched copy of a version and where it came from.
pub(super) struct Fetched {
    pub tree: PathBuf,
    pub provenance: vibe_publish::layout_publish::Provenance,
    /// The fetched manifest declares the version frozen.
    pub frozen: bool,
    /// The tag's Git objects and facts (a per-package source).
    pub git: Option<source::GitOrigin>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_admit_by_package_and_group() {
        let none = Filters::parse(&[], &[]).unwrap();
        assert!(none.admits("org.a", "x"));
        let one = Filters::parse(&["org.a/x".into()], &[]).unwrap();
        assert!(one.admits("org.a", "x"));
        assert!(!one.admits("org.a", "y"));
        let group = Filters::parse(&[], &["org.b".into()]).unwrap();
        assert!(group.admits("org.b", "z"));
        assert!(!group.admits("org.a", "x"));
        assert!(Filters::parse(&["no-slash".into()], &[]).is_err());
    }
}
