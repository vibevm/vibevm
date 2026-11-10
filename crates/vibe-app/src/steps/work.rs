//! Kept work directories (`PROP-065#work`): `--keep-work` leaves the work
//! directory of a successful run in place, and this module is what shows
//! the kept directories of a home with their state and size and what
//! deletes them again. Only `<home>/cache/app-work/` is ever read or
//! removed here — the download cache and the run records are other homes.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#work");

use std::fs;
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_wire::generated::application_work as wire;

use crate::error::{AppError, StepError};
use crate::index::StepOutcome;
use crate::installer::io;
use crate::resolve::qualified_ref;
use crate::settings::SettingsRoot;

use super::state::StepsFile;

mod cohorts;
pub use cohorts::{clean_import, work_import};

const WORK_PROTOCOL: &str = "vibe-app-work/1";

/// A byte count as a terminal shows it: `912 B`, `1.4 MiB`.
///
/// ```
/// assert_eq!(vibe_app::human_bytes(912), "912 B");
/// assert_eq!(vibe_app::human_bytes(1536), "1.5 KiB");
/// ```
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// The bytes a file holds, or a directory tree below `path`. A symbolic
/// link counts as the link, so no tree is walked twice.
pub(crate) fn tree_bytes(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if !meta.is_dir() {
        return meta.len();
    }
    fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| tree_bytes(&entry.path()))
                .sum()
        })
        .unwrap_or_default()
}

/// The kept work directories of the home, by package key
/// (`vibe app work`). `package` narrows the listing to one package and
/// makes an empty answer the refusal `work_missing`.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let listing = vibe_app::work(&settings, None).unwrap();
/// assert_eq!(listing.protocol, "vibe-app-work/1");
/// assert!(listing.entries.is_empty());
/// assert_eq!(
///     vibe_app::work(&settings, Some("org.example/tool")).unwrap_err().code(),
///     "work_missing"
/// );
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#work")]
pub fn work(
    settings: &SettingsRoot,
    package: Option<&str>,
) -> Result<wire::ApplicationWork, AppError> {
    listing(settings, package, true)
}

fn listing(
    settings: &SettingsRoot,
    package: Option<&str>,
    inspect_retention: bool,
) -> Result<wire::ApplicationWork, AppError> {
    let home = settings.app_work_dir();
    let mut entries = kept(settings, &home, package, inspect_retention)?;
    // A migration's work directory is kept like any other and is listed and
    // cleaned here (`PROP-075#MIGRATION-FAILURE`). It is keyed by a slot and
    // a version pair rather than by a package, so narrowing the listing to
    // one package never selects one.
    if package.is_none() {
        entries.extend(migrations(settings));
        entries.extend(ecosystems(settings));
    }
    if entries.is_empty()
        && let Some(spelling) = package
    {
        return Err(StepError::WorkMissing {
            package: spelling.into(),
            home: home.display().to_string(),
        }
        .into());
    }
    Ok(wire::ApplicationWork {
        protocol: WORK_PROTOCOL.into(),
        home: home.display().to_string(),
        entries,
        imports: if package.is_none() {
            cohorts::imports(settings, None)?
        } else {
            vec![]
        },
        removed: Vec::new(),
        freed: 0,
    })
}

/// Delete the kept work directories `package` selects, or every one of the
/// home when it is `None` (`vibe app work --clean [--all]`). The home's
/// mutation lock is held, so a clean during another vibe command is
/// refused rather than racing it.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let cleaned = vibe_app::clean_work(&settings, None).unwrap();
/// assert!(cleaned.removed.is_empty() && cleaned.freed == 0);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#work")]
pub fn clean_work(
    settings: &SettingsRoot,
    package: Option<&str>,
) -> Result<wire::ApplicationWork, AppError> {
    let held = settings.lock()?;
    let lock = &held;
    let mut listing = listing(settings, package, false)?;
    cohorts::cleanup(lock, settings, &mut listing, package.is_none())?;
    Ok(listing)
}

/// The kept migration work directories of the home as listing rows
/// (`PROP-075#MIGRATION-FAILURE`).
fn migrations(settings: &SettingsRoot) -> Vec<wire::Entry> {
    crate::migrate::kept_migration_work(settings)
        .into_iter()
        .map(|dir| {
            let summary = crate::migrate::migration_summary(&dir);
            let (state, step) = match &summary {
                Some(kept) if kept.failed.is_some() => {
                    (wire::EntryState::Failed, kept.failed.clone())
                }
                Some(kept) if kept.pending => (wire::EntryState::Interrupted, None),
                Some(_) => (wire::EntryState::Done, None),
                None => (wire::EntryState::Unknown, None),
            };
            let parts = parts(&dir);
            wire::Entry {
                operation: None,
                retained_for: None,
                key: summary
                    .map(|kept| kept.key)
                    .unwrap_or_else(|| format!("migrate:{}", file_name(&dir))),
                path: dir.display().to_string(),
                state,
                step,
                changed: changed(&dir),
                bytes: parts.sources + parts.out + parts.inputs + parts.other,
                parts,
            }
        })
        .collect()
}

/// The kept ecosystem-adapter work directories of the home as listing
/// rows (`PROP-075#ECOSYSTEM-ADAPTER`). Like a migration's, they are keyed
/// by a slot and not by a package, so narrowing the listing to one package
/// never selects one.
fn ecosystems(settings: &SettingsRoot) -> Vec<wire::Entry> {
    crate::ecosystem::kept_ecosystem_work(settings)
        .into_iter()
        .map(|dir| {
            let summary = crate::ecosystem::ecosystem_summary(&dir);
            let (state, step) = match &summary {
                Some(kept) if kept.failed.is_some() => {
                    (wire::EntryState::Failed, kept.failed.clone())
                }
                Some(_) => (wire::EntryState::Done, None),
                None => (wire::EntryState::Unknown, None),
            };
            let parts = parts(&dir);
            wire::Entry {
                operation: None,
                retained_for: None,
                key: summary
                    .map(|kept| kept.key)
                    .unwrap_or_else(|| format!("ecosystem:{}", file_name(&dir))),
                path: dir.display().to_string(),
                state,
                step,
                changed: changed(&dir),
                bytes: parts.sources + parts.out + parts.inputs + parts.other,
                parts,
            }
        })
        .collect()
}

/// Read every work directory of `home` that `package` matches.
fn kept(
    settings: &SettingsRoot,
    home: &Path,
    package: Option<&str>,
    inspect_retention: bool,
) -> Result<Vec<wire::Entry>, AppError> {
    let wanted = package.map(qualified_ref).transpose()?;
    let mut entries = Vec::new();
    for dir in directories(home) {
        let name = file_name(&dir);
        if name == "imports" {
            continue;
        }
        if let Some(wanted) = &wanted {
            let group = wanted
                .group
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            let prefix = format!("{group}.{}@", wanted.name.as_str());
            let matches = name.strip_prefix(&prefix).is_some_and(|version| {
                semver::Version::parse(version)
                    .is_ok_and(|version| wanted.version.matches(&version))
            });
            if !matches {
                continue;
            }
        }
        entries.push(entry(settings, &dir, &name, inspect_retention)?);
    }
    entries.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(entries)
}

fn entry(
    settings: &SettingsRoot,
    dir: &Path,
    name: &str,
    inspect_retention: bool,
) -> Result<wire::Entry, AppError> {
    let state = StepsFile::load(dir).ok().flatten();
    let parts = parts(dir);
    let (outcome, step) = reached(state.as_ref());
    let retained_for = if inspect_retention {
        match &state {Some(state) if state.operation.as_ref().is_some_and(|o|o.phase==vibe_wire::generated::shared::ApplicationPendingOperationPhase::Complete)=>crate::operations::pending::retained_for(settings,state)?,_=>None}
    } else {
        None
    };
    let operation = state
        .as_ref()
        .and_then(|s| s.operation.as_ref())
        .map(|op| wire::Operation {
            target: op.target.clone(),
            phase: op.phase.clone(),
        });
    Ok(wire::Entry {
        operation,
        retained_for: retained_for.map(|owner| wire::Retention { owner }),
        key: state
            .map(|state| state.package.key())
            .or_else(|| key_of(name))
            .unwrap_or_else(|| name.to_owned()),
        path: dir.display().to_string(),
        state: outcome,
        step,
        changed: changed(dir),
        bytes: parts.sources + parts.out + parts.inputs + parts.other,
        parts,
    })
}

/// How far the kept run got: the first step that is neither done nor
/// skipped, otherwise a directory kept after success.
fn reached(state: Option<&StepsFile>) -> (wire::EntryState, Option<String>) {
    let Some(state) = state else {
        return (wire::EntryState::Unknown, None);
    };
    let open = state
        .steps
        .iter()
        .find(|step| !matches!(step.outcome, StepOutcome::Done | StepOutcome::Skipped));
    match open {
        None if state.operation.as_ref().is_some_and(|op| {
            op.phase != vibe_wire::generated::shared::ApplicationPendingOperationPhase::Complete
                || op.publication_record.is_none()
        }) =>
        {
            (wire::EntryState::Unknown, None)
        }
        None => (wire::EntryState::Done, None),
        Some(step) => match step.outcome {
            StepOutcome::Failed => (wire::EntryState::Failed, Some(step.id.clone())),
            StepOutcome::Interrupted => (wire::EntryState::Interrupted, Some(step.id.clone())),
            _ => (wire::EntryState::Delegated, None),
        },
    }
}

/// The bytes of `sources/`, `out/`, `inputs/` and everything else.
fn parts(dir: &Path) -> wire::Parts {
    let mut parts = wire::Parts {
        sources: 0,
        out: 0,
        inputs: 0,
        other: 0,
    };
    for path in fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
    {
        let bytes = tree_bytes(&path);
        match file_name(&path).as_str() {
            "sources" => parts.sources += bytes,
            "out" => parts.out += bytes,
            "inputs" => parts.inputs += bytes,
            _ => parts.other += bytes,
        }
    }
    parts
}

/// When the kept run last wrote `steps.json`, or the directory's own mtime
/// when it has none.
fn changed(dir: &Path) -> String {
    fs::metadata(dir.join("steps.json"))
        .or_else(|_| fs::metadata(dir))
        .and_then(|meta| meta.modified())
        .ok()
        .map(|time| {
            chrono::DateTime::<chrono::Utc>::from(time)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        })
        .unwrap_or_default()
}

/// The installation key a work directory name `group.name@version` spells.
fn key_of(name: &str) -> Option<String> {
    let (coordinate, version) = name.rsplit_once('@')?;
    let (group, package) = coordinate.rsplit_once('.')?;
    Some(format!("{group}/{package}@{version}"))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{AuthoredCommands, PackageCoordinate};
    use crate::steps::state::StepState;

    fn state(version: &str, outcomes: &[(&str, StepOutcome)]) -> StepsFile {
        StepsFile {
            source_evidence: Default::default(),
            operation: None,
            package: PackageCoordinate {
                group: "org.example".into(),
                name: "tool".into(),
                version: version.into(),
            },
            option: "main".into(),
            features: Vec::new(),
            steps: outcomes
                .iter()
                .map(|(id, outcome)| StepState {
                    id: (*id).into(),
                    kind: "recipe".into(),
                    outcome: *outcome,
                    reason: None,
                    started: None,
                    finished: None,
                    uninstall: AuthoredCommands::default(),
                    agent: None,
                    env: Vec::new(),
                    served: Vec::new(),
                })
                .collect(),
        }
    }

    fn work_dir(settings: &SettingsRoot, name: &str) -> PathBuf {
        let dir = settings.app_work_dir().join(name);
        for part in ["sources", "out", "inputs"] {
            fs::create_dir_all(dir.join(part)).unwrap();
        }
        dir
    }

    #[test]
    fn kept_directories_report_their_state_size_and_parts() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());

        let done = work_dir(&settings, "org.example.tool@1.0.0");
        fs::write(done.join("sources/upstream.tar"), vec![b'x'; 2048]).unwrap();
        fs::write(done.join("inputs/archive"), vec![b'y'; 16]).unwrap();
        state(
            "1.0.0",
            &[("fetch", StepOutcome::Done), ("build", StepOutcome::Done)],
        )
        .save(&done)
        .unwrap();

        let failed = work_dir(&settings, "org.example.tool@2.0.0");
        state(
            "2.0.0",
            &[
                ("fetch", StepOutcome::Done),
                ("fix", StepOutcome::Skipped),
                ("build", StepOutcome::Failed),
            ],
        )
        .save(&failed)
        .unwrap();

        let stale = work_dir(&settings, "org.example.other@1.0.0");
        fs::write(stale.join("out/binary"), vec![b'z'; 32]).unwrap();

        let listing = work(&settings, None).unwrap();
        let seen: Vec<(&str, &wire::EntryState, Option<&str>)> = listing
            .entries
            .iter()
            .map(|entry| (entry.key.as_str(), &entry.state, entry.step.as_deref()))
            .collect();
        assert_eq!(
            seen,
            [
                ("org.example/other@1.0.0", &wire::EntryState::Unknown, None),
                ("org.example/tool@1.0.0", &wire::EntryState::Done, None),
                (
                    "org.example/tool@2.0.0",
                    &wire::EntryState::Failed,
                    Some("build")
                ),
            ]
        );
        let kept = &listing.entries[1];
        assert_eq!(kept.parts.sources, 2048);
        assert_eq!(kept.parts.inputs, 16);
        assert_eq!(kept.parts.out, 0);
        assert!(kept.parts.other > 0, "steps.json is the rest");
        assert_eq!(
            kept.bytes,
            kept.parts.sources + kept.parts.inputs + kept.parts.other
        );
        assert!(kept.changed.ends_with('Z'), "{}", kept.changed);

        // One package narrows the listing; a version requirement selects.
        let one = work(&settings, Some("org.example/tool@2")).unwrap();
        assert_eq!(one.entries.len(), 1);
        assert_eq!(one.entries[0].key, "org.example/tool@2.0.0");
        assert_eq!(
            work(&settings, Some("org.example/absent"))
                .unwrap_err()
                .code(),
            "work_missing"
        );
    }

    #[test]
    fn a_clean_removes_only_the_selected_directories() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let one = work_dir(&settings, "org.example.tool@1.0.0");
        fs::write(one.join("sources/blob"), vec![b'x'; 64]).unwrap();
        let two = work_dir(&settings, "org.example.other@1.0.0");
        let sources = settings.app_sources_cache().join("abc");
        fs::create_dir_all(&sources).unwrap();
        let runs = settings.app_runs_dir().join("org.example.tool@1.0.0");
        fs::create_dir_all(&runs).unwrap();

        let cleaned = clean_work(&settings, Some("org.example/tool")).unwrap();
        assert_eq!(cleaned.removed, [one.display().to_string()]);
        assert!(cleaned.freed >= 64);
        assert!(!one.exists() && two.exists());
        assert!(sources.exists(), "the download cache is never touched");
        assert!(runs.exists(), "the run records are never touched");

        // The same home locked a second time: a sibling test forking a
        // child can still share the first lock's file for an instant, which
        // `lock()` rides out by retrying briefly (B-228).
        work_dir(&settings, "org.example.tool@1.0.0");
        let all = clean_work(&settings, None).unwrap();
        assert_eq!(all.removed.len(), 2);
        assert!(work(&settings, None).unwrap().entries.is_empty());
    }
}
