use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};

use super::super::store::{BINARY_NAME, INDEX_BINARY_NAME, VersionStore, open_regular_no_follow};

static TEMP_NONCE: AtomicU64 = AtomicU64::new(1);

fn posix_shim(command: &str, binary_name: &str) -> String {
    format!(
        "#!/bin/sh\n\
         # vibevm (VVM) shim — execs the active instance from ../vibevm/current.\n\
         self=\"$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\"\n\
         home=\"$VIBEVM_SHELL_HOME\"\n\
         [ -z \"$home\" ] && home=\"$(cat \"$self/../vibevm/current\" 2>/dev/null)\"\n\
         [ -z \"$home\" ] && home=\"$VIBEVM_HOME\"\n\
         if [ -z \"$home\" ]; then\n\
         \x20 echo '{command}: no active version — run: vibe self use <selector>' >&2\n\
         \x20 exit 1\n\
         fi\n\
         program=\"$home/bin/{binary_name}\"\n\
         [ ! -f \"$program\" ] && program=\"$home/{binary_name}\"\n\
         exec \"$program\" \"$@\"\n"
    )
}

fn cmd_shim(command: &str, binary_name: &str) -> String {
    format!(
        "@echo off\r\n\
         set \"VVM_CUR=%~dp0..\\vibevm\\current\"\r\n\
         set \"VVM_HOME=%VIBEVM_SHELL_HOME%\"\r\n\
         if \"%VVM_HOME%\"==\"\" if exist \"%VVM_CUR%\" set /p VVM_HOME=<\"%VVM_CUR%\"\r\n\
         if \"%VVM_HOME%\"==\"\" set \"VVM_HOME=%VIBEVM_HOME%\"\r\n\
         if \"%VVM_HOME%\"==\"\" (\r\n\
         echo {command}: no active version - run: vibe self use ^<selector^> 1>&2\r\n\
         exit /b 1\r\n\
         )\r\n\
         set \"VVM_EXE=%VVM_HOME%\\bin\\{binary_name}\"\r\n\
         if not exist \"%VVM_EXE%\" set \"VVM_EXE=%VVM_HOME%\\{binary_name}\"\r\n\
         \"%VVM_EXE%\" %*\r\n"
    )
}

pub(crate) fn write_shims(store: &VersionStore) -> Result<()> {
    let bin_dir = store.shim_dir();
    store.guard_mutation_tree(&bin_dir)?;
    fs::create_dir_all(&bin_dir).with_context(|| format!("creating `{}`", bin_dir.display()))?;
    for (command, binary_name) in [("vibe", BINARY_NAME), ("vibe-index", INDEX_BINARY_NAME)] {
        let posix = bin_dir.join(command);
        store.guard_mutation_path(&posix)?;
        write_shim_atomic(&posix, posix_shim(command, binary_name).as_bytes(), true)?;
        if cfg!(windows) {
            let cmd = bin_dir.join(format!("{command}.cmd"));
            store.guard_mutation_path(&cmd)?;
            write_shim_atomic(&cmd, cmd_shim(command, binary_name).as_bytes(), false)?;
        }
    }
    Ok(())
}

pub(crate) fn shim_statuses(store: &VersionStore) -> Vec<(&'static str, bool)> {
    let bin_dir = store.shim_dir();
    [("vibe", BINARY_NAME), ("vibe-index", INDEX_BINARY_NAME)]
        .into_iter()
        .map(|(command, binary_name)| {
            let posix = shim_file_matches(
                &bin_dir.join(command),
                posix_shim(command, binary_name).as_bytes(),
                true,
            );
            let cmd = !cfg!(windows)
                || shim_file_matches(
                    &bin_dir.join(format!("{command}.cmd")),
                    cmd_shim(command, binary_name).as_bytes(),
                    false,
                );
            (command, posix && cmd)
        })
        .collect()
}

fn shim_file_matches(path: &Path, expected: &[u8], executable: bool) -> bool {
    let Ok((file, metadata)) = open_regular_no_follow(path) else {
        return false;
    };
    if metadata.len() != expected.len() as u64 {
        return false;
    }
    #[cfg(unix)]
    if executable {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return false;
        }
    }
    #[cfg(not(unix))]
    let _ = executable;
    let mut actual = Vec::with_capacity(expected.len());
    file.take(expected.len() as u64 + 1)
        .read_to_end(&mut actual)
        .is_ok()
        && actual == expected
}

pub(super) fn write_shim_atomic(path: &Path, bytes: &[u8], executable: bool) -> Result<()> {
    write_shim_atomic_using(path, executable, |file| {
        file.write_all(bytes)?;
        file.sync_all()
    })
}

pub(super) fn write_shim_atomic_using(
    path: &Path,
    executable: bool,
    write: impl FnOnce(&mut fs::File) -> io::Result<()>,
) -> Result<()> {
    let parent = path.parent().context("shim path has no parent")?;
    let nonce = TEMP_NONCE.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("shim");
    let temp = parent.join(format!(".{name}.{}-{nonce}.tmp", std::process::id()));
    write_shim_atomic_at(path, &temp, executable, write)
}

pub(super) fn write_shim_atomic_at(
    path: &Path,
    temp: &Path,
    executable: bool,
    write: impl FnOnce(&mut fs::File) -> io::Result<()>,
) -> Result<()> {
    let mut owned = false;
    let result = (|| -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temp)
            .with_context(|| format!("creating `{}`", temp.display()))?;
        owned = true;
        write(&mut file).with_context(|| format!("writing `{}`", temp.display()))?;
        drop(file);
        if executable {
            set_executable(temp)?;
        }
        fs::rename(temp, path)
            .with_context(|| format!("atomically replacing shim `{}`", path.display()))?;
        Ok(())
    })();
    if result.is_err() && owned {
        let _ = fs::remove_file(temp);
    }
    result
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("chmod +x `{}`", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}
