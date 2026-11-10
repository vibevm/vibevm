//! What a mirror run selects (PROP-075 `#MIRROR-CONFIG`).
//!
//! This replaces the `Filters { packages, groups }` of exact matching that
//! `vibe registry mirror` carried before: the seam the source side reads is
//! still the two methods [`Selection::admits`] and [`Selection::packages`],
//! so listing is unchanged, and everything new — `all`, `exclude`, the
//! `[[template]]` rows and the payload budget — rides beside them.
//!
//! Three things decide a selection, in this order:
//!
//! 1. `--package` / `--group` on the command line, which **override**
//!    `[select]` wholesale rather than intersecting with it, the rule
//!    `#MIRROR-CONFIG` states;
//! 2. `--config mirror.toml`;
//! 3. the selection the target recorded for this source in
//!    `_index/mirrors/<source>.toml`, so a later bare run repeats it and
//!    adds only what newly matches.
//!
//! With none of the three the selection is today's: everything the source
//! carries.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG");

use std::path::PathBuf;

use anyhow::{Result, bail};

use super::config::{self, Validated, ValidatedTemplate};
use super::record;

const SPEC: &str = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA";

/// Where a run's selection came from, which is what decides whether it is
/// written back into the mirror record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Origin {
    /// `--package` / `--group`, or neither (today's "everything").
    Flags,
    /// `--config mirror.toml`.
    Config,
    /// The `[select]` table the target already records for this source.
    Recorded,
}

/// One `[[template]]` row as a run reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TemplateSelection {
    pub group: String,
    pub name: String,
    pub versions: Vec<semver::VersionReq>,
    pub channels: Vec<String>,
    pub newest: Option<u32>,
    pub platforms: Vec<String>,
    pub archs: Vec<String>,
    pub installers: Vec<String>,
    pub payloads: bool,
}

impl TemplateSelection {
    pub(super) fn package(&self) -> String {
        format!("{}/{}", self.group, self.name)
    }
}

/// `[payloads]`, resolved: the budget, and the drop the projection writes
/// into (never the registry target — `#MIRROR-PAYLOAD-PROJECTION`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PayloadPlan {
    pub max_total: Option<u64>,
    pub into: PathBuf,
}

/// Either spelling of an `exclude` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Exclusion {
    Group(String),
    Package(String, String),
}

/// The effective selection of one run.
#[derive(Debug, Clone)]
pub(super) struct Selection {
    /// `[select] all`: every package the source carries, before excludes.
    all: bool,
    packages: Vec<(String, String)>,
    groups: Vec<String>,
    exclude: Vec<Exclusion>,
    templates: Vec<TemplateSelection>,
    payloads: Option<PayloadPlan>,
    origin: Origin,
}

impl Default for Selection {
    /// Today's default: no filter at all, so everything is admitted.
    fn default() -> Self {
        Selection {
            all: true,
            packages: Vec::new(),
            groups: Vec::new(),
            exclude: Vec::new(),
            templates: Vec::new(),
            payloads: None,
            origin: Origin::Flags,
        }
    }
}

impl Selection {
    /// `--package` / `--group`: exactly the old `Filters::parse`.
    pub(super) fn from_flags(packages: &[String], groups: &[String]) -> Result<Self> {
        let mut parsed = Vec::new();
        for raw in packages {
            let (group, name) = raw.split_once('/').ok_or_else(|| {
                anyhow::anyhow!(
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
        Ok(Selection {
            all: parsed.is_empty() && groups.is_empty(),
            packages: parsed,
            groups: groups.to_vec(),
            ..Selection::default()
        })
    }

    /// A validated `mirror.toml`, with the context only the run knows:
    /// the configured `[artifacts] packages` drops in winning order and
    /// the registry target's own root.
    ///
    /// The two refusals that need that context are here: an empty
    /// selection, which is never a silent empty run, and a payload drop
    /// that is the registry target, which would grow a monorepo by forty
    /// gigabytes.
    pub(super) fn from_config(
        config: &Validated,
        drops: &[PathBuf],
        target_root: Option<&std::path::Path>,
    ) -> Result<Self> {
        let shown = config.path.display().to_string();
        let select = &config.select;
        if !select.all
            && select.groups.is_empty()
            && select.packages.is_empty()
            && config.templates.is_empty()
        {
            bail!(
                "`{shown}` selects nothing: `[select] all` is false with no `groups`, no \
                 `packages` and no `[[template]]` row \
                 (violates {SPEC}; fix: set `all = true`, or name the groups, packages or \
                 template packages to mirror)"
            );
        }
        let payloads = resolve_payloads(config, drops, target_root)?;
        Ok(Selection {
            all: select.all,
            packages: split_all(&select.packages),
            groups: select.groups.clone(),
            exclude: select
                .exclude
                .iter()
                .map(String::as_str)
                .map(exclusion)
                .collect(),
            templates: config.templates.iter().map(template).collect(),
            payloads,
            origin: Origin::Config,
        })
    }

    /// The `[select]` table and `[[template]]` rows the target recorded
    /// for this source. A record written by a run that had no selection
    /// (every mirror run before this feature) carries none, and the bare
    /// run then means what it always did: everything.
    pub(super) fn from_record(recorded: &record::SelectRecord) -> Self {
        Selection {
            all: recorded.all,
            packages: split_all(&recorded.packages),
            groups: recorded.groups.clone(),
            exclude: recorded
                .exclude
                .iter()
                .map(String::as_str)
                .map(exclusion)
                .collect(),
            templates: recorded.templates.iter().map(recorded_template).collect(),
            // The budget and the drop are the operator's, not the
            // registry's: a payload projection is always asked for by the
            // configuration of the run that performs it.
            payloads: None,
            origin: Origin::Recorded,
        }
    }

    /// Whether this run copies `group/name` — the seam the source side
    /// reads, unchanged from `Filters`.
    pub(super) fn admits(&self, group: &str, name: &str) -> bool {
        if self.excludes(group, name) {
            return false;
        }
        if self.all {
            return true;
        }
        if self
            .templates
            .iter()
            .any(|row| row.group == group && row.name == name)
        {
            // A `[[template]]` row selects the template package itself: a
            // mirror copies the package and snapshots its inputs
            // (`#MIRROR-TEMPLATE-SNAPSHOT`).
            return true;
        }
        if self.packages.is_empty() && self.groups.is_empty() {
            // Neither ordinary axis names anything, so only the
            // `[[template]]` rows select — the two-axis rule below reads
            // an empty axis as unconstrained, which here would turn a
            // selection of template packages alone into everything. A run
            // with no selection at all never reaches this: it is `all`.
            return false;
        }
        let by_package =
            self.packages.is_empty() || self.packages.iter().any(|(g, n)| g == group && n == name);
        let by_group = self.groups.is_empty() || self.groups.iter().any(|g| g == group);
        by_package && by_group
    }

    /// The packages named one by one — what a per-package source without
    /// an index must be given, since it cannot be listed.
    pub(super) fn packages(&self) -> &[(String, String)] {
        &self.packages
    }

    pub(super) fn templates(&self) -> &[TemplateSelection] {
        &self.templates
    }

    pub(super) fn payloads(&self) -> Option<&PayloadPlan> {
        self.payloads.as_ref()
    }

    pub(super) fn origin(&self) -> Origin {
        self.origin
    }

    /// The record to write into `_index/mirrors/<source>.toml`, or `None`
    /// when this run has no selection of its own to record.
    ///
    /// **Only a declared selection is recorded.** A bare run repeating
    /// what the record already says must not rewrite it — the record is
    /// the selection, not a log of runs — and `--package` / `--group`
    /// are a per-run override, not a declaration: the flag predates this
    /// feature and has a second job, which is making a source that cannot
    /// be listed at all mirrorable one package at a time
    /// (`cli::RegistryMirrorArgs::packages`). Recording it would turn that
    /// workaround into a permanent narrowing of every later bare run, and
    /// would let one override destroy the operator's declaration. It also
    /// keeps the record byte-identical for every workflow that predates
    /// `--config`, which is what makes `schema = 1` honest.
    pub(super) fn to_record(&self) -> Option<record::SelectRecord> {
        if self.origin != Origin::Config {
            return None;
        }
        Some(record::SelectRecord {
            all: self.all,
            groups: self.groups.clone(),
            packages: self
                .packages
                .iter()
                .map(|(group, name)| format!("{group}/{name}"))
                .collect(),
            exclude: self
                .exclude
                .iter()
                .map(|entry| match entry {
                    Exclusion::Group(group) => group.clone(),
                    Exclusion::Package(group, name) => format!("{group}/{name}"),
                })
                .collect(),
            templates: self
                .templates
                .iter()
                .map(|row| record::TemplateRecord {
                    package: row.package(),
                    versions: row.versions.iter().map(ToString::to_string).collect(),
                    channels: row.channels.clone(),
                    newest: row.newest,
                    platforms: row.platforms.clone(),
                    archs: row.archs.clone(),
                    installers: row.installers.clone(),
                    payloads: row.payloads,
                })
                .collect(),
        })
    }

    fn excludes(&self, group: &str, name: &str) -> bool {
        self.exclude.iter().any(|entry| match entry {
            Exclusion::Group(excluded) => excluded == group,
            Exclusion::Package(excluded_group, excluded_name) => {
                excluded_group == group && excluded_name == name
            }
        })
    }
}

/// Where payload projection writes, and the budget over it.
///
/// `into` defaults to the first configured `[artifacts] packages` drop;
/// with no drop configured and no `into` the refusal names
/// `[artifacts] packages`, because payload projection is the one part of a
/// mirror run that writes outside the registry target and so must say
/// where.
fn resolve_payloads(
    config: &Validated,
    drops: &[PathBuf],
    target_root: Option<&std::path::Path>,
) -> Result<Option<PayloadPlan>> {
    let shown = config.path.display().to_string();
    let wanted = config.templates.iter().any(|row| row.payloads);
    if !wanted {
        return Ok(None);
    }
    let into = match &config.payloads.into {
        Some(into) => into.clone(),
        None => drops
            .first()
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "`{shown}` asks for payloads but names no `[payloads] into`, and no \
                     `[artifacts] packages` drop is configured \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-PROJECTION; \
                     fix: write `[payloads] into = \"<directory>\"`, or declare an \
                     `[artifacts] packages` drop in `~/.vibe/config.toml`)"
                )
            })?,
    };
    if let Some(root) = target_root {
        let under = |path: &std::path::Path, root: &std::path::Path| {
            let (path, root) = (
                path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
                root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
            );
            path.starts_with(&root) || root.starts_with(&path)
        };
        if under(&into, root) {
            bail!(
                "`[payloads] into` of `{shown}` is `{}`, which is the registry target `{}` \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-PROJECTION; \
                 fix: payloads go into a package drop, never into the registry — a monorepo must \
                 not grow forty gigabytes; name a directory outside the target)",
                into.display(),
                root.display()
            );
        }
    }
    Ok(Some(PayloadPlan {
        max_total: config.payloads.max_total,
        into,
    }))
}

fn template(row: &ValidatedTemplate) -> TemplateSelection {
    TemplateSelection {
        group: row.group.clone(),
        name: row.name.clone(),
        versions: row.versions.clone(),
        channels: row.channels.iter().map(ToString::to_string).collect(),
        newest: row.newest,
        platforms: row.platforms.clone(),
        archs: row.archs.clone(),
        installers: row.installers.clone(),
        payloads: row.payloads,
    }
}

/// A recorded row is read back as it was written. Its spellings were
/// validated by the run that wrote it, and a spelling this vibe cannot
/// parse is dropped rather than fatal: the record is derived state the
/// next `--config` run rewrites, and a mirror must not become unusable
/// because an older vibe wrote a requirement this one reads differently.
fn recorded_template(row: &record::TemplateRecord) -> TemplateSelection {
    let (group, name) = row
        .package
        .split_once('/')
        .map(|(group, name)| (group.to_string(), name.to_string()))
        .unwrap_or_else(|| (row.package.clone(), String::new()));
    TemplateSelection {
        group,
        name,
        versions: row
            .versions
            .iter()
            .filter_map(|spelling| semver::VersionReq::parse(spelling).ok())
            .collect(),
        channels: row.channels.clone(),
        newest: row.newest,
        platforms: row.platforms.clone(),
        archs: row.archs.clone(),
        installers: row.installers.clone(),
        payloads: row.payloads,
    }
}

fn split_all(spellings: &[String]) -> Vec<(String, String)> {
    spellings
        .iter()
        .filter_map(|spelling| spelling.split_once('/'))
        .map(|(group, name)| (group.to_string(), name.to_string()))
        .collect()
}

fn exclusion(entry: &str) -> Exclusion {
    match entry.split_once('/') {
        Some((group, name)) => Exclusion::Package(group.to_string(), name.to_string()),
        None => Exclusion::Group(entry.to_string()),
    }
}

/// Read `--config`, `--package` / `--group` and the target's recorded
/// selection into the one selection a run uses.
pub(super) fn resolve(
    config_path: Option<&std::path::Path>,
    packages: &[String],
    groups: &[String],
    recorded: Option<&record::SelectRecord>,
    drops: &[PathBuf],
    target_root: Option<&std::path::Path>,
) -> Result<Selection> {
    if !packages.is_empty() || !groups.is_empty() {
        // Flags override `[select]` wholesale (`#MIRROR-CONFIG`). The
        // configuration is still read, so a misspelt document is refused
        // rather than silently unused.
        if let Some(path) = config_path {
            config::read(path)?;
        }
        return Selection::from_flags(packages, groups);
    }
    if let Some(path) = config_path {
        return Selection::from_config(&config::read(path)?, drops, target_root);
    }
    match recorded {
        Some(recorded) => Ok(Selection::from_record(recorded)),
        None => Selection::from_flags(packages, groups),
    }
}

#[cfg(test)]
#[path = "selection/tests.rs"]
mod tests;
