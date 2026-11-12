//! `vibe registry alias <registry> add|remove|list` — the maintainer's
//! edits to the short names a directory-layout registry ISSUES
//! (PROP-008 §2.6).
//!
//! Since the owner's ruling of 2026-11-12 these rows are the WHOLE
//! short-name namespace a client resolves through: a package this
//! registry carries is reachable as `<group>/<name>` always, and as a
//! bare `<name>` only once a row here issues that spelling. So
//! `node = "org.vibevm.node/node"` — a row the earlier design refused
//! outright — is the ordinary edit.
//!
//! The table lives at `<registry>/_index/aliases.toml` and is the
//! registry OWNER's: whoever can publish into the registry can say what
//! its short spellings mean, and a package author cannot. That is the
//! whole point of the owner's ruling — a short name is a registry-wide
//! namespace, and letting packages claim them would be «an
//! uncontrolled war for short names» — nor is a package reachable by
//! its own name merely because it bears it: «мы не можем дать людям
//! самим выбирать любое имя».
//!
//! **The table and the built index move in one commit.** The edit rides
//! the ordinary publish batch, as a mirror's record and its history
//! bundles do: one `BatchConfig` whose `files` carries the new table,
//! no items and no facts, so a monorepo writes table and `_index/`
//! together and a localdir writes both or neither. A table that landed
//! without its projection would be an alias that silently did nothing,
//! which is the one failure mode a short-name feature must not have.
//!
//! **A maintainer is refused what a reader is merely warned about.**
//! The whole table is validated against the registry's own index before
//! anything is written — including rows that were sound when they were
//! authored and have since been outgrown, because the person editing
//! the table now is the one who can act on them.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name");

use anyhow::{Context as _, Result, bail};
use serde::Serialize;
use vibe_core::layout_registry::{ALIAS_TABLE_FILE, INDEX_DIR, index_dir};
use vibe_core::manifest::{Manifest, RegistrySection, RegistryType};
use vibe_index::aliases::{self, AliasTable};
use vibe_index::types::AliasTarget;
use vibe_publish::layout_publish::{BatchConfig, LayoutRegistry, LayoutTarget, publish_batch};

use super::layout_root;
use crate::cli::{RegistryAliasAction, RegistryAliasArgs};
use crate::output;

#[derive(Debug, Serialize)]
struct AliasRow {
    alias: String,
    target: String,
}

#[derive(Debug, Serialize)]
struct AliasReport {
    registry: String,
    action: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    aliases: Vec<AliasRow>,
    index_refreshed: bool,
    dry_run: bool,
}

pub(super) fn run_alias(
    ctx: &output::Context,
    args: RegistryAliasArgs,
    root_offline: bool,
) -> Result<()> {
    let project_root = super::resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(Manifest::FILENAME);
    let manifest = match manifest_path.exists() {
        true => Manifest::read(&manifest_path)
            .with_context(|| format!("reading `{}`", manifest_path.display()))?,
        false => Manifest::default(),
    };
    let effective = super::effective_registries(&project_root, &manifest)?;
    let Some(section) = effective
        .registries
        .iter()
        .find(|r| r.name == args.registry)
        .cloned()
    else {
        bail!(
            "no `[[registry]]` named `{}` is configured — `vibe registry list` shows the ones \
             that are",
            args.registry
        );
    };
    if !layout_root::is_layout(&section) {
        bail!(
            "registry `{}` is a `{}` registry — an alias table lives in a directory-layout \
             registry's own `{INDEX_DIR}/{ALIAS_TABLE_FILE}`, so only `monorepo` and `localdir` \
             registries carry one. A per-package git or org registry keeps its index in a \
             separate checkout; edit that one with `vibe-index alias`.",
            section.name,
            section.checked_type()?.as_str(),
        );
    }
    let operation = vibe_package_source::source_operation(
        &vibe_package_source::PackageSourceOptions::default(),
        &manifest,
        &project_root,
        vibe_package_source::deferred_user_registry(crate::output::seeds_default_registry()),
    )?;
    let opened = layout_root::open(
        &section,
        &effective.mirrors,
        &operation,
        root_offline,
        &ctx.progress(),
    )?;
    let identity = opened.identity().clone();
    let (index, _) = vibe_index::layout::load_or_project(opened.root(), &identity)
        .context("projecting the registry's index")?;
    let mut table = AliasTable::read(&index_dir(opened.root()))?;

    let (action, alias, target) = match args.action {
        RegistryAliasAction::List => {
            return render(
                ctx,
                &report(&section.name, "list", None, None, &table, false, false),
            );
        }
        RegistryAliasAction::Remove { alias } => {
            if table.remove(&alias).is_none() {
                bail!(
                    "registry `{}` issues no short name `{alias}` — `vibe registry alias {} list` \
                     shows the ones it does",
                    section.name,
                    section.name,
                );
            }
            ("remove", Some(alias), None)
        }
        RegistryAliasAction::Add {
            alias,
            target,
            replace,
        } => {
            let target = parse_target(&target)?;
            let spelled = aliases::spell(&target);
            if let Some(held) = table.get(&alias) {
                if held == &target {
                    // A mutation that changes nothing writes nothing: no
                    // file, no reprojection, no commit.
                    return render(
                        ctx,
                        &report(
                            &section.name,
                            "unchanged",
                            Some(alias),
                            Some(spelled),
                            &table,
                            false,
                            args.dry_run,
                        ),
                    );
                }
                if !replace {
                    bail!(
                        "`{alias}` already means {} in registry `{}` — one spelling has one \
                         meaning, so re-pointing it is a deliberate act: pass `--replace` to \
                         change it, or pick another spelling",
                        aliases::spell(held),
                        section.name,
                    );
                }
            }
            table.insert(alias.clone(), target);
            ("add", Some(alias), Some(spelled))
        }
    };

    refuse_faults(&section.name, &index, &table)?;

    let staged = tempfile::Builder::new()
        .prefix("vibe-aliases-")
        .suffix(".toml")
        .tempfile()?;
    // An emptied table is written as an empty document rather than
    // deleted: a batch adds and replaces files, it does not remove them.
    // The listing digest looks at the ROWS and not at the file, so an
    // empty document and no document read the same — a registry that
    // drops its last alias stops asking anything of its older clients.
    std::fs::write(staged.path(), table.render())?;

    let outcome = super::observed_stage(ctx, "Writing the alias table", || {
        publish_batch(
            &BatchConfig {
                target: layout_target(&section)?,
                registry: LayoutRegistry {
                    name: section.name.clone(),
                    url: section.url.clone(),
                },
                items: Vec::new(),
                facts: Vec::new(),
                files: vec![(
                    format!("{INDEX_DIR}/{ALIAS_TABLE_FILE}"),
                    staged.path().to_path_buf(),
                )],
                removed: Vec::new(),
                message: Some(message(action, alias.as_deref(), target.as_deref())),
                dry_run: args.dry_run,
            },
            ctx.progress().cancellation(),
        )
        .map_err(anyhow::Error::from)
    })?;

    render(
        ctx,
        &report(
            &section.name,
            action,
            alias,
            target,
            &table,
            outcome.index_refreshed,
            args.dry_run,
        ),
    )
}

/// Refuse the whole table when any row would not stand — including rows
/// written long ago that the registry has since outgrown, because the
/// maintainer editing it now is the one who can act on them.
pub(crate) fn refuse_faults(
    registry: &str,
    index: &vibe_index::index::Index,
    table: &AliasTable,
) -> Result<()> {
    let faults = aliases::check(index, table);
    if faults.is_empty() {
        return Ok(());
    }
    let lines: Vec<String> = faults.iter().map(ToString::to_string).collect();
    bail!(
        "the alias table of registry `{registry}` would not stand:\n  {}",
        lines.join("\n  "),
    )
}

fn layout_target(section: &RegistrySection) -> Result<LayoutTarget> {
    Ok(match section.checked_type()? {
        RegistryType::Localdir => LayoutTarget::Localdir {
            root: vibe_registry::localdir_path(&section.url)?,
        },
        RegistryType::Monorepo => LayoutTarget::Monorepo {
            url: section.url.clone(),
            branch: section.r#ref.clone(),
        },
        other => bail!(
            "registry `{}` is a `{}` registry, not a monorepo or localdir",
            section.name,
            other.as_str()
        ),
    })
}

fn message(action: &str, alias: Option<&str>, target: Option<&str>) -> String {
    match (action, alias, target) {
        ("add", Some(alias), Some(target)) => format!("alias: `{alias}` means {target}"),
        ("remove", Some(alias), _) => format!("alias: withdraw `{alias}`"),
        _ => "alias: update the table".to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn report(
    registry: &str,
    action: &'static str,
    alias: Option<String>,
    target: Option<String>,
    table: &AliasTable,
    index_refreshed: bool,
    dry_run: bool,
) -> AliasReport {
    AliasReport {
        registry: registry.to_owned(),
        action,
        alias,
        target,
        aliases: table
            .iter()
            .map(|(alias, target)| AliasRow {
                alias: alias.clone(),
                target: aliases::spell(target),
            })
            .collect(),
        index_refreshed,
        dry_run,
    }
}

fn render(ctx: &output::Context, report: &AliasReport) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(report);
    }
    if !ctx.is_quiet() {
        match (report.action, &report.alias, &report.target) {
            ("add", Some(alias), Some(target)) => {
                println!("`{alias}` now means {target} in {}", report.registry);
            }
            ("unchanged", Some(alias), Some(target)) => {
                println!("`{alias}` already means {target} in {}", report.registry);
            }
            ("remove", Some(alias), _) => {
                println!("withdrew the alias `{alias}` from {}", report.registry);
            }
            _ => {}
        }
    }
    for row in &report.aliases {
        println!("{} -> {}", row.alias, row.target);
    }
    Ok(())
}

fn parse_target(spelling: &str) -> Result<AliasTarget> {
    let parsed = vibe_core::PackageRef::parse(spelling)?;
    let Some(group) = parsed.group else {
        bail!(
            "`{spelling}` carries no group — an alias target is always the fully qualified \
             `<group>/<name>`, because supplying the group is the whole point of the alias"
        );
    };
    if !matches!(parsed.version, vibe_core::VersionSpec::Latest) {
        bail!(
            "`{spelling}` pins a version — an alias is a spelling of a PACKAGE, so version \
             selection stays the operator's"
        );
    }
    Ok(AliasTarget {
        group,
        name: parsed.name.as_str().to_owned(),
    })
}

/// The alias table a `vibe registry mirror` run should carry from
/// `source` into `target`, and the rows it must not touch.
///
/// Exposed here, beside the maintainer's own edits, because the merge
/// rule is a statement about ONE table with two authors and belongs
/// with the table rather than with the mirror pipeline.
pub(crate) fn mirrored(
    source: &AliasTable,
    target: &AliasTable,
    recorded: &[(String, String)],
    carried: impl Fn(&AliasTarget) -> bool,
) -> MirrorMerge {
    let recorded_target = |alias: &str| {
        recorded
            .iter()
            .find(|(a, _)| a == alias)
            .map(|(_, t)| t.clone())
    };
    let mut merged = target.clone();
    let mut merge = MirrorMerge::default();
    for (alias, row) in source.iter() {
        if !carried(row) {
            // The target does not have the package, so the spelling
            // would point at nothing. Dropping it is not silent: the
            // run says which rows it left behind and why.
            merge.dropped.push((alias.clone(), aliases::spell(row)));
            continue;
        }
        match (
            merged.get(alias).map(aliases::spell),
            recorded_target(alias),
        ) {
            // Untouched since the last run — the source is still this
            // row's author, so the source's value wins.
            (Some(held), Some(last)) if held == last => {
                merge.updated(alias, row, &mut merged, held != aliases::spell(row));
            }
            // The mirror owner wrote or edited this row. The source
            // never overwrites a local hand; a disagreement is reported
            // and the local value stands.
            (Some(held), _) => {
                if held != aliases::spell(row) {
                    merge
                        .conflicts
                        .push((alias.clone(), held, aliases::spell(row)));
                }
            }
            // New to this mirror.
            (None, _) => merge.updated(alias, row, &mut merged, true),
        }
    }
    // A row the source retired goes only if the mirror owner never
    // touched it; one they edited is theirs to retire.
    for (alias, last) in recorded {
        if source.get(alias).is_some() {
            continue;
        }
        if merged.get(alias).map(aliases::spell).as_deref() == Some(last.as_str()) {
            merged.remove(alias);
            merge.withdrawn.push(alias.clone());
        }
    }
    merge.changed = &merged != target;
    merge.table = merged;
    merge
}

/// What one mirror run did to the target's alias table.
#[derive(Debug, Default, Clone)]
pub(crate) struct MirrorMerge {
    pub(crate) table: AliasTable,
    /// Rows taken from the source this run.
    pub(crate) copied: Vec<(String, String)>,
    /// Source rows left behind because the target does not carry the
    /// package they name.
    pub(crate) dropped: Vec<(String, String)>,
    /// Rows the mirror owner edited and the source now disagrees with:
    /// `(alias, the mirror's value, the source's value)`. The mirror's
    /// value stands.
    pub(crate) conflicts: Vec<(String, String, String)>,
    /// Rows the source retired and the mirror had not edited.
    pub(crate) withdrawn: Vec<String>,
    /// Whether the merged table differs from the target's.
    changed: bool,
}

impl MirrorMerge {
    fn updated(&mut self, alias: &str, row: &AliasTarget, into: &mut AliasTable, changed: bool) {
        if changed {
            into.insert(alias.to_owned(), row.clone());
        }
        self.copied.push((alias.to_owned(), aliases::spell(row)));
    }

    /// Take a row back out of the merged table — a row that would not
    /// stand against the target's own index. It stops being carried as
    /// well as stops being written, so the next run does not read its
    /// absence as a mirror-owner deletion.
    pub(crate) fn drop_row(&mut self, alias: &str) {
        if let Some(target) = self.table.remove(alias) {
            self.dropped
                .push((alias.to_owned(), aliases::spell(&target)));
        }
        self.copied.retain(|(held, _)| held != alias);
        self.changed = true;
    }

    /// Whether this run has anything to write: a table it changed, or
    /// rows it must newly record as carried.
    pub(crate) fn changed(&self) -> bool {
        self.changed
    }

    /// Every row the run carried, as the mirror record should remember
    /// them: this is what the NEXT run compares the target against to
    /// tell a mirror-owner edit from an untouched copy.
    pub(crate) fn recorded(&self) -> Vec<(String, String)> {
        self.copied.clone()
    }
}

#[cfg(test)]
#[path = "alias/tests.rs"]
mod tests;
