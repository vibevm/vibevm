//! `--prune` — removing what the **recorded** selection no longer matches
//! (PROP-075 `#MIRROR-PRUNE`, `#MIRROR-NO-SILENT-PRUNE`).
//!
//! Narrowing a selection never deletes: versions are immutable and a
//! project may have locked one, so removal is explicit and always a
//! two-step. The plan is printed first — one row per removal with the
//! reason that reached it — and nothing is removed without a yes.
//!
//! A plan is read against **one** reading of the target and applied
//! against a possibly newer one, which is why an unattended run without
//! `--yes` is refused rather than allowed to copy and not prune: a plan
//! that described a tree the same run then changed would be a document
//! about nothing. `--dry-run --prune` prints the plan and stops.
//!
//! Everything inside the registry goes through the batch's removal
//! channel, so a pruned version's bytes, its index rows and its journal
//! fact leave in ONE commit. The drop is the one thing outside it, and it
//! is emptied **after** the commit: a registry that stopped advertising
//! bytes it still has is safe, and the other order is not.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PRUNE");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_publish::layout_publish::{Removal, RemovedVersion};
use vibe_wire::generated::registry_mirror_report as wire;

use super::record::MirrorRecord;
use super::selection::{Origin, PayloadPlan, Selection};
use super::templates::StaleSnapshot;
use crate::output;

const SPEC: &str = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PRUNE";

/// What `--prune`, `--prune-payloads` and `--yes` asked for.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Request {
    pub prune: bool,
    pub payloads: bool,
    pub yes: bool,
}

/// What one planned removal is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum What {
    /// A package version directory in the target.
    Version,
    /// A template snapshot, whole or by version, under `_index/`.
    Snapshot,
    /// A projected payload in the drop, outside the registry.
    Payload,
}

impl What {
    pub(super) fn label(self) -> &'static str {
        match self {
            What::Version => "version",
            What::Snapshot => "snapshot",
            What::Payload => "payload",
        }
    }
}

/// One row of the plan: what would go, from where, and why.
#[derive(Debug, Clone)]
pub(super) struct Row {
    pub what: What,
    /// Registry-relative for a version or a snapshot; the absolute drop
    /// path for a payload. Either way the path **spells the coordinate of
    /// its fact**, which is what makes a plan readable and a removal
    /// checkable.
    pub path: String,
    pub package: String,
    pub version: Option<String>,
    pub reason: String,
}

/// The whole plan: its rows for a person to read, the batch channel for
/// what is inside the registry, and the drop paths for what is not.
#[derive(Debug, Clone, Default)]
pub(super) struct Plan {
    pub rows: Vec<Row>,
    pub removed: Vec<Removal>,
    pub payloads: Vec<PathBuf>,
}

impl Plan {
    pub(super) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Refuse the flag combinations that cannot mean what they say, before
/// anything is listed.
pub(super) fn check(request: Request, selection: &Selection, config: Option<&Path>) -> Result<()> {
    if request.payloads && !request.prune {
        bail!(
            "`--prune-payloads` removes the payloads of the versions `--prune` drops, and \
             `--prune` was not given \
             (violates {SPEC}; fix: pass both, or neither — nothing is ever removed without \
             `--prune`)"
        );
    }
    if !request.prune {
        return Ok(());
    }
    if selection.origin() == Origin::Flags {
        // `--package` / `--group` are a per-run override, not a
        // declaration (`#P4-AS-BUILT` 1): pruning by one would delete
        // what the operator's own declaration still asks for.
        bail!(
            "`--prune` removes what the recorded selection no longer matches, and this run has \
             no declared selection{} \
             (violates {SPEC}; fix: run with `--config mirror.toml`, or bare against a target \
             whose `_index/mirrors/<source>.toml` records a `[select]` — `--package` / `--group` \
             are a per-run override and are never recorded)",
            match config {
                Some(_) => ", because `--package` / `--group` overrode `--config` wholesale",
                None => "",
            }
        );
    }
    if request.payloads && selection.payloads().is_none() {
        bail!(
            "`--prune-payloads` needs the drop the payloads were projected into, and no \
             `[[template]]` row of this selection asks for payloads \
             (violates spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-PROJECTION; \
             fix: run with the `--config` that projected them — its `payloads = true` row and \
             `[payloads] into` are what name the drop)"
        );
    }
    Ok(())
}

/// Plan the removals against one reading of the target.
pub(super) fn plan(
    request: Request,
    selection: &Selection,
    source: &str,
    target_root: &Path,
    target_index: &vibe_index::index::Index,
    stale: &[StaleSnapshot],
) -> Result<Plan> {
    if !request.prune {
        return Ok(Plan::default());
    }
    let mut plan = Plan::default();
    let protected = copied_elsewhere(target_root, source)?;
    for entry in target_index.iter_versions() {
        let package = format!("{}/{}", entry.group, entry.name);
        if selection.admits(entry.group.as_str(), &entry.name) {
            continue;
        }
        if protected.contains(&(package.clone(), entry.version.to_string())) {
            // Another source's record says it copied this version, so
            // this source's run is not the one that may remove it.
            continue;
        }
        let path = format!("{}/{}/v{}", entry.group, entry.name, entry.version);
        plan.rows.push(Row {
            what: What::Version,
            path: path.clone(),
            package: package.clone(),
            version: Some(entry.version.to_string()),
            reason: "no longer selected by `[select]`".to_owned(),
        });
        plan.removed.push(Removal {
            path,
            version: Some(RemovedVersion {
                group: entry.group.clone(),
                name: entry.name.clone(),
                version: entry.version.clone(),
            }),
        });
    }
    for snapshot in stale {
        for version in &snapshot.versions {
            let path = format!(
                "{}/digests/{version}",
                vibe_registry::templates::snapshot::relative_dir(&snapshot.group, &snapshot.name)
            );
            plan.rows.push(Row {
                what: What::Snapshot,
                path: path.clone(),
                package: snapshot.package.clone(),
                version: Some(version.clone()),
                reason: format!("no longer selected by `[[template]]` {}", snapshot.package),
            });
            plan.removed.push(Removal {
                path,
                version: None,
            });
        }
    }
    for (package, group, name) in orphan_snapshots(target_root, selection)? {
        let path = vibe_registry::templates::snapshot::relative_dir(&group, &name);
        plan.rows.push(Row {
            what: What::Snapshot,
            path: path.clone(),
            package: package.clone(),
            version: None,
            reason: "the selection carries no `[[template]]` row for it".to_owned(),
        });
        plan.removed.push(Removal {
            path,
            version: None,
        });
    }
    if let (true, Some(payloads)) = (request.payloads, selection.payloads()) {
        plan_payloads(&mut plan, payloads, stale, selection, target_root)?;
    }
    Ok(plan)
}

/// The drop directories of the produced versions the snapshot side drops,
/// and the whole package directory of a template the selection dropped.
fn plan_payloads(
    plan: &mut Plan,
    payloads: &PayloadPlan,
    stale: &[StaleSnapshot],
    selection: &Selection,
    target_root: &Path,
) -> Result<()> {
    let push = |plan: &mut Plan, path: PathBuf, package: String, version: Option<String>| {
        if !path.exists() {
            return;
        }
        plan.rows.push(Row {
            what: What::Payload,
            path: vibe_core::remap::drop::shown_path(&path),
            package,
            version: version.clone(),
            reason: match &version {
                Some(_) => "its version is no longer selected".to_owned(),
                None => "its template package is no longer selected".to_owned(),
            },
        });
        plan.payloads.push(path);
    };
    for snapshot in stale {
        let dir = payloads
            .into
            .join(snapshot.group.as_str())
            .join(&snapshot.name);
        for version in &snapshot.versions {
            push(
                plan,
                dir.join(version),
                snapshot.package.clone(),
                Some(version.clone()),
            );
        }
    }
    for (package, group, name) in orphan_snapshots(target_root, selection)? {
        push(
            plan,
            payloads.into.join(group.as_str()).join(&name),
            package,
            None,
        );
    }
    Ok(())
}

/// The snapshots the target carries for packages this selection names no
/// `[[template]]` row for.
fn orphan_snapshots(
    target_root: &Path,
    selection: &Selection,
) -> Result<Vec<(String, vibe_core::Group, String)>> {
    let root = target_root.join(format!(
        "{}/{}",
        vibe_core::layout_registry::INDEX_DIR,
        vibe_registry::templates::snapshot::SNAPSHOT_SUBDIR
    ));
    let mut found = Vec::new();
    let Ok(groups) = std::fs::read_dir(&root) else {
        return Ok(found);
    };
    for group in groups.flatten().filter(|e| e.path().is_dir()) {
        let Some(spelling) = group.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Ok(parsed) = vibe_core::Group::parse(&spelling) else {
            continue;
        };
        let names = std::fs::read_dir(group.path())
            .with_context(|| format!("reading `{}`", group.path().display()))?;
        for name in names.flatten().filter(|e| e.path().is_dir()) {
            let Some(name) = name.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let package = format!("{spelling}/{name}");
            if selection
                .templates()
                .iter()
                .any(|row| row.package() == package)
            {
                continue;
            }
            found.push((package, parsed.clone(), name));
        }
    }
    Ok(found)
}

/// `(package, version)` of every version a **different** source's record
/// says it copied into this target.
///
/// Such a version is never pruned by this source's run: the two sources
/// are two declarations, and one narrowing is not the other's word.
fn copied_elsewhere(target_root: &Path, source: &str) -> Result<BTreeSet<(String, String)>> {
    let dir = target_root.join(format!("{}/mirrors", vibe_core::layout_registry::INDEX_DIR));
    let mut protected = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(protected);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        if path.file_stem().and_then(|s| s.to_str()) == Some(source) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        // A record this vibe cannot read protects nothing it does not
        // name, which is the permissive side of a destructive act: an
        // unreadable record is reported, never a reason to delete.
        let Ok(record) = toml::from_str::<MirrorRecord>(&text) else {
            continue;
        };
        for row in record.versions {
            protected.insert((row.package, row.version));
        }
    }
    Ok(protected)
}

/// Print the plan — one row per removal with its reason — and answer its
/// question.
///
/// Returns whether to remove. A dry run never removes and never asks: the
/// plan is the whole answer it owes.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PRUNE")]
pub(super) fn answer(
    ctx: &output::Context,
    plan: &Plan,
    request: Request,
    dry_run: bool,
) -> Result<bool> {
    if plan.is_empty() {
        if request.prune && !ctx.is_json() {
            ctx.step("--prune: the recorded selection still matches everything in the target");
        }
        return Ok(false);
    }
    show(ctx, plan);
    if dry_run {
        return Ok(false);
    }
    if request.yes {
        return Ok(true);
    }
    let count = plan.rows.len();
    let plural = if count == 1 { "" } else { "s" };
    if !attended(ctx) {
        bail!(
            "prune_consent_required: the plan above removes {count} thing{plural} from the \
             target, and this run has nobody to ask; nothing was written \
             (violates {SPEC}; fix: read the plan, then re-run the same command with `--yes`; \
             `--dry-run --prune` prints it without writing anything either way)"
        );
    }
    crate::output::prompts::confirm_then(
        &format!("remove the {count} thing{plural} above?"),
        false,
        "prune",
        |accepted| {
            let verb = if accepted { "removing" } else { "keeping" };
            format!("{verb} {count} thing{plural}")
        },
    )
    .context("reading the prune answer")
}

/// `AV-FMT-009`'s `pruned[]`: the plan, for a client that reads the
/// document rather than the printed rows.
pub(super) fn rows(plan: &Plan) -> Vec<wire::PrunedItem> {
    plan.rows
        .iter()
        .map(|row| wire::PrunedItem {
            kind: match row.what {
                What::Version => wire::PrunedKind::Version,
                What::Snapshot => wire::PrunedKind::Snapshot,
                What::Payload => wire::PrunedKind::Payload,
            },
            package: row.package.clone(),
            path: row.path.clone(),
            version: row.version.clone(),
            reason: row.reason.clone(),
        })
        .collect()
}

/// Whether this run may ask: a terminal, not `--json`, not `--unattended`.
fn attended(ctx: &output::Context) -> bool {
    !ctx.is_json() && !ctx.is_unattended() && console::user_attended()
}

fn show(ctx: &output::Context, plan: &Plan) {
    if ctx.is_json() {
        return;
    }
    ctx.step(&format!("--prune: {} thing(s) to remove", plan.rows.len()));
    for row in &plan.rows {
        let version = row
            .version
            .as_deref()
            .map(|version| format!("@{version}"))
            .unwrap_or_default();
        ctx.step(&format!(
            "  {} {}{version} — {} ({})",
            row.what.label(),
            row.package,
            row.reason,
            row.path
        ));
    }
}

/// Remove the projected payloads the plan names, after the registry's own
/// commit has stopped advertising them.
pub(super) fn remove_payloads(ctx: &output::Context, plan: &Plan) -> Result<usize> {
    let mut removed = 0usize;
    for path in &plan.payloads {
        let outcome = match path.is_dir() {
            true => std::fs::remove_dir_all(path),
            false => std::fs::remove_file(path),
        };
        match outcome {
            Ok(()) => removed += 1,
            // Someone else removed it between the plan and here, which is
            // the outcome asked for.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("removing `{}`", path.display()));
            }
        }
    }
    if removed > 0 && !ctx.is_json() {
        ctx.step(&format!(
            "--prune-payloads: {removed} payload director(ies) removed from the drop"
        ));
    }
    Ok(removed)
}

#[cfg(test)]
#[path = "prune/tests.rs"]
mod tests;
