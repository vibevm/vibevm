//! Best-effort same-process rollback for stable native build staging.

use std::path::Path;

use vibe_safefs::{Project, StableFileState};
use vibe_wire::generated::native::e1::build_request as request;

use crate::ARTIFACT_RECORD_DIR;
use crate::mechanism::contain::{checked_relative, join_relative, walk_tree};

struct SavedFile {
    backup: String,
    destination: String,
    state: StableFileState,
}

struct SavedRecord {
    destination: String,
    saved: Option<SavedFile>,
}

pub(super) struct Snapshot {
    project_root: std::path::PathBuf,
    backup_root: String,
    staging_root: String,
    staging_present: bool,
    staging: Vec<SavedFile>,
    records: Vec<SavedRecord>,
}

pub(super) fn staging(
    project_root: &Path,
    build_root: &str,
    target: &str,
) -> Result<(request::StagingAuthority, std::path::PathBuf), String> {
    let relative = checked_relative(&format!("{build_root}/vibe-native/{target}"))
        .map_err(|error| error.reason().to_owned())?;
    let absolute = join_relative(project_root, &relative);
    Ok((
        request::StagingAuthority {
            root_absolute: vibe_core::machine_json_path(&absolute),
            root_relative: relative,
        },
        absolute,
    ))
}

impl Snapshot {
    pub(super) fn capture<'id>(
        project_root: &Path,
        build_root: &str,
        staging_root: &str,
        target: &str,
        output_ids: impl Iterator<Item = &'id str>,
    ) -> Result<Self, String> {
        let project = Project::open(project_root).map_err(report)?;
        let backup_root = format!("{build_root}/.vibe-native-rollback-{target}");
        let staging_present = project
            .dir_if_present(&staging_root.split('/').collect::<Vec<_>>())
            .map_err(report)?
            .is_some();
        project.reset_dir(&backup_root).map_err(report)?;
        let outcome = (|| {
            let staging = if staging_present {
                snapshot_tree(&project, project_root, staging_root, &backup_root)?
            } else {
                Vec::new()
            };
            let mut records = Vec::new();
            for id in output_ids {
                let destination = format!("{ARTIFACT_RECORD_DIR}/{id}.json");
                let backup = format!("{backup_root}/records/{id}.json");
                let saved = snapshot_file(&project, &destination, &backup)?;
                records.push(SavedRecord { destination, saved });
            }
            Ok((staging, records))
        })();
        let (staging, records) = match outcome {
            Ok(value) => value,
            Err(error) => {
                let _ = project.remove_dir_all_if_present(&backup_root);
                return Err(error);
            }
        };
        Ok(Self {
            project_root: project_root.to_path_buf(),
            backup_root,
            staging_root: staging_root.to_owned(),
            staging_present,
            staging,
            records,
        })
    }

    pub(super) fn restore(self) -> Result<(), String> {
        let project = Project::open(&self.project_root).map_err(report)?;
        if self.staging_present {
            project.reset_dir(&self.staging_root).map_err(report)?;
            for file in &self.staging {
                restore_file(&project, file)?;
            }
        } else {
            remove_any(&project, &self.staging_root)?;
        }
        for record in &self.records {
            match &record.saved {
                Some(file) => restore_file(&project, file)?,
                None => {
                    project.remove_file(&record.destination).map_err(report)?;
                }
            }
        }
        project
            .remove_dir_all_if_present(&self.backup_root)
            .map(|_| ())
            .map_err(report)
    }

    /// Retire post-commit backup state. A later capture always resets the
    /// same private root, so cleanup is GC and cannot invalidate output and
    /// records that have already committed successfully.
    pub(super) fn commit(self) {
        if let Ok(project) = Project::open(&self.project_root) {
            let _ = remove_any(&project, &self.backup_root);
        }
    }
}

fn snapshot_tree(
    project: &Project,
    project_root: &Path,
    staging_root: &str,
    backup_root: &str,
) -> Result<Vec<SavedFile>, String> {
    let absolute = staging_root
        .split('/')
        .fold(project_root.to_path_buf(), |path, part| path.join(part));
    let files =
        walk_tree(&absolute).map_err(|error| format!("{}: {}", error.path, error.reason))?;
    let mut saved = Vec::with_capacity(files.len());
    for (relative, _) in files {
        let source = format!("{staging_root}/{relative}");
        let backup = format!("{backup_root}/staging/{relative}");
        if let Some(file) = snapshot_file(project, &source, &backup)? {
            saved.push(file);
        } else {
            return Err(format!("staging file `{source}` vanished during snapshot"));
        }
    }
    Ok(saved)
}

fn snapshot_file(
    project: &Project,
    source: &str,
    backup: &str,
) -> Result<Option<SavedFile>, String> {
    let Some(state) = project.stable_file_state(source).map_err(report)? else {
        return Ok(None);
    };
    project
        .copy_stable_file_to_expected(
            source,
            project,
            backup,
            state.unix_mode,
            &state.sha256,
            state.bytes,
        )
        .map_err(|error| format!("{:#}", error.into_report()))?;
    Ok(Some(SavedFile {
        backup: backup.to_owned(),
        destination: source.to_owned(),
        state,
    }))
}

fn restore_file(project: &Project, file: &SavedFile) -> Result<(), String> {
    match project
        .stable_file_state(&file.destination)
        .map_err(report)?
    {
        Some(current) if current == file.state => return Ok(()),
        Some(_) => {
            project.remove_file(&file.destination).map_err(report)?;
        }
        None => {}
    }
    project
        .copy_stable_file_to_expected(
            &file.backup,
            project,
            &file.destination,
            file.state.unix_mode,
            &file.state.sha256,
            file.state.bytes,
        )
        .map(|_| ())
        .map_err(|error| format!("{:#}", error.into_report()))
}

fn remove_any(project: &Project, relative: &str) -> Result<(), String> {
    match project.remove_dir_all_if_present(relative) {
        Ok(_) => Ok(()),
        Err(directory) => project.remove_file(relative).map(|_| ()).map_err(|file| {
            format!(
                "could not remove new staging as a directory ({directory:#}) or file ({file:#})"
            )
        }),
    }
}

fn report(error: impl std::fmt::Display) -> String {
    format!("{error:#}")
}
