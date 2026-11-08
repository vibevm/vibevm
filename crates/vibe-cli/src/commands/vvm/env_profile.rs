//! Read-only observation of the durable target selected by the existing persister.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3);
use super::{RawUserVar, Shell, managed_path_line, path_with_prefix, rc_io};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetKind {
    RcFile,
    WindowsRegistry,
    Unavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObservationStatus {
    Present,
    Missing,
    Unavailable,
    Malformed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BlockStatus {
    Present,
    Absent,
    Malformed,
    NotApplicable,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PathEntryStatus {
    Correct,
    Missing,
    Different,
    Ambiguous,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProbeStatus {
    NotRun,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Effectiveness {
    Unknown,
}

/// Exact target observation; correctness names the managed PATH entry only.
/// Neither the managed block nor registry state proves a future login PATH.
#[derive(Debug, Serialize)]
#[specmark::spec(
    documents = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
pub(crate) struct EnvironmentObservation {
    pub target_kind: TargetKind,
    pub target: Option<String>,
    pub shell: Option<Shell>,
    pub observation: ObservationStatus,
    pub managed_block: BlockStatus,
    pub path_entry: PathEntryStatus,
    pub login_probe: ProbeStatus,
    pub login_effectiveness: Effectiveness,
    pub error: Option<String>,
}
impl EnvironmentObservation {
    pub(crate) fn unavailable(error: String) -> Self {
        Self {
            target_kind: TargetKind::Unavailable,
            target: None,
            shell: None,
            observation: ObservationStatus::Unavailable,
            managed_block: BlockStatus::Unknown,
            path_entry: PathEntryStatus::Unknown,
            login_probe: ProbeStatus::NotRun,
            login_effectiveness: Effectiveness::Unknown,
            error: Some(error),
        }
    }
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
pub(super) fn inspect_rc(path: &Path, shell: Shell, dir: &Path) -> EnvironmentObservation {
    let mut result = EnvironmentObservation::unavailable(String::new());
    result.target_kind = TargetKind::RcFile;
    result.target = Some(path.display().to_string());
    result.shell = Some(shell);
    let (text, permissions) = match rc_io::read_rc_file(path) {
        Ok(value) => value,
        Err(error) => {
            result.error = Some(format!("{error:#}"));
            return result;
        }
    };
    result.error = None;
    result.observation = if permissions.is_some() {
        ObservationStatus::Present
    } else {
        ObservationStatus::Missing
    };
    match rc_io::managed_block_present(&text) {
        Ok(true) => result.managed_block = BlockStatus::Present,
        Ok(false) => {
            result.managed_block = BlockStatus::Absent;
            result.path_entry = PathEntryStatus::Missing;
            return result;
        }
        Err(error) => {
            result.observation = ObservationStatus::Malformed;
            result.managed_block = BlockStatus::Malformed;
            result.error = Some(format!("{error:#}"));
            return result;
        }
    }
    let (_, block, _) = match rc_io::split_block(&text) {
        Ok(value) => value,
        Err(error) => {
            result.error = Some(format!("{error:#}"));
            return result;
        }
    };
    let (prefix, expected) = managed_path_line(shell, dir);
    let entries: Vec<_> = block
        .iter()
        .filter(|line| line.starts_with(prefix))
        .collect();
    result.path_entry = match entries.as_slice() {
        [] => PathEntryStatus::Missing,
        [line] if **line == expected => PathEntryStatus::Correct,
        [_] => PathEntryStatus::Different,
        _ => PathEntryStatus::Ambiguous,
    };
    result
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
pub(super) fn inspect_registry(
    raw: anyhow::Result<Option<RawUserVar>>,
    dir: &Path,
) -> EnvironmentObservation {
    let mut result = EnvironmentObservation::unavailable(String::new());
    result.target_kind = TargetKind::WindowsRegistry;
    result.target = Some("HKCU\\Environment\\Path".into());
    result.managed_block = BlockStatus::NotApplicable;
    match raw {
        Err(error) => result.error = Some(format!("{error:#}")),
        Ok(None) => {
            result.error = None;
            result.observation = ObservationStatus::Missing;
            result.path_entry = PathEntryStatus::Missing;
        }
        Ok(Some(raw)) => {
            result.error = None;
            result.observation = ObservationStatus::Present;
            result.path_entry =
                if path_with_prefix(&raw.value, &dir.display().to_string()).is_none() {
                    PathEntryStatus::Correct
                } else {
                    PathEntryStatus::Different
                };
        }
    }
    result
}

#[cfg(test)]
#[path = "env_profile_tests.rs"]
mod tests;
