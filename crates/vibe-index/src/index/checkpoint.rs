//! Incremental-reindex bookkeeping — `<data-dir>/state/checkpoint.json`.
//! Records the last known head commit + tag list per package repo so
//! subsequent `reindex --incremental` runs only re-walk repos whose
//! state has changed.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

const FILENAME: &str = "checkpoint.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    #[serde(default = "default_schema")]
    pub schema_version: u32,
    #[serde(default)]
    pub generated_at: Option<DateTime<Utc>>,
    /// Repository directory name (under the org-dir) → snapshot.
    #[serde(default)]
    pub repos: BTreeMap<String, RepoSnapshot>,
}

impl Default for Checkpoint {
    fn default() -> Self {
        Checkpoint {
            schema_version: default_schema(),
            generated_at: None,
            repos: BTreeMap::new(),
        }
    }
}

fn default_schema() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoSnapshot {
    /// Commit SHA of the default branch's HEAD when the snapshot was taken.
    pub head_commit: Option<String>,
    /// Sorted list of `v<semver>` tags observed at snapshot time.
    pub tags: Vec<String>,
}

pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join("state").join(FILENAME)
}

pub fn load(data_dir: &Path) -> Result<Checkpoint> {
    let p = path(data_dir);
    let bytes = match std::fs::read(&p) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Checkpoint::default());
        }
        Err(e) => {
            return Err(Error::Io {
                path: p.clone(),
                message: e.to_string(),
            });
        }
    };
    serde_json::from_slice(&bytes).map_err(|e| Error::Malformed(format!("checkpoint.json: {e}")))
}

pub fn save(data_dir: &Path, checkpoint: &Checkpoint) -> Result<()> {
    let state_dir = data_dir.join("state");
    std::fs::create_dir_all(&state_dir).map_err(|e| Error::Io {
        path: state_dir.clone(),
        message: e.to_string(),
    })?;
    let mut bytes = serde_json::to_vec_pretty(checkpoint)
        .map_err(|e| Error::Malformed(format!("could not serialise checkpoint: {e}")))?;
    bytes.push(b'\n');
    crate::index::persistence::atomic_write(&path(data_dir), &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_returns_empty_checkpoint() {
        let dir = tempdir().unwrap();
        let cp = load(dir.path()).unwrap();
        assert!(cp.repos.is_empty());
        assert_eq!(cp.schema_version, 1);
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let mut cp = Checkpoint {
            schema_version: 1,
            generated_at: Some(
                DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            repos: BTreeMap::new(),
        };
        cp.repos.insert(
            "flow-wal".into(),
            RepoSnapshot {
                head_commit: Some("abc123".into()),
                tags: vec!["v0.1.0".into()],
            },
        );
        save(dir.path(), &cp).unwrap();
        let back = load(dir.path()).unwrap();
        assert_eq!(cp, back);
    }
}
