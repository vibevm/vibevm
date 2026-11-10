//! `vibe registry mirror <source> --into <target>` — save a registry into a
//! `monorepo` or `localdir` registry, then keep it in sync (PROP-002
//! `#registry-mirror`).
//!
//! The run is `list → plan → fetch → publish`:
//!
//! 1. **list** the source's versions — from its index (`primary.jsonl`,
//!    with each version's content hash, frozen and yanked flags), from its
//!    own tree when it is itself a layout registry, from the tags of the
//!    packages `--package` names, or, without an index and without
//!    `--package`, from the `v<semver>` tags of every package repository
//!    the host's organisation listing names (`org_api`);
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

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::manifest::{Manifest, RegistrySection};
use vibe_publish::git_publish::redact_credentials;
use vibe_publish::layout_publish::{
    BatchConfig, GitTree, LayoutRegistry, PublishItem, publish_batch, read_target,
};
use vibe_wire::generated::registry_mirror_report::{MirrorAction, RegistryMirrorReport};

use crate::cli::RegistryMirrorArgs;
use crate::output;

mod config;
mod git_source;
mod history;
mod org_api;
mod plan;
mod record;
mod selection;
mod source;
mod target;
mod templates;

use history::{history_bundles, mirror_record};
use plan::{Decision, plan_versions};
use selection::Selection;
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
    // The target is read first because it is what carries the selection a
    // bare re-run repeats (`#MIRROR-CONFIG`): what the source is asked to
    // list depends on it.
    let tree = super::observed_stage(ctx, "Reading the target registry", || {
        read_target(&target.layout).map_err(anyhow::Error::from)
    })?;
    let (target_index, target_state) =
        vibe_index::layout::load_or_project(tree.root(), &target.identity())
            .context("projecting the target registry's index")?;
    let recorded = record::MirrorRecord::read(tree.root(), &source_reg.name)?;
    let drops: Vec<PathBuf> = vibe_core::remap::active()
        .package_drops()
        .into_iter()
        .map(Path::to_path_buf)
        .collect();
    let selection = selection::resolve(
        args.config.as_deref(),
        &args.packages,
        &args.groups,
        recorded.as_ref().and_then(|r| r.select.as_ref()),
        &drops,
        local_root(&target.layout),
    )?;
    announce(ctx, &selection);

    let reader = SourceReader::open(
        &source_reg,
        &effective.mirrors,
        &operation,
        root_offline,
        &progress,
    )?;
    refuse_templates_without_a_tree(&selection, &reader, &source_reg.name)?;
    let listing = super::observed_stage(ctx, "Listing the source registry", || {
        reader.list(&selection)
    })?;

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
    let mut files = mirror_record(
        &source_reg,
        &planned,
        selection.to_record(),
        tree.root(),
        scratch.path(),
    )?;
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
    // The snapshot rides the same batch as the versions, so
    // `#MIRROR-TEMPLATE-SNAPSHOT`'s "its digests are fixed there" is the
    // git commit itself. An alarm inside it stops the run here, before
    // the batch is assembled and so before anything is written.
    let snapshots = match &reader {
        SourceReader::Layout(opened) => {
            super::observed_stage(ctx, "Snapshotting the templates' inputs", || {
                templates::snapshots(ctx, &selection, opened.root(), tree.root(), scratch.path())
            })?
        }
        // Refused before the listing by `refuse_templates_without_a_tree`,
        // so a non-layout source has no `[[template]]` row to answer for.
        SourceReader::PerPackage { .. } => Vec::new(),
    };
    let snapshot_count = snapshots.len();
    files.extend(snapshots);
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
        // A snapshot file the target does not hold is a write the batch
        // would make, so a dry run that selected a new template version
        // must not report the index as untouched.
        report.index_refreshed = recorded > 0
            || snapshot_count > 0
            || target_state != vibe_core::layout_registry::IndexState::Current;
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
        // No run without `--prune` ever deletes anything, however narrow
        // the selection became (PROP-075 `#MIRROR-NO-SILENT-PRUNE`), and
        // `--prune` does not exist yet.
        removed: Vec::new(),
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

/// Say where the selection came from, and where a payload projection
/// would write, before anything is listed.
///
/// A declarative selection is otherwise invisible: an operator who ran
/// `--config` cannot tell from the version rows alone whether the file was
/// read, and one who ran bare cannot tell that the target's recorded
/// selection is what narrowed the run.
fn announce(ctx: &output::Context, selection: &Selection) {
    match selection.origin() {
        selection::Origin::Flags => {}
        selection::Origin::Config => ctx.step("Selecting from `--config`"),
        selection::Origin::Recorded => {
            ctx.step(
                "Selecting as the target's mirror record says, adding only what newly matches",
            );
        }
    }
    if let Some(payloads) = selection.payloads() {
        let budget = payloads
            .max_total
            .map(|bytes| format!(", budget {bytes} byte(s)"))
            .unwrap_or_else(|| ", no budget".to_string());
        ctx.step(&format!(
            "Projecting payloads into the drop `{}`{budget}",
            payloads.into.display()
        ));
    }
}

/// The registry target's own directory, when it has one on this machine:
/// a `localdir` root or a working copy. A `monorepo` is a url, so there is
/// nothing local a payload drop could collide with.
fn local_root(target: &vibe_publish::layout_publish::LayoutTarget) -> Option<&Path> {
    use vibe_publish::layout_publish::LayoutTarget;
    match target {
        LayoutTarget::Localdir { root } => Some(root.as_path()),
        LayoutTarget::WorkingCopy { path } => Some(path.as_path()),
        LayoutTarget::Monorepo { .. } => None,
    }
}

/// A snapshot is written from a template package's own tree, which only a
/// directory-shaped source has (`TemplateLayer::for_directory`). A
/// per-package Git registry answers a version list with a tag list, so
/// rendering from one is deferred (PROP-075 `#TEMPLATE-DISCOVERY`) and a
/// `[[template]]` row against it is refused **by name**, before anything
/// is listed or written.
fn refuse_templates_without_a_tree(
    selection: &Selection,
    reader: &SourceReader,
    source: &str,
) -> Result<()> {
    if selection.templates().is_empty() || matches!(reader, SourceReader::Layout(_)) {
        return Ok(());
    }
    let rows = selection
        .templates()
        .iter()
        .map(selection::TemplateSelection::package)
        .collect::<Vec<_>>()
        .join(", ");
    bail!(
        "the selection carries `[[template]]` row(s) for {rows}, and the source `{source}` is a \
         per-package Git registry: a template snapshot is written from the template package's \
         own tree, which only a `monorepo` or `localdir` source has \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DISCOVERY; \
         fix: mirror the template out of a layout registry, or drop the `[[template]]` row(s) \
         and copy the package with `[select]`)"
    )
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
    fn flag_selection_admits_by_package_and_group() {
        let none = Selection::from_flags(&[], &[]).unwrap();
        assert!(none.admits("org.a", "x"));
        let one = Selection::from_flags(&["org.a/x".into()], &[]).unwrap();
        assert!(one.admits("org.a", "x"));
        assert!(!one.admits("org.a", "y"));
        let group = Selection::from_flags(&[], &["org.b".into()]).unwrap();
        assert!(group.admits("org.b", "z"));
        assert!(!group.admits("org.a", "x"));
        assert!(Selection::from_flags(&["no-slash".into()], &[]).is_err());
    }
}
