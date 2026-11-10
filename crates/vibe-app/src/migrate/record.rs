//! The shared work directory and the `vibe-app-migration/1` record inside
//! it (PROP-075 `#MIGRATION-FAILURE`).
//!
//! The record and the command's machine answer are the **same** document:
//! the kept work directory's `migration.json` is the report of the last run
//! over it, which is exactly what `--resume` needs in order to re-run only
//! the import when the export already succeeded. There is no second
//! mechanism and no second format.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE");

use std::fs;
use std::path::{Path, PathBuf};

use vibe_wire::generated::application_migration as wire;

use crate::error::{AppError, StepError};
use crate::index::{MigrationEntryRecord, MigrationOutcomeRecord, MigrationResult};
use crate::installer::io;
use crate::settings::SettingsRoot;

use super::MigrationPlan;

/// The in-document protocol tag of the migration record and report.
pub const MIGRATION_PROTOCOL: &str = "vibe-app-migration/1";

/// The file the record lives in, inside the work directory.
const RECORD_FILE: &str = "migration.json";

/// Open the shared work directory of one pair:
/// `<home>/cache/app-migrate/<slot>/<from>-to-<to>/<timestamp>/`.
///
/// A fresh run always gets its own timestamped directory, because the
/// previous one is evidence and is never replaced. `--resume` instead
/// reuses the newest kept directory of the same pair, and refuses when
/// there is none — the same contract `PROP-065#resume` gives an
/// installation.
pub(super) fn open(
    settings: &SettingsRoot,
    plan: &MigrationPlan,
    resume: bool,
) -> Result<(PathBuf, bool), AppError> {
    let pair = pair_dir(settings, plan);
    if resume {
        let newest = runs(&pair)
            .pop()
            .ok_or_else(|| StepError::NothingToResume {
                key: format!("{}@{}", plan.slot, plan.to_version),
                work: pair.display().to_string(),
                reason: "no migration work directory of this version pair is kept".to_owned(),
            })?;
        return Ok((newest, true));
    }
    let work = fresh(&pair, &stamp());
    fs::create_dir_all(&work).map_err(|source| io("creating", &work, source))?;
    Ok((work, false))
}

/// A run directory that does not exist yet. Two runs of one pair inside the
/// same second must not merge their evidence, so the second takes `-2`.
fn fresh(pair: &Path, stamp: &str) -> PathBuf {
    let first = pair.join(stamp);
    if !first.exists() {
        return first;
    }
    (2u32..)
        .map(|n| pair.join(format!("{stamp}-{n}")))
        .find(|path| !path.exists())
        .unwrap_or(first)
}

/// `<home>/cache/app-migrate/<slot>/<from>-to-<to>`.
fn pair_dir(settings: &SettingsRoot, plan: &MigrationPlan) -> PathBuf {
    settings
        .app_migrate_dir()
        .join(sanitised(&plan.slot))
        .join(format!(
            "{}-to-{}",
            sanitised(&plan.from_version),
            sanitised(&plan.to_version)
        ))
}

/// A slot or version as one path component: everything that is not a
/// portable file-name character becomes `_`, so no declaration can reach
/// out of the migration root.
fn sanitised(value: &str) -> String {
    let mut text: String = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if text.is_empty() || text.chars().all(|c| c == '.') {
        text = "_".to_owned();
    }
    text
}

/// The run directories of one pair, oldest first.
fn runs(pair: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(pair)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

/// Every kept migration work directory of the home, oldest first. This is
/// what `vibe app work` lists and cleans beside the installation work
/// directories (PROP-075 `#MIGRATION-FAILURE`).
pub fn kept_runs(settings: &SettingsRoot) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for slot in directories(&settings.app_migrate_dir()) {
        for pair in directories(&slot) {
            found.extend(runs(&pair));
        }
    }
    found
}

/// The migration work directories of one slot, oldest first.
pub fn work_runs(settings: &SettingsRoot, slot: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for pair in directories(&settings.app_migrate_dir().join(sanitised(slot))) {
        found.extend(runs(&pair));
    }
    found
}

fn directories(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect()
        })
        .unwrap_or_default();
    paths.sort();
    paths
}

/// How one kept migration work directory shows up in `vibe app work`
/// (PROP-075 `#MIGRATION-FAILURE`): keyed by slot and version pair, because
/// a migration belongs to a pair and not to one package.
pub(crate) struct Kept {
    pub key: String,
    pub failed: Option<String>,
    pub pending: bool,
}

/// What the kept record of `work` says about itself; `None` when it holds
/// no readable record.
pub(crate) fn summary(work: &Path) -> Option<Kept> {
    let document = read(work)?;
    let failing = document.entries.iter().find(|entry| {
        matches!(
            entry.outcome,
            wire::Outcome::ExportFailed | wire::Outcome::ImportFailed
        )
    });
    Some(Kept {
        key: format!(
            "migrate:{}:{}-to-{}",
            document.slot,
            version_of(&document.from),
            version_of(&document.to)
        ),
        failed: failing.and_then(|entry| entry.step.clone()),
        pending: document
            .entries
            .iter()
            .any(|entry| matches!(entry.outcome, wire::Outcome::Pending)),
    })
}

/// The version half of an installation key.
fn version_of(key: &str) -> &str {
    key.rsplit_once('@').map_or(key, |(_, version)| version)
}

/// `20261110T101500Z`, the run directory's own name.
fn stamp() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

/// An empty report over `plan`, before any entry has run.
pub(super) fn empty(
    plan: &MigrationPlan,
    work: &Path,
    resumed: bool,
) -> wire::ApplicationMigration {
    wire::ApplicationMigration {
        protocol: MIGRATION_PROTOCOL.to_owned(),
        ok: true,
        slot: plan.slot.clone(),
        from: plan.from.clone(),
        to: plan.to.clone(),
        work: work.display().to_string(),
        resumed,
        entries: Vec::new(),
        notices: Vec::new(),
    }
}

/// The record of a declared entry that this run did not select.
pub(super) fn skipped(entry: &MigrationEntryRecord, reason: &str) -> wire::Entry {
    wire::Entry {
        id: entry.id.clone(),
        name: entry.name.clone(),
        outcome: wire::Outcome::Skipped,
        description: entry.description.clone(),
        side: None,
        step: None,
        log: None,
        reason: Some(reason.to_owned()),
        retry: None,
        started: None,
        finished: None,
    }
}

/// The kept record of the work directory, if it holds a readable one. A
/// corrupt record is no record: the run then re-runs both sides, which is
/// always safe because a migration is the package's own idempotent script.
pub(super) fn read(work: &Path) -> Option<wire::ApplicationMigration> {
    let bytes = fs::read(work.join(RECORD_FILE)).ok()?;
    let document: wire::ApplicationMigration = serde_json::from_slice(&bytes).ok()?;
    (document.protocol == MIGRATION_PROTOCOL).then_some(document)
}

/// One entry of a kept record.
pub(super) fn entry_of<'a>(
    previous: Option<&'a wire::ApplicationMigration>,
    id: &str,
) -> Option<&'a wire::Entry> {
    previous?.entries.iter().find(|entry| entry.id == id)
}

/// Rewrite the record after every side, so an interrupted run leaves a
/// true one behind.
pub(super) fn save(work: &Path, document: &wire::ApplicationMigration) -> Result<(), AppError> {
    let path = work.join(RECORD_FILE);
    let mut bytes = serde_json::to_vec_pretty(document).map_err(|error| {
        AppError::from(StepError::MigrationRecord {
            work: path.display().to_string(),
            reason: error.to_string(),
        })
    })?;
    bytes.push(b'\n');
    crate::settings::write_atomic(&path, &bytes)?;
    Ok(())
}

/// What the run tells the new installation to retain: one outcome per entry
/// that actually ran (PROP-075 `#MIGRATION-FAILURE`).
pub(super) fn outcomes(
    plan: &MigrationPlan,
    document: &wire::ApplicationMigration,
) -> Vec<MigrationOutcomeRecord> {
    document
        .entries
        .iter()
        .filter_map(|entry| {
            let result = match entry.outcome {
                wire::Outcome::Done => MigrationResult::Done,
                wire::Outcome::ExportFailed => MigrationResult::ExportFailed,
                wire::Outcome::ImportFailed => MigrationResult::ImportFailed,
                wire::Outcome::Pending => MigrationResult::Pending,
                wire::Outcome::Skipped => return None,
            };
            Some(MigrationOutcomeRecord {
                id: entry.id.clone(),
                from: plan.from.clone(),
                to: plan.to.clone(),
                result,
                step: entry.step.clone(),
                work: Some(document.work.clone()),
                log: entry.log.clone(),
                at: entry.finished.clone(),
            })
        })
        .collect()
}
