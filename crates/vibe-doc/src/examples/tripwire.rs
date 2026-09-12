//! The tripwire: proof that the run stayed inside its sandbox
//! (PROP-057 `##PIPE-EXAMPLE-RUNNER`).
//!
//! The isolation variables are not the guarantee. A variable can be set to
//! the wrong thing, a command can resolve a path a different way, and the
//! failure mode is silent: the examples still pass, and the operator's real
//! state has moved. The guarantee is a before-and-after comparison, the
//! same posture `tools/user-home-tripwire.sh` takes for the test suite.
//!
//! Two subjects, and the second one is narrow on purpose:
//!
//! * **the real per-user home** (`~/.vibe`) — walked whole. It is the
//!   state an example could plausibly write, and the one whose loss the
//!   operator would notice last.
//! * **the source tree** — not walked whole (a checkout with a build
//!   directory in it is millions of files and the walk would cost more
//!   than the run). The guard covers the places a `vibe` run has actually
//!   been seen to touch: the materialised dependency tree, the project's
//!   own `.vibe/`, the lock file and the manifest, plus the top level, so
//!   a NEW stray file at the root is caught by name.
//!
//! What the tripwire deliberately does not cover is the documentation
//! package's own pages: `--accept` writes there, and a guard that forbade
//! it would forbid the feature.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EXAMPLE-RUNNER");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::{DocError, Result};
use crate::examples::sandbox::RunnerEnv;

/// Paths inside the source tree a `vibe` run has been observed to write.
const GUARDED: &[&str] = &["vibevm/vibedeps", ".vibe", "vibe.lock", "vibe.toml"];

/// A recorded state: path → (size, modification time in nanoseconds).
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    entries: BTreeMap<String, (u64, u128)>,
}

/// Record the state the run must not change.
pub fn snapshot(env: &RunnerEnv) -> Snapshot {
    let mut entries = BTreeMap::new();
    if let Some(home) = &env.settings_home {
        walk(home, home, &mut entries, "home");
    }
    if let Some(repo) = &env.repo_root {
        for rel in GUARDED {
            let path = repo.join(rel);
            walk(&path, repo, &mut entries, "repo");
        }
        // The top level by name only: a stray file appearing at the root
        // of the checkout is the failure this half exists to catch.
        if let Ok(read) = std::fs::read_dir(repo) {
            for entry in read.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                entries.insert(format!("repo-top:{name}"), (0, 0));
            }
        }
    }
    Snapshot { entries }
}

/// Compare the state now against the recording, and refuse the run if
/// anything moved.
pub fn verify(before: &Snapshot, env: &RunnerEnv) -> Result<()> {
    let after = snapshot(env);
    let mut moved: Vec<String> = Vec::new();
    for (key, value) in &after.entries {
        match before.entries.get(key) {
            None => moved.push(format!("appeared: {key}")),
            Some(old) if old != value => moved.push(format!("changed: {key}")),
            Some(_) => {}
        }
    }
    for key in before.entries.keys() {
        if !after.entries.contains_key(key) {
            moved.push(format!("vanished: {key}"));
        }
    }
    if moved.is_empty() {
        return Ok(());
    }
    moved.truncate(20);
    Err(DocError::Tripwire {
        message: moved.join("; "),
    })
}

fn walk(path: &Path, base: &Path, into: &mut BTreeMap<String, (u64, u128)>, tag: &str) {
    if !path.exists() {
        return;
    }
    for entry in walkdir::WalkDir::new(path)
        .sort_by_file_name()
        .into_iter()
        .flatten()
    {
        let Ok(rel) = entry.path().strip_prefix(base) else {
            continue;
        };
        let key = format!("{tag}:{}", rel.to_string_lossy().replace('\\', "/"));
        let (size, modified) = match entry.metadata() {
            Ok(meta) => (
                meta.len(),
                meta.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos())
                    .unwrap_or(0),
            ),
            Err(_) => (0, 0),
        };
        into.insert(key, (size, modified));
    }
}

/// Where the tripwire looked, for the report's closing line.
pub fn subjects(env: &RunnerEnv) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(home) = &env.settings_home {
        out.push(home.clone());
    }
    if let Some(repo) = &env.repo_root {
        out.extend(GUARDED.iter().map(|rel| repo.join(rel)));
    }
    out
}
