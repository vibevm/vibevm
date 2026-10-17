//! Exact workspace input witnesses across the plan/confirmation boundary.

use std::path::PathBuf;
use vibe_workspace::{Workspace, WorkspaceError};

#[derive(Debug, Clone)]
pub(crate) struct FilePreconditions(Vec<(PathBuf, Option<Vec<u8>>)>);

fn read(path: &std::path::Path) -> Result<Option<Vec<u8>>, WorkspaceError> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Ok(meta) if meta.is_file() => std::fs::read(path).map(Some).map_err(|e| failure(path, e)),
        Ok(_) => Err(failure(path, "input is not a regular file")),
        Err(error) => Err(failure(path, error)),
    }
}
fn failure(path: &std::path::Path, reason: impl ToString) -> WorkspaceError {
    WorkspaceError::Io {
        path: path.to_path_buf(),
        reason: reason.to_string(),
    }
}
impl FilePreconditions {
    pub(crate) fn capture(workspace: &Workspace) -> Result<Self, WorkspaceError> {
        let mut paths = workspace
            .iter_nodes()
            .map(|(rel, _)| workspace.node_abs_path(rel).join("vibe.toml"))
            .collect::<Vec<_>>();
        paths.push(workspace.lockfile_path());
        Ok(Self(
            paths
                .into_iter()
                .map(|path| read(&path).map(|bytes| (path, bytes)))
                .collect::<Result<_, _>>()?,
        ))
    }
    pub(crate) fn validate(&self) -> Result<(), WorkspaceError> {
        for (path, before) in &self.0 {
            if &read(path)? != before {
                return Err(failure(
                    path,
                    "workspace input changed since planning; refusing before writes",
                ));
            }
        }
        Ok(())
    }
}
