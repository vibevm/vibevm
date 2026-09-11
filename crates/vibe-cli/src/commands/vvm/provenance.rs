//! Runtime identity and source provenance for `self current`/`which`/`source`.

use std::path::{Path, PathBuf};

use anyhow::Result;

use super::error::VvmError;
use super::model::{InstallRecord, Origin};
use super::store::{INDEX_BINARY_NAME, VersionStore};
use super::{VvmEnv, git, selfloc, source};
use crate::cli::VvmWhichArgs;
use crate::output;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceExecution {
    pub executable: PathBuf,
    pub source: PathBuf,
    pub commit: String,
}

impl SourceExecution {
    fn human_identity(&self) -> String {
        format!(
            "source origin=external source={:?} executable={:?} commit={} sha256=- selector=-",
            self.source.display().to_string(),
            self.executable.display().to_string(),
            self.commit
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RunningIdentity {
    Installed(InstallRecord),
    Source(SourceExecution),
}

fn running_identity_at(
    store: &VersionStore,
    executable: &Path,
    commit_for: impl FnOnce(&Path) -> Option<String>,
) -> Result<Option<RunningIdentity>> {
    if let Some(location) = selfloc::derive_self(Some(executable))
        && let Some(record) = store.record_at(&location.home)?
    {
        return Ok(Some(RunningIdentity::Installed(record)));
    }
    let Some(source) = exact_root(executable) else {
        return Ok(None);
    };
    let executable = PathBuf::from(source::external_path(executable));
    let commit = commit_for(&source).unwrap_or_else(|| "unknown".to_string());
    Ok(Some(RunningIdentity::Source(SourceExecution {
        executable,
        source,
        commit,
    })))
}

pub(super) fn running_identity(store: &VersionStore) -> Result<Option<RunningIdentity>> {
    let Some(executable) = std::env::current_exe().ok() else {
        return Ok(None);
    };
    running_identity_at(store, &executable, |root| git::rev_parse(root, "HEAD").ok())
}

/// Prefer the managed executable's own immutable identity; bare/dev runs have none.
pub(super) fn running_record(store: &VersionStore) -> Result<Option<InstallRecord>> {
    Ok(match running_identity(store)? {
        Some(RunningIdentity::Installed(record)) => Some(record),
        _ => None,
    })
}

fn exact_root(path: &Path) -> Option<PathBuf> {
    source::find_source_root(path).map(|root| PathBuf::from(source::external_path(&root)))
}

pub(super) fn executable_source_root() -> Option<PathBuf> {
    std::env::current_exe().ok().as_deref().and_then(exact_root)
}

fn resolve_source_path(
    store: &VersionStore,
    record: Option<&InstallRecord>,
    executable: Option<&Path>,
    cwd: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(record) = record {
        return match record.origin {
            Origin::Binary => {
                let source = store.instance_source_dir(&record.version_id(), record.instance);
                source.is_dir().then(|| {
                    source
                        .canonicalize()
                        .unwrap_or_else(|_| source.to_path_buf())
                })
            }
            Origin::External => record
                .source_path
                .as_deref()
                .and_then(|path| exact_root(Path::new(path))),
            Origin::Managed => exact_root(&store.mirror_dir()),
        };
    }
    executable
        .and_then(exact_root)
        .or_else(|| cwd.and_then(exact_root))
}

fn current_source(store: &VersionStore, env: &VvmEnv) -> Result<(PathBuf, Option<InstallRecord>)> {
    let running = running_identity(store)?;
    if let Some(RunningIdentity::Source(source)) = running.as_ref() {
        return Ok((source.source.clone(), None));
    }
    let record = match running {
        Some(RunningIdentity::Installed(record)) => Some(record),
        None => store.active()?,
        Some(RunningIdentity::Source(_)) => unreachable!(),
    };
    let path = resolve_source_path(store, record.as_ref(), None, env.cwd.as_deref())
        .ok_or(VvmError::NoSource)?;
    Ok((path, record))
}

fn same_record(left: &InstallRecord, right: &InstallRecord) -> bool {
    left.version_id() == right.version_id() && left.instance == right.instance
}

fn markers(
    record: &InstallRecord,
    active: Option<&InstallRecord>,
    running: Option<&InstallRecord>,
) -> String {
    match (
        active.is_some_and(|item| same_record(item, record)),
        running.is_some_and(|item| same_record(item, record)),
    ) {
        (true, true) => "*>",
        (true, false) => "*",
        (false, true) => ">",
        (false, false) => " ",
    }
    .to_string()
}

fn running_json(identity: Option<&RunningIdentity>) -> serde_json::Value {
    match identity {
        Some(RunningIdentity::Installed(record)) => serde_json::json!({
            "kind": "installed",
            "selector": record.selector().to_string(),
            "origin": record.origin.as_str(),
            "source": record.source_label(),
            "commit": record.commit,
            "payload_sha256": record.payload_sha256,
        }),
        Some(RunningIdentity::Source(source)) => serde_json::json!({
            "kind": "source",
            "selector": null,
            "origin": "external",
            "source": source.source.display().to_string(),
            "executable": source.executable.display().to_string(),
            "commit": source.commit,
            "payload_sha256": null,
        }),
        None => serde_json::Value::Null,
    }
}

pub(super) fn run_ls_cmd(ctx: &output::Context, env: &VvmEnv) -> Result<()> {
    let store = env.store()?;
    let mut state = store.load_state()?;
    state
        .installs
        .sort_by(|a, b| a.id.cmp(&b.id).then(a.instance.cmp(&b.instance)));
    let active = store.active()?;
    let running = running_identity(&store)?;
    let installed_running = match running.as_ref() {
        Some(RunningIdentity::Installed(record)) => Some(record),
        _ => None,
    };

    if ctx.is_json() {
        let installs = state
            .installs
            .iter()
            .map(|record| {
                serde_json::json!({
                    "id": record.version_id().to_string(),
                    "selector": record.selector().to_string(),
                    "instance": record.instance,
                    "commit": record.commit,
                    "toolchain": record.toolchain,
                    "profile": record.profile.as_str(),
                    "origin": record.origin.as_str(),
                    "source": record.source_label(),
                    "source_path": record.source_path,
                    "payload_sha256": record.payload_sha256,
                    "installed_at": record.installed_at,
                    "active": active.as_ref().is_some_and(|item| same_record(item, record)),
                    "running": installed_running.is_some_and(|item| same_record(item, record)),
                })
            })
            .collect::<Vec<_>>();
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "self:ls",
            "active_selector": active.as_ref().map(|record| record.selector().to_string()),
            "running": running_json(running.as_ref()),
            "count": installs.len(),
            "installs": installs,
        }));
    }

    if state.installs.is_empty() && running.is_none() {
        ctx.summary("(no versions installed — run `vibe self install`)");
        return Ok(());
    }
    for record in &state.installs {
        ctx.step(&format!(
            "{} {}",
            markers(record, active.as_ref(), installed_running),
            record.human_identity()
        ));
    }
    if let Some(RunningIdentity::Source(source)) = running {
        ctx.step(&format!(" > {}", source.human_identity()));
    }
    ctx.summary(&format!("{} instance(s) installed.", state.installs.len()));
    Ok(())
}

pub(super) fn run_current_cmd(ctx: &output::Context, env: &VvmEnv) -> Result<()> {
    let store = env.store()?;
    let active = store.active()?;
    let running = running_identity(&store)?;
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "self:current",
            "active_selector": active.as_ref().map(|record| record.selector().to_string()),
            "active": active.as_ref().map(|record| serde_json::json!({
                "selector": record.selector().to_string(),
                "origin": record.origin.as_str(),
                "source": record.source_label(),
                "commit": record.commit,
                "payload_sha256": record.payload_sha256,
            })),
            "running": running_json(running.as_ref()),
        }));
    }

    match running.as_ref() {
        Some(RunningIdentity::Installed(record))
            if active
                .as_ref()
                .is_some_and(|active| same_record(active, record)) =>
        {
            ctx.summary(&format!("*> {}", record.human_identity()));
        }
        Some(RunningIdentity::Installed(record)) => {
            if let Some(active) = active.as_ref() {
                ctx.summary(&format!("*  {}", active.human_identity()));
            }
            ctx.summary(&format!(" > {}", record.human_identity()));
        }
        Some(RunningIdentity::Source(source)) => {
            if let Some(active) = active.as_ref() {
                ctx.summary(&format!("*  {}", active.human_identity()));
            }
            ctx.summary(&format!(" > {}", source.human_identity()));
        }
        None => match active {
            Some(record) => ctx.summary(&format!("*  {}", record.human_identity())),
            None => ctx.summary("(no active or identified running version)"),
        },
    }
    Ok(())
}

pub(super) fn run_source_cmd(ctx: &output::Context, env: &VvmEnv) -> Result<()> {
    let store = env.store()?;
    let (path, record) = current_source(&store, env)?;
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "self:source",
            "path": path.display().to_string(),
            "selector": record.as_ref().map(|record| record.selector().to_string()),
        }));
    }
    println!("{}", path.display());
    Ok(())
}

fn which_resolution(
    store: &VersionStore,
    running: Option<&RunningIdentity>,
    active: Option<&InstallRecord>,
    component: &str,
    cwd: Option<&Path>,
) -> Result<(PathBuf, Option<String>)> {
    if let Some(RunningIdentity::Source(source)) = running {
        let path = match component {
            "vibe" => source.executable.clone(),
            "vibe-index" => source.executable.with_file_name(INDEX_BINARY_NAME),
            "source" => source.source.clone(),
            other => return Err(VvmError::UnknownWhich(other.to_string()).into()),
        };
        return Ok((require_member(component, path)?, None));
    }
    let record = match running {
        Some(RunningIdentity::Installed(record)) => Some(record),
        _ => active,
    };
    let selector = record.map(|record| record.selector().to_string());
    let path = match component {
        "vibe" => {
            let record = record.ok_or(VvmError::NoActiveVersion)?;
            store.binary_path(&record.version_id(), record.instance)
        }
        "vibe-index" => {
            let record = record.ok_or(VvmError::NoActiveVersion)?;
            store.index_binary_path(&record.version_id(), record.instance)
        }
        "source" => resolve_source_path(store, record, None, cwd).ok_or(VvmError::NoSource)?,
        other => return Err(VvmError::UnknownWhich(other.to_string()).into()),
    };
    Ok((require_member(component, path)?, selector))
}

fn require_member(component: &str, path: PathBuf) -> Result<PathBuf> {
    let present = if component == "source" {
        path.is_dir()
    } else {
        path.is_file()
    };
    if !present {
        return Err(VvmError::MissingWhich {
            component: component.to_string(),
            path: path.display().to_string(),
        }
        .into());
    }
    Ok(path)
}

pub(super) fn run_which_cmd(ctx: &output::Context, env: &VvmEnv, args: VvmWhichArgs) -> Result<()> {
    let store = env.store()?;
    let running = running_identity(&store)?;
    let active = store.active()?;
    let (path, selector) = which_resolution(
        &store,
        running.as_ref(),
        active.as_ref(),
        &args.component,
        env.cwd.as_deref(),
    )?;
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "self:which",
            "component": args.component,
            "path": path.display().to_string(),
            "selector": selector,
        }));
    }
    println!("{}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::model::{Kind, Profile, VersionId};
    use super::*;

    fn source_root(root: &Path) {
        std::fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
        std::fs::create_dir_all(root.join("crates/vibe-cli")).unwrap();
    }

    fn record(origin: Origin, source_path: Option<String>) -> InstallRecord {
        InstallRecord {
            kind: Kind::Tag,
            id: "1.0.0".into(),
            instance: 4,
            commit: "a".repeat(40),
            toolchain: "prebuilt".into(),
            profile: Profile::Release,
            installed_at: "now".into(),
            origin,
            source_path,
            payload_sha256: Some("b".repeat(64)),
            distribution_manifest_sha256: None,
        }
    }

    #[test]
    fn binary_source_is_instance_owned_and_never_taken_from_cwd() {
        let temp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(temp.path().join("opt"));
        let record = record(Origin::Binary, None);
        let source = store.instance_source_dir(&record.version_id(), record.instance);
        std::fs::create_dir_all(&source).unwrap();
        let wrong = temp.path().join("wrong-worktree");
        std::fs::create_dir_all(&wrong).unwrap();
        source_root(&wrong);

        let actual = resolve_source_path(&store, Some(&record), None, Some(&wrong)).unwrap();
        assert_eq!(actual, source.canonicalize().unwrap());
    }

    #[test]
    fn source_execution_uses_executable_git_root_before_cwd() {
        let temp = tempfile::tempdir().unwrap();
        let running = temp.path().join("running");
        let cwd = temp.path().join("cwd");
        std::fs::create_dir_all(running.join("target/debug")).unwrap();
        std::fs::create_dir_all(&cwd).unwrap();
        source_root(&running);
        source_root(&cwd);
        let executable = running.join("target/debug/vibe");
        let store = VersionStore::new(temp.path().join("opt"));

        let actual = resolve_source_path(&store, None, Some(&executable), Some(&cwd)).unwrap();
        assert!(selfloc::same_location(actual, &running));
    }

    #[test]
    fn external_recorded_worktree_beats_an_unrelated_cwd() {
        let temp = tempfile::tempdir().unwrap();
        let linked = temp.path().join("linked");
        let cwd = temp.path().join("cwd");
        std::fs::create_dir_all(&linked).unwrap();
        std::fs::create_dir_all(&cwd).unwrap();
        source_root(&linked);
        source_root(&cwd);
        let record = record(
            Origin::External,
            Some(linked.canonicalize().unwrap().display().to_string()),
        );
        let store = VersionStore::new(temp.path().join("opt"));

        let actual = resolve_source_path(&store, Some(&record), None, Some(&cwd)).unwrap();
        assert!(selfloc::same_location(actual, &linked));
        assert_eq!(record.version_id(), VersionId::new(Kind::Tag, "1.0.0"));
    }

    #[test]
    fn direct_worktree_binary_beats_an_active_binary_instance_for_identity_and_which() {
        let temp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(temp.path().join("opt"));
        let active = record(Origin::Binary, None);
        store.record_install(active.clone()).unwrap();
        let active_home = store.instance_dir(&active.version_id(), active.instance);
        std::fs::create_dir_all(active_home.join("bin")).unwrap();
        std::fs::write(
            store.binary_path(&active.version_id(), active.instance),
            b"managed",
        )
        .unwrap();
        store.write_current(&active_home).unwrap();

        let worktree = temp.path().join("other-worktree");
        let executable = worktree.join("target/debug/vibe.exe");
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        source_root(&worktree);
        std::fs::write(&executable, b"source-run").unwrap();
        let commit = "c".repeat(40);
        let running = running_identity_at(&store, &executable, |_| Some(commit.clone()))
            .unwrap()
            .unwrap();
        let RunningIdentity::Source(source) = &running else {
            panic!("worktree executable must be source identity")
        };
        assert!(selfloc::same_location(&source.source, &worktree));
        assert_eq!(source.commit, commit);

        let (which_vibe, selector) =
            which_resolution(&store, Some(&running), Some(&active), "vibe", None).unwrap();
        assert!(selfloc::same_location(which_vibe, &executable));
        assert!(selector.is_none(), "source executions invent no local #N");
        let (which_source, _) =
            which_resolution(&store, Some(&running), Some(&active), "source", None).unwrap();
        assert!(selfloc::same_location(which_source, &worktree));
        let error = which_resolution(&store, Some(&running), Some(&active), "vibe-index", None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("is not built"));
        let sibling = executable.with_file_name(INDEX_BINARY_NAME);
        std::fs::write(&sibling, b"index").unwrap();
        let (which_index, _) =
            which_resolution(&store, Some(&running), Some(&active), "vibe-index", None).unwrap();
        assert!(selfloc::same_location(which_index, sibling));
        assert_eq!(
            store.active().unwrap().unwrap().selector(),
            active.selector()
        );
    }

    #[test]
    fn active_and_running_installed_markers_are_independent() {
        let active = record(Origin::Binary, None);
        let mut running = active.clone();
        running.instance += 1;
        assert_eq!(markers(&active, Some(&active), Some(&running)), "*");
        assert_eq!(markers(&running, Some(&active), Some(&running)), ">");
        assert_eq!(markers(&active, Some(&active), Some(&active)), "*>");
    }

    #[test]
    fn slash_branch_running_binary_maps_to_its_exact_installed_identity() {
        let temp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(temp.path().join("opt"));
        let mut record = record(Origin::External, Some("/source".into()));
        record.kind = Kind::Branch;
        record.id = "feature/versions/topic".into();
        record.instance = 7;
        let home = store.instance_dir(&record.version_id(), record.instance);
        let executable = home.join("bin").join(super::super::store::BINARY_NAME);
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, b"vibe").unwrap();
        store.record_install(record.clone()).unwrap();
        let location = selfloc::derive_self(Some(&executable)).unwrap();
        assert!(selfloc::same_location(&location.home, &home));
        assert_eq!(
            store.record_at(&location.home).unwrap(),
            Some(record.clone()),
            "home={home:?} derived={:?}",
            location.home
        );

        let identity = running_identity_at(&store, &executable, |_| None)
            .unwrap()
            .unwrap();
        assert_eq!(identity, RunningIdentity::Installed(record));
    }
}
