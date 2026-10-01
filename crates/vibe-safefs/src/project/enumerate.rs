//! Capability-relative directory enumeration, bounded and unbounded.
//!
//! Split out of the project cell it serves so neither outgrows the file-length
//! budget, and because both entry points are one question with one loop: what
//! names does this pinned directory hold, and how many of them may this caller
//! afford to learn about.
//!
//! The width fence is the load-bearing half. A tree walk that must not be
//! sized by what it finds cannot ask a directory for everything and judge the
//! answer afterwards — by then it has already paid. So the ceiling lives
//! inside the loop, and the unbounded entry point is the same loop with the
//! ceiling lifted rather than a second implementation of it.

use anyhow::{Context, Result, bail};
use cap_fs_ext::DirExt;

use super::{Pinned, Project};
use crate::component::ensure_safe_component;
use crate::file::cap_options;

/// One observed direct child kind. This does not retain a byte witness: a
/// later reader must open and verify that file through the held capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildKind {
    RegularFile,
    Directory,
}

impl Project {
    /// Classify one direct child of a held directory without following links.
    /// Missing is distinct from an unsafe link, reparse point, special file,
    /// or I/O failure. The result is a momentary kind observation, not a
    /// transaction or permission to consume later bytes from an ambient path.
    pub fn child_kind_nofollow(&self, directory: &Pinned, name: &str) -> Result<Option<ChildKind>> {
        ensure_safe_component(name)?;
        let display = directory.join(name);
        let metadata = match directory.dir.symlink_metadata(name) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(anyhow::Error::new(error).context(format!(
                    "inspecting child `{}` no-follow",
                    display.display()
                )));
            }
        };
        if metadata.file_type().is_symlink() {
            bail!("`{}` is a symlink or reparse point", display.display());
        }
        if metadata.is_dir() {
            directory
                .dir
                .open_dir_nofollow(name)
                .with_context(|| format!("opening no-follow directory `{}`", display.display()))?;
            return Ok(Some(ChildKind::Directory));
        }
        if metadata.is_file() {
            let mut options = cap_options();
            let opened = directory
                .dir
                .open_with(name, options.read(true))
                .with_context(|| format!("opening no-follow file `{}`", display.display()))?;
            let opened_metadata = opened.metadata()?;
            if opened_metadata.file_type().is_symlink() || !opened_metadata.is_file() {
                bail!("`{}` changed away from a regular file", display.display());
            }
            return Ok(Some(ChildKind::RegularFile));
        }
        bail!("`{}` is not a regular file or directory", display.display())
    }

    /// List the direct child names of `directory` through the retained
    /// capability; entry and non-UTF8-name errors propagate.
    ///
    /// Unbounded, and the one caller shape that may be: this is the whole set
    /// or an error. A walk that must not be sized by whatever it finds wants
    /// [`child_names_bounded`](Self::child_names_bounded) instead.
    pub fn child_names(&self, directory: &Pinned) -> Result<Vec<String>> {
        self.child_names_bounded(directory, usize::MAX)
    }

    /// The same enumeration under a mechanical width fence: refuse on the
    /// entry after `max`, **before** retaining it.
    ///
    /// The fence is where it is on purpose. A caller that lists first and
    /// counts afterwards has already paid for the directory it was trying not
    /// to pay for, so the ceiling has to be a property of the loop rather than
    /// of its answer. Nothing is truncated: a directory over the fence has no
    /// bounded answer, and returning its first `max` names would be a
    /// different directory's listing presented as this one's.
    ///
    /// Order is the filesystem's, deliberately: a canonical order is the
    /// caller's law (byte-wise over the names it got), and sorting here would
    /// hide from that caller which sort it is relying on.
    ///
    /// `max == usize::MAX` is the unbounded case and cannot overflow — the
    /// fence compares a retained count, it never computes `max + 1`.
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let home = tempfile::tempdir()?;
    /// let project = vibe_safefs::Project::open(home.path())?;
    /// project.write_atomic("pack/a.txt", b"a")?;
    /// project.write_atomic("pack/b.txt", b"b")?;
    /// let pack = project.dir(&["pack"], false)?;
    /// assert_eq!(project.child_names_bounded(&pack, 2)?.len(), 2);
    /// assert!(project.child_names_bounded(&pack, 1).is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn child_names_bounded(&self, directory: &Pinned, max: usize) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in directory
            .dir
            .entries()
            .with_context(|| format!("listing `{}`", directory.path.display()))?
        {
            let entry = entry.with_context(|| format!("listing `{}`", directory.path.display()))?;
            if names.len() == max {
                bail!(
                    "`{}` holds more than {max} direct children; refusing to retain another name \
                     rather than answer with a truncated listing",
                    directory.path.display()
                );
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                bail!("non-UTF8 name in `{}`", directory.path.display());
            };
            names.push(name.to_string());
        }
        Ok(names)
    }
}

#[cfg(test)]
#[path = "child_names_tests.rs"]
mod child_names_tests;

#[cfg(test)]
#[path = "entry_kind_tests.rs"]
mod entry_kind_tests;
