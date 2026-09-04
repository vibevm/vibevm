//! The pinned project-root capability and its no-follow directory walk.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use cap_fs_ext::DirExt;
use specmark::spec;

use crate::component::ensure_safe_component;

mod absolute;
mod enumerate;
mod reset;

pub use absolute::{PinnedAbsentPath, PinnedAbsoluteFile};

/// The pinned project-root capability every mutation goes through.
///
/// ```no_run
/// let project = vibe_safefs::Project::open(std::path::Path::new("/abs/project")).unwrap();
/// assert!(project.root_path().is_absolute());
/// ```
#[derive(Debug)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub struct Project {
    pub(crate) root: cap_std::fs::Dir,
    pub(crate) root_path: PathBuf,
}

/// A capability-pinned subdirectory of the project.
///
/// ```no_run
/// let project = vibe_safefs::Project::open(std::path::Path::new("/abs/project")).unwrap();
/// let docs: vibe_safefs::Pinned = project.dir(&["docs"], true).unwrap();
/// assert!(docs.join("guide.md").ends_with("guide.md"));
/// ```
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub struct Pinned {
    pub(crate) dir: cap_std::fs::Dir,
    pub(crate) path: PathBuf,
}

/// Why an exclusive child creation did not yield a capability — and,
/// decisively, whether this call ever created anything at that name.
///
/// `Result<Pinned>` collapses two opposite disk states into one `Err`. A caller
/// that treats them alike either walks away from a name its own `create_dir`
/// succeeded at — leaving an entry no later run can attribute — or reaches for
/// a name it never created, which belongs to whoever did.
///
/// The distinction the type carries is **what this call did**, not what is on
/// disk now. Only `create_dir` returning `Ok` is proved; between that and the
/// reopen the entry can be removed, renamed, or replaced by anything at all.
/// So `created()` licenses a *guarded revalidation* — reopen no-follow, verify,
/// and remove only what verifies as the empty directory this call made — and
/// nothing stronger.
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub enum ExclusiveChildError {
    /// Nothing was created: the name is unsafe, already taken, or the
    /// creation itself failed. Nothing at that name came from this caller, so
    /// touching it would be touching somebody else's entry.
    NotCreated(anyhow::Error),
    /// This call's `create_dir` succeeded, and the entry now at that name could
    /// **not** be reopened no-follow — so it could not be verified, and may
    /// have been removed, renamed or replaced since. The caller must revalidate
    /// under that doubt: clean up only what it can prove is the directory it
    /// made, and name whatever it cannot as residue. It may never be silently
    /// dropped, and it may never be deleted unverified.
    CreatedNotReopened {
        /// The absolute display path this call created, for diagnostics and for
        /// naming residue — never re-opened with ambient authority.
        path: PathBuf,
        source: anyhow::Error,
    },
}

impl ExclusiveChildError {
    /// Whether this call's own `create_dir` succeeded at that name.
    ///
    /// This is evidence about the past, not a claim about the present: it says
    /// the caller is the one that put an entry there, which is what licenses a
    /// *guarded* cleanup. It does not say the entry is still that one.
    #[must_use]
    pub const fn created(&self) -> bool {
        matches!(self, Self::CreatedNotReopened { .. })
    }
}

impl std::fmt::Display for ExclusiveChildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotCreated(source) => write!(f, "{source:#}"),
            Self::CreatedNotReopened { path, source } => write!(
                f,
                "{source:#} (this call created `{}`, but the entry now at that name could not \
                 be reopened no-follow and may have been replaced since)",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ExclusiveChildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotCreated(source) | Self::CreatedNotReopened { source, .. } => source.source(),
        }
    }
}

impl Project {
    /// Open the trusted absolute project root. The only ambient-authority open
    /// in this crate.
    pub fn open(project_root: &Path) -> Result<Self> {
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
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// The project root itself as a pinned directory.
    pub fn root_dir(&self) -> Result<Pinned> {
        Ok(Pinned {
            dir: self
                .root
                .try_clone()
                .context("retaining the project-root capability")?,
            path: self.root_path.clone(),
        })
    }

    /// Opaque filesystem identity of the pinned project root.
    pub fn root_identity(&self) -> Result<crate::file::identity::FileIdentity> {
        self.root_dir()?.identity()
    }

    /// Walk to a descendant directory one component at a time, refusing
    /// links/reparse points; create missing components when `create`.
    pub fn dir(&self, components: &[&str], create: bool) -> Result<Pinned> {
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
                    // Two processes can both observe the absence and both
                    // create. Losing that race is not a failure — the loser
                    // wanted the directory to exist, and it does. Only the
                    // reopen is load-bearing, because it is what proves the
                    // winner did not plant a link.
                    match dir.create_dir(component) {
                        Ok(()) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                        Err(error) => {
                            return Err(anyhow::Error::new(error)
                                .context(format!("creating `{}`", path.display())));
                        }
                    }
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
    pub fn dir_if_present(&self, components: &[&str]) -> Result<Option<Pinned>> {
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

    /// Open a chain of directories below an already-pinned `base`, recording
    /// every directory this invocation actually created. A caller that later
    /// fails needs that list: an empty `docs/nested/` is observable state, so
    /// "no output file landed" is not the same claim as "nothing changed".
    pub(crate) fn dir_at_recording(
        &self,
        base: &Pinned,
        components: &[&str],
        created: &mut Vec<PathBuf>,
    ) -> Result<Pinned> {
        let mut current = base.shallow_clone()?;
        for component in components {
            let (child, made) = current.ensure_child_recording(component)?;
            if made {
                created.push(child.path.clone());
            }
            current = child;
        }
        Ok(current)
    }

    /// Remove one directory directly inside `directory` when it is empty;
    /// `Ok(false)` for non-empty or absent. Links refuse. Emptiness is
    /// inspected through a no-follow child capability which is dropped before
    /// removal — Windows refuses to delete a directory an open handle still
    /// names.
    pub fn remove_dir_if_empty(&self, directory: &Pinned, name: &str) -> Result<bool> {
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
            Err(error) if crate::file::is_not_empty(&error) => Ok(false),
            Err(error) => Err(anyhow::Error::new(error).context(format!(
                "removing directory `{}`",
                directory.join(name).display()
            ))),
        }
    }

    /// Open (creating when absent) `.vibe/<name>` through the capability,
    /// verify it is a regular single-link file, take an exclusive OS file lock
    /// on it, and then prove the path still names the object that was locked.
    /// The handle is released by drop or process death; a second holder blocks
    /// until this one finishes.
    pub fn lock(&self, name: &str) -> Result<LockGuard> {
        match self.acquire(name, true)? {
            Some(guard) => Ok(guard),
            // A blocking acquisition only returns without a guard if the OS
            // said "would block", which `lock()` never does.
            None => bail!("locking `.vibe/{name}` returned no guard"),
        }
    }

    /// Try to take the same lock without blocking; `Ok(None)` while another
    /// holder owns it.
    pub fn try_lock(&self, name: &str) -> Result<Option<LockGuard>> {
        self.acquire(name, false)
    }

    /// The whole acquisition, including the recheck the naive version is
    /// missing.
    ///
    /// An OS file lock is taken on an **open file description**, not on a
    /// name. Between opening `.vibe/<name>` and locking the handle, another
    /// process can unlink that name and create a fresh file at it — and then
    /// two holders each own a lock, on two different objects, while both
    /// believe they own the project. So after the lock is held, the name is
    /// reopened through the same pinned capability and its identity compared
    /// to the locked handle's. A mismatch means the lock is on a stale object:
    /// it is released and the acquisition retried, because the winner of that
    /// race is whoever locks the file the path currently names.
    fn acquire(&self, name: &str, blocking: bool) -> Result<Option<LockGuard>> {
        ensure_safe_component(name)?;
        let vibe = self.dir(&[".vibe"], true)?;
        let display = vibe.join(name);
        for _ in 0..LOCK_ATTEMPTS {
            let mut options = crate::file::cap_options();
            let file = vibe
                .dir
                .open_with(name, options.read(true).write(true).create(true))
                .with_context(|| format!("opening lock `{}`", display.display()))?
                .into_std();
            crate::file::verify_regular_single_link(&file, &display)?;
            crate::race_hook::before_lock(&vibe, name);
            let held = if blocking {
                file.lock()
                    .map(|()| true)
                    .with_context(|| format!("locking `{}`", display.display()))?
            } else {
                match file.try_lock() {
                    Ok(()) => true,
                    Err(std::fs::TryLockError::WouldBlock) => false,
                    Err(error) => {
                        return Err(anyhow::Error::new(error)
                            .context(format!("try-locking `{}`", display.display())));
                    }
                }
            };
            if !held {
                return Ok(None);
            }
            if still_named(&vibe, name, &file, &display)? {
                return Ok(Some(LockGuard { _file: file }));
            }
            // Dropping releases the lock on the object the name no longer
            // means, so the next attempt can contend for the current one.
            drop(file);
        }
        bail!(
            "`{}` was replaced under every one of {LOCK_ATTEMPTS} lock attempts; refusing to \
             hold a lock on an object the path no longer names",
            display.display()
        )
    }
}

/// How many times an acquisition re-contends after losing the object it
/// locked. A name that is rebound this many times in a row is not a lock file
/// anyone is cooperating over.
const LOCK_ATTEMPTS: u32 = 8;

/// Whether `name` still resolves to the very object `locked` holds.
fn still_named(vibe: &Pinned, name: &str, locked: &std::fs::File, display: &Path) -> Result<bool> {
    let mut options = crate::file::cap_options();
    match vibe.dir.open_with(name, options.read(true)) {
        Ok(current) => {
            let current = current.into_std();
            crate::file::verify_regular_single_link(&current, display)?;
            let held = crate::file::identity::file_identity(locked, display)?;
            let named = crate::file::identity::file_identity(&current, display)?;
            Ok(crate::race_hook::lock_identity_matches(held == named))
        }
        // Unlinked under us: the lock is on an object with no name at all.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => {
            Err(anyhow::Error::new(error)
                .context(format!("rechecking lock `{}`", display.display())))
        }
    }
}

/// An exclusive cross-process lock held for one whole transaction.
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub struct LockGuard {
    /// Intentionally live: dropping it releases the OS lock.
    _file: std::fs::File,
}

impl Pinned {
    /// Opaque filesystem identity of this held directory capability.
    pub fn identity(&self) -> Result<crate::file::identity::FileIdentity> {
        let handle = self
            .dir
            .try_clone()
            .with_context(|| format!("retaining `{}` for identity", self.path.display()))?
            .into_std_file();
        crate::file::identity::file_identity(&handle, &self.path)
    }

    /// Exact Unix permission bits for this held directory; absent elsewhere.
    pub fn unix_mode(&self) -> Result<Option<u32>> {
        let metadata = self
            .dir
            .try_clone()
            .with_context(|| format!("retaining `{}` for mode", self.path.display()))?
            .into_std_file()
            .metadata()
            .with_context(|| format!("inspecting directory `{}`", self.path.display()))?;
        Ok(crate::file::unix_mode(&metadata))
    }

    /// Open one direct child directory, refusing reparse points.
    pub fn open_child(&self, name: &str) -> Result<Self> {
        ensure_safe_component(name)?;
        let child = self.join(name);
        let dir = self
            .dir
            .open_dir_nofollow(name)
            .with_context(|| format!("opening no-follow directory `{}`", child.display()))?;
        Ok(Self { path: child, dir })
    }

    /// Open one direct child directory without following links; `Ok(None)`
    /// when it does not exist. Every other failure — including a link, a
    /// reparse point and a non-directory occupant — propagates.
    pub fn open_child_checked(&self, name: &str) -> Result<Option<Self>> {
        ensure_safe_component(name)?;
        let child = self.join(name);
        match self.dir.open_dir_nofollow(name) {
            Ok(dir) => Ok(Some(Self { path: child, dir })),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(anyhow::Error::new(error)
                .context(format!("opening no-follow directory `{}`", child.display()))),
        }
    }

    /// [`ensure_child`](Self::ensure_child), reporting whether **this call**
    /// is the one that created the directory.
    ///
    /// The answer comes from the `create_dir` syscall, not from the earlier
    /// probe: between "it is absent" and "create it" another process can win,
    /// and a loser that reported `created = true` would make the caller offer
    /// to remove a directory somebody else owns. The loser reopens no-follow,
    /// so losing the race still proves the winner did not plant a link.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub fn ensure_child_recording(&self, name: &str) -> Result<(Self, bool)> {
        if let Some(existing) = self.open_child_checked(name)? {
            return Ok((existing, false));
        }
        ensure_safe_component(name)?;
        let child = self.join(name);
        crate::race_hook::before_create_dir(self, name);
        let created = match self.dir.create_dir(name) {
            Ok(()) => true,
            // Lost the race. The directory the caller asked for exists, but
            // this invocation did not make it, so it does not own it.
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
            Err(error) => {
                return Err(
                    anyhow::Error::new(error).context(format!("creating `{}`", child.display()))
                );
            }
        };
        let dir = self
            .dir
            .open_dir_nofollow(name)
            .with_context(|| format!("reopening `{}` after creation", child.display()))?;
        Ok((Self { path: child, dir }, created))
    }

    /// Open one direct child directory, creating it when absent; an existing
    /// link/reparse child refuses.
    pub fn ensure_child(&self, name: &str) -> Result<Self> {
        ensure_safe_component(name)?;
        let child = self.join(name);
        crate::race_hook::before_create_dir(self, name);
        match self.dir.open_dir_nofollow(name) {
            Ok(dir) => Ok(Self { path: child, dir }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // Same benign create race as `Project::dir`: the loser still
                // gets the directory it asked for, and still proves through
                // the no-follow reopen that it is a directory, not a link.
                match self.dir.create_dir(name) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => {
                        return Err(anyhow::Error::new(error)
                            .context(format!("creating `{}`", child.display())));
                    }
                }
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
    ///
    /// The failure is **discriminated**, because the two ways this can fail
    /// differ in what this call did, and a caller that cannot tell them apart
    /// must guess. `NotCreated` means nothing at that name came from here;
    /// `CreatedNotReopened` means this call's `create_dir` succeeded and the
    /// entry then could not be reopened no-follow — the one case a bare `?`
    /// turns into an entry nobody ever collects.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub fn create_child_exclusive(&self, name: &str) -> Result<Self, ExclusiveChildError> {
        let child = self.join(name);
        ensure_safe_component(name).map_err(ExclusiveChildError::NotCreated)?;
        self.dir
            .create_dir(name)
            .with_context(|| {
                format!(
                    "exclusively creating `{}` (existing entry refuses)",
                    child.display()
                )
            })
            .map_err(ExclusiveChildError::NotCreated)?;
        // From here this call has created an entry at that name. What is there
        // *now* is a separate question — the reopen below is what would answer
        // it — so every path out says only the first fact.
        if let Some(injected) = crate::race_hook::after_create_dir(self, name) {
            return Err(ExclusiveChildError::CreatedNotReopened {
                path: child,
                source: anyhow::Error::new(injected),
            });
        }
        match self.dir.open_dir_nofollow(name) {
            Ok(dir) => Ok(Self { path: child, dir }),
            Err(error) => Err(ExclusiveChildError::CreatedNotReopened {
                source: anyhow::Error::new(error)
                    .context(format!("reopening created `{}`", child.display())),
                path: child,
            }),
        }
    }

    pub(crate) fn shallow_clone(&self) -> Result<Self> {
        Ok(Self {
            path: self.path.clone(),
            dir: self.dir.try_clone().with_context(|| {
                format!("retaining the pinned capability `{}`", self.path.display())
            })?,
        })
    }

    /// The absolute display path of a child, for diagnostics only — never a
    /// path this crate then opens with ambient authority.
    #[must_use]
    pub fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    /// The absolute display path of this pinned directory.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Walk `components` below `base` without following links. `Ok(None)` when an
/// intermediate directory does not exist.
pub(crate) fn descend(base: &Pinned, components: &[&str]) -> std::io::Result<Option<Pinned>> {
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

#[cfg(test)]
#[path = "project/race_tests.rs"]
mod race_tests;

#[cfg(test)]
#[path = "project/reset_tests.rs"]
mod reset_tests;

#[cfg(test)]
#[path = "project/absolute_tests.rs"]
mod absolute_tests;
