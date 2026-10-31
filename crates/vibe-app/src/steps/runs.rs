//! Run records (`PROP-065#runs`): every install, update, resume and
//! uninstall writes `<home>/cache/app-runs/<key>/<timestamp>/` with
//! `run.json`, the plan, `steps.json` and one `steps/<id>/output.log` per
//! executed step; `vibe app runs` lists them newest first and prunes them.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#runs");

use std::fs;
use std::path::{Path, PathBuf};

use vibe_wire::generated::application::e1::run as record;
use vibe_wire::generated::application_runs as listing;

use crate::error::AppError;
use crate::index::PackageCoordinate;
use crate::installer::io;
use crate::operations::Command;
use crate::resolve::qualified_ref;
use crate::settings::{SettingsRoot, write_atomic};

use super::plan::Plan;
use super::state::StepsFile;

/// How a recorded run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunOutcome {
    Done,
    Failed,
    Interrupted,
}

/// The open record of one run; dropping it unclosed records a failure.
pub(crate) struct RunRecord {
    dir: PathBuf,
    record: record::RunRecord,
    closed: bool,
}

const RUN_PROTOCOL: &str = "vibe-app-run/1";

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

impl RunRecord {
    /// Create the record directory and write `run.json` and the plan.
    pub fn open(
        settings: &SettingsRoot,
        package: &PackageCoordinate,
        command: Command,
        option: &str,
        plan: Option<&Plan>,
    ) -> Result<Self, AppError> {
        let base = settings.app_runs_dir().join(package.directory_name());
        let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ").to_string();
        let mut dir = base.join(&stamp);
        let mut suffix = 1;
        while dir.exists() {
            dir = base.join(format!("{stamp}-{suffix}"));
            suffix += 1;
        }
        fs::create_dir_all(&dir).map_err(|source| io("creating", &dir, source))?;
        let run = Self {
            dir,
            record: record::RunRecord {
                protocol: RUN_PROTOCOL.into(),
                key: package.key(),
                command: match command {
                    Command::Install => record::Command::Install,
                    Command::Update => record::Command::Update,
                    Command::Uninstall => record::Command::Uninstall,
                },
                option: option.into(),
                started: now(),
                finished: None,
                outcome: record::Outcome::Running,
            },
            closed: false,
        };
        run.write_json("run.json", &run.record)?;
        if let Some(plan) = plan {
            run.write_json("plan.json", &plan.to_wire())?;
        }
        Ok(run)
    }

    fn write_json(&self, name: &str, value: &impl serde::Serialize) -> Result<(), AppError> {
        let path = self.dir.join(name);
        let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| {
            io(
                "serialising",
                &path,
                std::io::Error::other(error.to_string()),
            )
        })?;
        bytes.push(b'\n');
        Ok(write_atomic(&path, &bytes)?)
    }

    /// `steps/<id>/output.log` of one step.
    pub fn log(&self, step: &str) -> PathBuf {
        self.dir.join("steps").join(step).join("output.log")
    }

    /// Mirror the work directory's `steps.json`; best effort.
    pub fn steps(&self, state: &StepsFile) {
        let _ = state.save(&self.dir);
    }

    /// Record how the run ended; best effort.
    pub fn close(&mut self, outcome: RunOutcome) {
        self.closed = true;
        self.record.finished = Some(now());
        self.record.outcome = match outcome {
            RunOutcome::Done => record::Outcome::Done,
            RunOutcome::Failed => record::Outcome::Failed,
            RunOutcome::Interrupted => record::Outcome::Interrupted,
        };
        let _ = self.write_json("run.json", &self.record);
    }
}

impl Drop for RunRecord {
    fn drop(&mut self) {
        if !self.closed {
            self.close(RunOutcome::Failed);
        }
    }
}

/// The run records of every installed version `spelling` matches, newest
/// first (`vibe app runs PACKAGE`).
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let listing = vibe_app::runs(&settings, "org.example/tool").unwrap();
/// assert_eq!(listing.protocol, "vibe-app-runs/1");
/// assert!(listing.runs.is_empty());
/// ```
pub fn runs(settings: &SettingsRoot, spelling: &str) -> Result<listing::ApplicationRuns, AppError> {
    let requested = qualified_ref(spelling)?;
    let group = requested
        .group
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    let prefix = format!("{group}.{}@", requested.name.as_str());
    let mut runs = Vec::new();
    for key_dir in entries(&settings.app_runs_dir()) {
        let name = key_dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let matches = name.strip_prefix(&prefix).is_some_and(|version| {
            semver::Version::parse(version).is_ok_and(|version| requested.version.matches(&version))
        });
        if !matches {
            continue;
        }
        for run_dir in entries(&key_dir) {
            if let Some(run) = read_run(&run_dir) {
                runs.push(run);
            }
        }
    }
    runs.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
    Ok(listing::ApplicationRuns {
        protocol: "vibe-app-runs/1".into(),
        package: spelling.into(),
        runs,
        pruned: Vec::new(),
    })
}

/// Keep the newest `keep` records `spelling` matches; delete the rest.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// assert!(vibe_app::prune_runs(&settings, "org.example/tool", 3).unwrap().pruned.is_empty());
/// ```
pub fn prune_runs(
    settings: &SettingsRoot,
    spelling: &str,
    keep: usize,
) -> Result<listing::ApplicationRuns, AppError> {
    let mut listed = runs(settings, spelling)?;
    let doomed = listed.runs.split_off(keep.min(listed.runs.len()));
    for run in doomed {
        let path = PathBuf::from(&run.path);
        fs::remove_dir_all(&path).map_err(|source| io("removing", &path, source))?;
        listed.pruned.push(run.path);
    }
    Ok(listed)
}

fn entries(dir: &Path) -> Vec<PathBuf> {
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

fn read_run(dir: &Path) -> Option<listing::Run> {
    let bytes = fs::read(dir.join("run.json")).ok()?;
    let run: record::RunRecord = serde_json::from_slice(&bytes).ok()?;
    let logs = entries(&dir.join("steps"))
        .into_iter()
        .filter_map(|step| {
            let log = step.join("output.log");
            log.is_file().then(|| listing::Log {
                step: step
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                path: log.display().to_string(),
            })
        })
        .collect();
    Some(listing::Run {
        timestamp: dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        key: run.key,
        command: serde_json::to_value(run.command)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default(),
        outcome: serde_json::to_value(run.outcome)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default(),
        started: run.started,
        finished: run.finished,
        path: dir.display().to_string(),
        logs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::StepOutcome;
    use crate::steps::state::StepState;

    #[test]
    fn records_are_written_listed_newest_first_and_pruned() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let package = PackageCoordinate {
            group: "org.example".into(),
            name: "tool".into(),
            version: "1.0.0".into(),
        };
        let mut first =
            RunRecord::open(&settings, &package, Command::Install, "main", None).unwrap();
        let log = first.log("build");
        fs::create_dir_all(log.parent().unwrap()).unwrap();
        fs::write(&log, "built\n").unwrap();
        first.steps(&StepsFile {
            package: package.clone(),
            option: "main".into(),
            features: Vec::new(),
            steps: vec![StepState {
                id: "build".into(),
                kind: "recipe".into(),
                outcome: StepOutcome::Interrupted,
                reason: None,
                started: None,
                finished: None,
                uninstall: Default::default(),
            }],
        });
        first.close(RunOutcome::Interrupted);
        let second =
            RunRecord::open(&settings, &package, Command::Uninstall, "main", None).unwrap();
        drop(second);
        let listed = runs(&settings, "org.example/tool").unwrap();
        assert_eq!(listed.protocol, "vibe-app-runs/1");
        let summary: Vec<(&str, &str)> = listed
            .runs
            .iter()
            .map(|run| (run.command.as_str(), run.outcome.as_str()))
            .collect();
        assert_eq!(
            summary,
            [("uninstall", "failed"), ("install", "interrupted")]
        );
        assert_eq!(listed.runs[1].logs[0].step, "build");
        assert!(Path::new(&listed.runs[1].path).join("steps.json").is_file());
        assert!(
            runs(&settings, "org.example/other")
                .unwrap()
                .runs
                .is_empty()
        );
        let pruned = prune_runs(&settings, "org.example/tool", 1).unwrap();
        assert_eq!(pruned.runs.len(), 1);
        assert_eq!(pruned.pruned.len(), 1);
        assert_eq!(
            runs(&settings, "org.example/tool").unwrap().runs[0].command,
            "uninstall"
        );
    }
}
