//! The run's own work directory and the `vibe-app-ecosystem/1` record
//! inside it (PROP-075 `#ECOSYSTEM-ADAPTER`).
//!
//! The record and the command's machine answer are the **same** document,
//! for the reason a migration's are: the report *is* the record, so there
//! is one format and no second mechanism. The directory is kept on success
//! and on failure, because it holds the verb's log and the answer file a
//! `list` verb wrote.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER");

use std::fs;
use std::path::{Path, PathBuf};

use vibe_wire::generated::application_ecosystem as wire;

use crate::error::{AppError, StepError};
use crate::installer::io;
use crate::settings::SettingsRoot;

use super::{EcosystemRequest, wire_verb};

/// The in-document protocol tag of the adapter run record and report.
pub const ECOSYSTEM_PROTOCOL: &str = "vibe-app-ecosystem/1";

/// The file the record lives in, inside the work directory.
const RECORD_FILE: &str = "ecosystem.json";

/// Open a fresh work directory:
/// `<home>/cache/app-ecosystem/<slot>/<version>/<timestamp>/`.
///
/// A run always gets its own timestamped directory, because the previous
/// one is the evidence of what the last run did and is never replaced.
pub(super) fn open(
    settings: &SettingsRoot,
    slot: &str,
    version: &str,
) -> Result<PathBuf, AppError> {
    let parent = version_dir(settings, slot, version);
    let work = fresh(&parent, &stamp());
    fs::create_dir_all(&work).map_err(|source| io("creating", &work, source))?;
    Ok(work)
}

/// A run directory that does not exist yet. Two runs inside the same
/// second must not merge their evidence, so the second takes `-2`.
fn fresh(parent: &Path, stamp: &str) -> PathBuf {
    let first = parent.join(stamp);
    if !first.exists() {
        return first;
    }
    (2u32..)
        .map(|n| parent.join(format!("{stamp}-{n}")))
        .find(|path| !path.exists())
        .unwrap_or(first)
}

/// `<home>/cache/app-ecosystem/<slot>/<version>`.
fn version_dir(settings: &SettingsRoot, slot: &str, version: &str) -> PathBuf {
    settings
        .app_ecosystem_dir()
        .join(sanitised(slot))
        .join(sanitised(version))
}

/// A slot or version as one path component: everything that is not a
/// portable file-name character becomes `_`, so no declaration can reach
/// out of the ecosystem root.
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

/// The run directories of one version, oldest first.
fn runs(parent: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(parent)
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

/// Every kept adapter work directory of the home, oldest first. This is
/// what `vibe app work` lists and cleans beside the installation and
/// migration work directories.
pub fn kept_runs(settings: &SettingsRoot) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for slot in directories(&settings.app_ecosystem_dir()) {
        for version in directories(&slot) {
            found.extend(runs(&version));
        }
    }
    found
}

/// The adapter work directories of one slot, oldest first.
pub fn work_runs(settings: &SettingsRoot, slot: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for version in directories(&settings.app_ecosystem_dir().join(sanitised(slot))) {
        found.extend(runs(&version));
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

/// How one kept adapter work directory shows up in `vibe app work`: keyed
/// by the slot, the version and the verb, because an adapter run belongs
/// to a version and a verb and not to one package.
pub(crate) struct Kept {
    pub key: String,
    pub failed: Option<String>,
}

/// What the kept record of `work` says about itself; `None` when it holds
/// no readable record.
pub(crate) fn summary(work: &Path) -> Option<Kept> {
    let document = read(work)?;
    let failing = document
        .adapters
        .iter()
        .find(|row| row.outcome == wire::Outcome::Failed);
    Some(Kept {
        key: format!(
            "ecosystem:{}:{}:{}",
            document.slot,
            version_of(&document.key),
            verb_name(document.verb)
        ),
        failed: failing.and_then(|row| row.step.clone()),
    })
}

/// The version half of an installation key.
fn version_of(key: &str) -> &str {
    key.rsplit_once('@').map_or(key, |(_, version)| version)
}

/// The stable spelling of a wire verb.
fn verb_name(verb: wire::Verb) -> &'static str {
    match verb {
        wire::Verb::Install => "install",
        wire::Verb::List => "list",
        wire::Verb::Remove => "remove",
        wire::Verb::SelfUpgrade => "self_upgrade",
    }
}

/// `20261110T101500Z`, the run directory's own name.
fn stamp() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

/// An empty report over `request`, before any adapter has run.
pub(super) fn empty(
    request: &EcosystemRequest<'_>,
    key: &str,
    work: &Path,
) -> wire::ApplicationEcosystem {
    wire::ApplicationEcosystem {
        protocol: ECOSYSTEM_PROTOCOL.to_owned(),
        ok: true,
        slot: request.slot.to_owned(),
        key: key.to_owned(),
        verb: wire_verb(request.verb),
        work: work.display().to_string(),
        adapters: Vec::new(),
        notices: Vec::new(),
    }
}

/// The kept record of the work directory, if it holds a readable one.
pub(super) fn read(work: &Path) -> Option<wire::ApplicationEcosystem> {
    let bytes = fs::read(work.join(RECORD_FILE)).ok()?;
    let document: wire::ApplicationEcosystem = serde_json::from_slice(&bytes).ok()?;
    (document.protocol == ECOSYSTEM_PROTOCOL).then_some(document)
}

/// Rewrite the record after every adapter, so an interrupted run leaves a
/// true one behind.
pub(super) fn save(work: &Path, document: &wire::ApplicationEcosystem) -> Result<(), AppError> {
    let path = work.join(RECORD_FILE);
    let mut bytes = serde_json::to_vec_pretty(document).map_err(|error| {
        AppError::from(StepError::EcosystemRecord {
            work: path.display().to_string(),
            reason: error.to_string(),
        })
    })?;
    bytes.push(b'\n');
    crate::settings::write_atomic(&path, &bytes)?;
    Ok(())
}
