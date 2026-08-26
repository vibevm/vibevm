//! Capability-relative, no-follow filesystem primitives for package-skill
//! mutation.
//!
//! Static pathname checks are not mutation safety: a path re-checked then used
//! through ordinary `std::fs` calls can be redirected by a symlink/junction
//! swapped into an ancestor between check and use. This module pins mutation
//! to capabilities: the trusted absolute project root is opened once with
//! ambient authority, every descendant directory is reached one authored
//! component at a time with `open_dir_nofollow`, final file opens disable
//! symlink following, and publication is a same-directory staged file plus a
//! capability-relative `Dir::rename`. Directory capabilities stay pinned for
//! the whole mutation, so a namespace swap after the walk cannot redirect a
//! write that goes through them.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};

/// The pinned project-root capability every package-skill mutation goes
/// through.
#[derive(Debug)]
pub(crate) struct Project {
    root: cap_std::fs::Dir,
    root_path: PathBuf,
}

/// A capability-pinned subdirectory of the project.
#[derive(Debug)]
pub(crate) struct Pinned {
    dir: cap_std::fs::Dir,
    path: PathBuf,
}

impl Project {
    /// Open the trusted absolute project root. The only ambient-authority
    /// open in this module.
    pub(crate) fn open(project_root: &Path) -> Result<Self> {
        if !project_root.is_absolute() {
            bail!(
                "project root must be absolute: `{}`",
                project_root.display()
            );
        }
        let root = cap_std::fs::Dir::open_ambient_dir(project_root, cap_std::ambient_authority())
            .with_context(|| format!("opening project root `{}`", project_root.display()))?;
        Ok(Self {
            root,
            root_path: project_root.to_path_buf(),
        })
    }

    /// The absolute trusted root this capability was opened from.
    #[must_use]
    pub(crate) fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// Walk to a descendant directory one component at a time, refusing
    /// links/reparse points; create missing components when `create`.
    pub(crate) fn dir(&self, components: &[&str], create: bool) -> Result<Pinned> {
        let mut dir = self
            .root
            .try_clone()
            .context("retaining the project-root capability")?;
        let mut path = self.root_path.clone();
        for component in components {
            ensure_safe_component(component)?;
            path.push(component);
            match dir.open_dir_nofollow(component) {
                Ok(child) => dir = child,
                Err(error) if create && error.kind() == std::io::ErrorKind::NotFound => {
                    dir.create_dir(component)
                        .with_context(|| format!("creating `{}`", path.display()))?;
                    dir = dir
                        .open_dir_nofollow(component)
                        .with_context(|| format!("reopening created `{}`", path.display()))?;
                }
                Err(error) => {
                    return Err(anyhow::Error::new(error)
                        .context(format!("opening no-follow directory `{}`", path.display())));
                }
            }
        }
        Ok(Pinned { dir, path })
    }

    /// Walk to a descendant directory, returning `Ok(None)` only when it (or
    /// an ancestor of it) does not exist. Every other failure propagates:
    /// removal paths must not collapse arbitrary errors into absence.
    pub(crate) fn dir_if_present(&self, components: &[&str]) -> Result<Option<Pinned>> {
        let mut dir = self
            .root
            .try_clone()
            .context("retaining the project-root capability")?;
        let mut path = self.root_path.clone();
        for component in components {
            ensure_safe_component(component)?;
            path.push(component);
            match dir.open_dir_nofollow(component) {
                Ok(child) => dir = child,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(None);
                }
                Err(error) => {
                    return Err(anyhow::Error::new(error)
                        .context(format!("opening no-follow directory `{}`", path.display())));
                }
            }
        }
        Ok(Some(Pinned { dir, path }))
    }

    /// Atomically write one file at a forward-slashed relative path below
    /// `directory`: missing parents are created no-follow, the staging file
    /// lives in the destination's own directory, syncs, is renamed over the
    /// destination, the visible result is reopened and byte-verified, and the
    /// destination directory is synced where the platform supports it.
    /// Never a truncate-in-place write.
    pub(crate) fn write_atomic(
        &self,
        directory: &Pinned,
        relative: &str,
        bytes: &[u8],
    ) -> Result<()> {
        let (parents, name) = split_relative(relative)?;
        let destination = if parents.is_empty() {
            directory.shallow_clone()?
        } else {
            let chain = parents.iter().map(String::as_str).collect::<Vec<_>>();
            self.dir_at(directory, &chain, true)?
        };
        let staged = format!(".vibe-stage-{}", std::process::id());
        let mut options = cap_options();
        let staged_file = destination
            .dir
            .open_with(&staged, options.read(true).write(true).create_new(true))
            .with_context(|| format!("staging beside `{}`", destination.join(&name).display()))?;
        let mut std_file = staged_file.into_std();
        std_file
            .write_all(bytes)
            .and_then(|()| std_file.flush())
            .and_then(|()| std_file.sync_all())
            .with_context(|| format!("writing staged `{}`", destination.join(&staged).display()))?;
        destination
            .dir
            .rename(&staged, &destination.dir, &name)
            .with_context(|| format!("publishing `{}`", destination.join(&name).display()))?;
        // Reopen the visible file through the capability and verify the
        // exact bytes landed.
        match self.read_file(&destination, &name)? {
            Some(visible) if visible.as_slice() == bytes => {}
            Some(_) => bail!(
                "published bytes of `{}` do not match the staged bytes",
                destination.join(&name).display()
            ),
            None => bail!(
                "published file `{}` is absent immediately after publication",
                destination.join(&name).display()
            ),
        }
        // Directory sync is best-effort: some platforms do not support
        // fsync on directory handles.
        if let Ok(handle) = destination.dir.try_clone() {
            let _ = handle.into_std_file().sync_all();
        }
        Ok(())
    }

    /// Read one file at a relative path below `directory`, or `None` when
    /// absent. A link, a non-regular file, or a hard link count != 1 refuses.
    pub(crate) fn read_file(&self, directory: &Pinned, relative: &str) -> Result<Option<Vec<u8>>> {
        let (parents, name) = split_relative(relative)?;
        let holder = if parents.is_empty() {
            directory.shallow_clone()?
        } else {
            let chain = parents.iter().map(String::as_str).collect::<Vec<_>>();
            match descend(directory, &chain) {
                Ok(Some(holder)) => holder,
                // A missing intermediate directory means the file is absent.
                Ok(None) => return Ok(None),
                Err(error) => {
                    return Err(anyhow::Error::new(error).context(format!(
                        "opening no-follow directory below `{}`",
                        directory.path.display()
                    )));
                }
            }
        };
        let mut options = cap_options();
        match holder.dir.open_with(&name, options.read(true)) {
            Ok(file) => {
                let std_file = file.into_std();
                verify_regular_single_link(&std_file, &holder.join(&name))?;
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut &std_file, &mut bytes)
                    .with_context(|| format!("reading `{}`", holder.join(&name).display()))?;
                Ok(Some(bytes))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(anyhow::Error::new(error)
                .context(format!("opening `{}`", holder.join(&name).display()))),
        }
    }

    /// Remove one file at a relative path below `directory`; `Ok(false)`
    /// when absent. Links and non-regular files refuse.
    pub(crate) fn remove_file(&self, directory: &Pinned, relative: &str) -> Result<bool> {
        let (parents, name) = split_relative(relative)?;
        let holder = if parents.is_empty() {
            directory.shallow_clone()?
        } else {
            let chain = parents.iter().map(String::as_str).collect::<Vec<_>>();
            match descend(directory, &chain) {
                Ok(Some(holder)) => holder,
                // A missing intermediate directory means the file is absent.
                Ok(None) => return Ok(false),
                Err(error) => {
                    return Err(anyhow::Error::new(error).context(format!(
                        "opening no-follow directory below `{}`",
                        directory.path.display()
                    )));
                }
            }
        };
        let mut options = cap_options();
        match holder.dir.open_with(&name, options.read(true)) {
            Ok(file) => {
                let std_file = file.into_std();
                verify_regular_single_link(&std_file, &holder.join(&name))?;
                holder
                    .dir
                    .remove_file(&name)
                    .with_context(|| format!("removing `{}`", holder.join(&name).display()))?;
                Ok(true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(anyhow::Error::new(error)
                .context(format!("opening `{}`", holder.join(&name).display()))),
        }
    }

    /// Open a chain of directories below an already-pinned `base`.
    fn dir_at(&self, base: &Pinned, components: &[&str], create: bool) -> Result<Pinned> {
        let mut current = base.shallow_clone()?;
        for component in components {
            current = if create {
                current.ensure_child(component)?
            } else {
                current.open_child(component)?
            };
        }
        Ok(current)
    }

    /// Remove one directory directly inside `directory` when it is empty;
    /// `Ok(false)` for non-empty or absent. Links refuse. Emptiness is
    /// inspected through a no-follow child capability which is dropped
    /// before removal — Windows refuses to delete a directory an open
    /// handle still names.
    pub(crate) fn remove_dir_if_empty(&self, directory: &Pinned, name: &str) -> Result<bool> {
        ensure_safe_component(name)?;
        let child = match directory.dir.open_dir_nofollow(name) {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => {
                return Err(anyhow::Error::new(error).context(format!(
                    "opening no-follow directory `{}`",
                    directory.join(name).display()
                )));
            }
        };
        let empty = child
            .entries()
            .with_context(|| format!("listing `{}`", directory.join(name).display()))?
            .next()
            .is_none();
        drop(child);
        if !empty {
            return Ok(false);
        }
        match directory.dir.remove_dir(name) {
            Ok(()) => Ok(true),
            Err(error) if is_not_empty(&error) => Ok(false),
            Err(error) => Err(anyhow::Error::new(error).context(format!(
                "removing directory `{}`",
                directory.join(name).display()
            ))),
        }
    }

    /// List the direct child names of `directory` through the retained
    /// capability; entry and non-UTF8-name errors propagate.
    pub(crate) fn child_names(&self, directory: &Pinned) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in directory
            .dir
            .entries()
            .with_context(|| format!("listing `{}`", directory.path.display()))?
        {
            let entry = entry.with_context(|| format!("listing `{}`", directory.path.display()))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                bail!("non-UTF8 name in `{}`", directory.path.display());
            };
            names.push(name.to_string());
        }
        Ok(names)
    }

    /// Open (creating when absent) `.vibe/package-skills.lock` through the
    /// capability, verify it is a regular single-link file, then take an
    /// exclusive OS file lock on it. The handle is released by drop or
    /// process death; a second process blocks until the holder finishes.
    pub(crate) fn lock(&self) -> Result<LockGuard> {
        let vibe = self.dir(&[".vibe"], true)?;
        let mut options = cap_options();
        let file = vibe
            .dir
            .open_with(
                "package-skills.lock",
                options.read(true).write(true).create(true),
            )
            .with_context(|| {
                format!(
                    "opening lock `{}`",
                    vibe.join("package-skills.lock").display()
                )
            })?;
        let std_file = file.into_std();
        verify_regular_single_link(&std_file, &vibe.join("package-skills.lock"))?;
        std_file
            .lock()
            .with_context(|| "locking `.vibe/package-skills.lock`")?;
        Ok(LockGuard { _file: std_file })
    }
}

/// An exclusive cross-process lock held for one whole receipt transaction.
#[derive(Debug)]
pub(crate) struct LockGuard {
    /// Intentionally live: dropping it releases the OS lock.
    _file: std::fs::File,
}

#[cfg(test)]
impl LockGuard {
    /// Try to take the lock without blocking; `Ok(None)` while another
    /// process holds it. Test-only, reserved for the next child-process
    /// concurrency packet.
    #[allow(dead_code)]
    pub(crate) fn try_acquire(project: &Project) -> Result<Option<Self>> {
        let vibe = project.dir(&[".vibe"], true)?;
        let mut options = cap_options();
        let file = vibe
            .dir
            .open_with(
                "package-skills.lock",
                options.read(true).write(true).create(true),
            )
            .with_context(|| "opening lock `.vibe/package-skills.lock`")?;
        let std_file = file.into_std();
        verify_regular_single_link(&std_file, &vibe.join("package-skills.lock"))?;
        match std_file.try_lock() {
            Ok(()) => Ok(Some(Self { _file: std_file })),
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(error) => Err(anyhow::Error::new(error).context("try-locking package-skills lock")),
        }
    }
}

impl Pinned {
    /// Open one direct child directory, refusing reparse points.
    pub(crate) fn open_child(&self, name: &str) -> Result<Self> {
        ensure_safe_component(name)?;
        let child = self.join(name);
        let dir = self
            .dir
            .open_dir_nofollow(name)
            .with_context(|| format!("opening no-follow directory `{}`", child.display()))?;
        Ok(Self { path: child, dir })
    }

    /// Open one direct child directory, creating it when absent; an existing
    /// link/reparse child refuses.
    pub(crate) fn ensure_child(&self, name: &str) -> Result<Self> {
        ensure_safe_component(name)?;
        let child = self.join(name);
        match self.dir.open_dir_nofollow(name) {
            Ok(dir) => Ok(Self { path: child, dir }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.dir
                    .create_dir(name)
                    .with_context(|| format!("creating `{}`", child.display()))?;
                let dir = self
                    .dir
                    .open_dir_nofollow(name)
                    .with_context(|| format!("reopening created `{}`", child.display()))?;
                Ok(Self { path: child, dir })
            }
            Err(error) => Err(anyhow::Error::new(error).context(format!(
                "ensuring no-follow directory `{}`",
                child.display()
            ))),
        }
    }

    /// Create one direct child directory exclusively; an existing entry —
    /// including a crash leftover or attacker-planted spelling — refuses
    /// rather than being reused.
    pub(crate) fn create_child_exclusive(&self, name: &str) -> Result<Self> {
        ensure_safe_component(name)?;
        let child = self.join(name);
        self.dir.create_dir(name).with_context(|| {
            format!(
                "exclusively creating `{}` (existing entry refuses)",
                child.display()
            )
        })?;
        let dir = self
            .dir
            .open_dir_nofollow(name)
            .with_context(|| format!("reopening created `{}`", child.display()))?;
        Ok(Self { path: child, dir })
    }

    fn shallow_clone(&self) -> Result<Self> {
        Ok(Self {
            path: self.path.clone(),
            dir: self.dir.try_clone().with_context(|| {
                format!("retaining the pinned capability `{}`", self.path.display())
            })?,
        })
    }

    pub(crate) fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

fn cap_options() -> cap_std::fs::OpenOptions {
    let mut options = cap_std::fs::OpenOptions::new();
    options.follow(FollowSymlinks::No);
    options
}

/// Refuse anything but a regular single-link file: no directories, no
/// devices, no symlink/junction objects, no hard links shared with another
/// name.
fn verify_regular_single_link(file: &std::fs::File, display: &Path) -> Result<()> {
    let metadata = std::fs::File::metadata(file)
        .with_context(|| format!("inspecting `{}`", display.display()))?;
    if !metadata.is_file() {
        bail!("`{}` is not a regular file", display.display());
    }
    let links = number_of_links(file, &metadata, display)?;
    if links != 1 {
        bail!(
            "`{}` has {} names (hard link); refusing to treat it as exclusively owned",
            display.display(),
            links
        );
    }
    Ok(())
}

#[cfg(windows)]
fn number_of_links(
    file: &std::fs::File,
    _metadata: &std::fs::Metadata,
    display: &Path,
) -> Result<u64> {
    let information = winapi_util::file::information(file)
        .with_context(|| format!("inspecting `{}`", display.display()))?;
    Ok(information.number_of_links())
}

#[cfg(not(windows))]
fn number_of_links(
    _file: &std::fs::File,
    metadata: &std::fs::Metadata,
    _display: &Path,
) -> Result<u64> {
    use std::os::unix::fs::MetadataExt;
    Ok(metadata.nlink())
}

#[cfg(windows)]
fn is_not_empty(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(145)
}

#[cfg(not(windows))]
fn is_not_empty(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(39)
}

/// Walk `components` below `base` without following links. `Ok(None)` when
/// an intermediate directory does not exist.
fn descend(base: &Pinned, components: &[&str]) -> std::io::Result<Option<Pinned>> {
    let mut current = Pinned {
        path: base.path.clone(),
        dir: base
            .dir
            .try_clone()
            .map_err(|error| std::io::Error::other(format!("retaining capability: {error}")))?,
    };
    for component in components {
        match current.dir.open_dir_nofollow(component) {
            Ok(dir) => {
                current = Pinned {
                    path: current.join(component),
                    dir,
                };
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(error) => return Err(error),
        }
    }
    Ok(Some(current))
}

/// Split a forward-slashed relative path into parent components and file
/// name, refusing empty segments, dot/dot-dot, device names, and
/// dot/space-ended components.
pub(crate) fn split_relative(relative: &str) -> Result<(Vec<String>, String)> {
    let parts: Vec<&str> = relative.split('/').collect();
    let Some((last, parents)) = parts.split_last() else {
        bail!("empty relative path `{relative}`");
    };
    for component in parents {
        ensure_safe_component(component)
            .with_context(|| format!("in relative path `{relative}`"))?;
    }
    ensure_safe_component(last).with_context(|| format!("in relative path `{relative}`"))?;
    Ok((
        parents.iter().map(|c| (*c).to_string()).collect(),
        (*last).to_string(),
    ))
}

/// The mutation boundary judges components through the **one** portable
/// component law, never a second table of its own.
fn ensure_safe_component(component: &str) -> Result<()> {
    if !super::containment::valid_path_component(component) {
        bail!("unsafe relative component `{component}`");
    }
    Ok(())
}
