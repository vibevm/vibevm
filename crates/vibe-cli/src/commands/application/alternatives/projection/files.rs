//! No-follow file operations used by the projection transaction.

use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail};
use vibe_safefs::{Project, ensure_no_follow_walk};

use super::CURRENT_PROTOCOL;
use super::{managed_dir, read_regular};
use serde_json::{Value, json};

static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(in crate::commands::application::alternatives) fn proxy_filename(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

pub(super) fn sidecar_path(alias: &Path) -> Result<PathBuf> {
    let name = alias
        .file_name()
        .and_then(OsStr::to_str)
        .context("alias has no filename")?;
    Ok(alias.with_file_name(format!("{name}.proxy.json")))
}

pub(super) fn write_sidecar(
    project: &Project,
    settings: &Path,
    path: &Path,
    bytes: &[u8],
) -> Result<()> {
    ensure_no_follow_walk(settings, path, true)?;
    let parent = path.parent().context("sidecar has no parent")?;
    let relative = parent
        .strip_prefix(settings)
        .context("sidecar escapes settings root")?;
    let parts = relative
        .iter()
        .map(|part| {
            part.to_str()
                .ok_or_else(|| anyhow::anyhow!("non-UTF8 projection path"))
        })
        .collect::<Result<Vec<_>>>()?;
    let dir = managed_dir(project, &parts)?;
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .context("sidecar has no filename")?;
    project
        .write_atomic_durable_in(&dir, name, bytes)
        .map_err(|error| error.into_report())?;
    Ok(())
}

pub(super) fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    if path_present(path)? {
        bail!("projection file already exists `{}`", path.display());
    }
    let (staging, mut file) = create_staging_file(path)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        if path_present(path)? {
            bail!("projection file already exists `{}`", path.display());
        }
        fs::rename(&staging, path)
            .with_context(|| format!("publishing staged projection file `{}`", path.display()))?;
        sync_parent(path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&staging);
    }
    result
}

fn create_staging_file(path: &Path) -> Result<(PathBuf, File)> {
    let parent = path.parent().context("projection file has no parent")?;
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .context("projection file has no UTF-8 filename")?;
    for _ in 0..128 {
        let staging = parent.join(staging_name(name, "file"));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)
        {
            Ok(file) => return Ok((staging, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("creating staged projection file `{}`", staging.display())
                });
            }
        }
    }
    bail!("could not allocate a unique projection staging file")
}

pub(super) fn publish_immutable_directory<F>(parent: &Path, name: &str, build: F) -> Result<bool>
where
    F: FnOnce(&Path) -> Result<()>,
{
    let destination = parent.join(name);
    if path_present(&destination)? {
        return Ok(false);
    }
    let staging = create_staging_directory(parent, name)?;
    let result = (|| {
        build(&staging)?;
        sync_tree(&staging)?;
        if path_present(&destination)? {
            bail!(
                "immutable projection directory appeared during preparation `{}`",
                destination.display()
            );
        }
        fs::rename(&staging, &destination).with_context(|| {
            format!(
                "publishing immutable projection directory `{}`",
                destination.display()
            )
        })?;
        sync_directory(parent)?;
        Ok(true)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn create_staging_directory(parent: &Path, name: &str) -> Result<PathBuf> {
    for _ in 0..128 {
        let staging = parent.join(staging_name(name, "dir"));
        match fs::create_dir(&staging) {
            Ok(()) => return Ok(staging),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "creating immutable projection staging directory `{}`",
                        staging.display()
                    )
                });
            }
        }
    }
    bail!("could not allocate a unique projection staging directory")
}

fn staging_name(name: &str, kind: &str) -> String {
    let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!(".{name}.vibe-{kind}-{}-{sequence}", std::process::id())
}

fn sync_tree(path: &Path) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            sync_tree(&entry.path())?;
        } else if file_type.is_file() {
            File::open(entry.path())?.sync_all()?;
        } else {
            bail!(
                "refusing non-file in projection staging tree `{}`",
                entry.path().display()
            );
        }
    }
    sync_directory(path)
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(windows)]
fn sync_directory(path: &Path) -> Result<()> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?
        .sync_all()?;
    Ok(())
}

fn sync_parent(path: &Path) -> Result<()> {
    sync_directory(path.parent().context("projection path has no parent")?)
}

pub(super) fn link_or_copy(source: &Path, destination: &Path) -> Result<()> {
    if path_present(destination)? {
        bail!(
            "projection destination already exists `{}`",
            destination.display()
        );
    }
    if fs::hard_link(source, destination).is_ok() {
        sync_parent(destination)?;
        return Ok(());
    }

    let (staging, mut to) = create_staging_file(destination)?;
    let result = (|| {
        let mut from = File::open(source)?;
        std::io::copy(&mut from, &mut to)?;
        to.sync_all()?;
        drop(to);
        set_executable(&staging)?;
        File::open(&staging)?.sync_all()?;
        if path_present(destination)? {
            bail!(
                "projection destination already exists `{}`",
                destination.display()
            );
        }
        fs::rename(&staging, destination)?;
        sync_parent(destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&staging);
    }
    result
}

pub(super) fn set_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

pub(super) fn path_present(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn remove_intent(settings: &Path, intent: &Path, expected: &[u8]) -> Result<()> {
    ensure_no_follow_walk(settings, intent, true)?;
    if path_present(intent)? {
        if read_regular(intent)? != expected {
            bail!("application alternative intent changed after preparation");
        }
        fs::remove_file(intent)?;
    }
    Ok(())
}

pub(super) fn write_current(
    project: &Project,
    settings: &Path,
    current: &Path,
    projection: &str,
) -> Result<()> {
    ensure_no_follow_walk(settings, current, true)?;
    let parent = current.parent().context("current pointer has no parent")?;
    let relative = parent
        .strip_prefix(settings)
        .context("current pointer escapes settings root")?;
    let parts = relative
        .iter()
        .map(|part| {
            part.to_str()
                .ok_or_else(|| anyhow::anyhow!("non-UTF8 projection path"))
        })
        .collect::<Result<Vec<_>>>()?;
    let dir = managed_dir(project, &parts)?;
    let bytes = serde_json::to_vec(&json!({"protocol":CURRENT_PROTOCOL,"projection":projection}))?;
    project
        .write_atomic_durable_in(&dir, "current.json", &bytes)
        .map_err(|error| error.into_report())
        .context("publishing application current pointer")?;
    Ok(())
}

pub(super) fn prior_pointer(path: &Path) -> Result<Option<Value>> {
    if !path_present(path)? {
        return Ok(None);
    }
    let bytes = read_regular(path)?;
    if bytes.len() > 1024 {
        bail!("prior application pointer exceeds size limit");
    }
    let value: Value =
        serde_json::from_slice(&bytes).context("parsing prior application pointer")?;
    if value.get("protocol").and_then(Value::as_str) != Some(CURRENT_PROTOCOL) {
        bail!("prior application pointer uses an unsupported protocol");
    }
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_immutable_build_never_publishes_partial_directory() -> Result<()> {
        let root = tempfile::tempdir()?;
        let result = publish_immutable_directory(root.path(), "object", |staging| {
            write_new(&staging.join("owner.json"), b"owned")?;
            bail!("injected preparation failure")
        });

        assert!(result.is_err());
        assert!(!root.path().join("object").exists());
        assert!(fs::read_dir(root.path())?.next().is_none());
        Ok(())
    }

    #[test]
    fn immutable_directory_appears_only_after_the_complete_tree_is_synced() -> Result<()> {
        let root = tempfile::tempdir()?;
        assert!(publish_immutable_directory(
            root.path(),
            "object",
            |staging| {
                let commands = staging.join("commands");
                fs::create_dir(&commands)?;
                write_new(&staging.join("owner.json"), b"owned")?;
                write_new(&commands.join("editor.json"), b"descriptor")?;
                assert!(!root.path().join("object").exists());
                Ok(())
            }
        )?);

        let object = root.path().join("object");
        assert_eq!(fs::read(object.join("owner.json"))?, b"owned");
        assert_eq!(
            fs::read(object.join("commands/editor.json"))?,
            b"descriptor"
        );
        assert!(!publish_immutable_directory(
            root.path(),
            "object",
            |_| bail!("an existing immutable object must not be rebuilt")
        )?);
        Ok(())
    }

    #[test]
    fn abandoned_staging_directory_does_not_block_retry() -> Result<()> {
        let root = tempfile::tempdir()?;
        let abandoned = root.path().join(".object.vibe-dir-abandoned");
        fs::create_dir(&abandoned)?;
        write_new(&abandoned.join("owner.json"), b"partial")?;

        assert!(publish_immutable_directory(
            root.path(),
            "object",
            |staging| write_new(&staging.join("owner.json"), b"complete")
        )?);
        assert_eq!(
            fs::read(root.path().join("object/owner.json"))?,
            b"complete"
        );
        assert_eq!(fs::read(abandoned.join("owner.json"))?, b"partial");
        Ok(())
    }
}
