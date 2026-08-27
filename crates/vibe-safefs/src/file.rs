//! Capability-relative file publication, reading and probing.
//!
//! Publication is never a truncate-in-place write. A **unique owned** staging
//! file is created with `create_new` beside the destination — `create_new` is
//! the ownership proof: it fails rather than reusing a leftover, an
//! attacker-planted spelling, or another caller's in-flight stage, so this
//! crate never overwrites or deletes a neighbour it does not own. The stage is
//! written, synced, renamed through the pinned directory capability, and the
//! visible result is reopened and byte-verified under the candidate's own
//! length cap, so verification of a hostile replacement is bounded by what the
//! caller wrote. A namespace swap after the
//! walk cannot redirect any of it: every step goes through the capability, not
//! through a path re-resolved with ambient authority.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, bail};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use specmark::spec;

use crate::component::{STAGE_PREFIX, split_relative};
use crate::project::{Pinned, Project, descend};
use crate::publish::{PublishError, Published};

mod bounded;
mod create_new;
pub(crate) mod identity;
#[cfg(any(test, feature = "inject-failures"))]
pub use create_new::{fail_before_publish, fail_before_stage_cleanup};
pub(crate) use identity::is_not_empty;
use identity::{FileIdentity, file_identity, number_of_links};

/// How many distinct staging names to try before refusing. Exceeding this
/// means something is minting `.vibe-stage-*` faster than we can claim one.
const STAGE_ATTEMPTS: u32 = 64;

/// What a declared path looks like on disk right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub enum Presence {
    /// A regular, single-link, non-empty file reachable without following a
    /// link or reparse point.
    RegularNonEmpty,
    /// Nothing at that path, or a missing ancestor directory.
    Absent,
    /// Present but unusable: empty, a directory, a link/reparse point, or a
    /// hard link shared with another name.
    Unusable,
}

impl Project {
    /// Atomically publish one file at a forward-slashed project-relative path.
    /// Missing parents are created no-follow.
    pub fn write_atomic(&self, relative: &str, bytes: &[u8]) -> Result<Published, PublishError> {
        let root = self
            .root_dir()
            .map_err(|error| PublishError::before(Vec::new(), error))?;
        self.write_atomic_in(&root, relative, bytes)
    }

    /// The same publication, relative to an already-pinned directory.
    ///
    /// Failures carry [`PublishStage`](crate::PublishStage): everything up to
    /// the rename is provably invisible, the rename and the verification after
    /// it are not. Created directories are reported either way.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub fn write_atomic_in(
        &self,
        directory: &Pinned,
        relative: &str,
        bytes: &[u8],
    ) -> Result<Published, PublishError> {
        let mut created: Vec<std::path::PathBuf> = Vec::new();
        let before = |created: &[std::path::PathBuf], error: anyhow::Error| {
            PublishError::before(created.to_vec(), error)
        };
        let (parents, name) = split_relative(relative).map_err(|error| before(&created, error))?;
        let destination = if parents.is_empty() {
            directory
                .shallow_clone()
                .map_err(|error| before(&created, error))?
        } else {
            let chain = parents.iter().map(String::as_str).collect::<Vec<_>>();
            self.dir_at_recording(directory, &chain, &mut created)
                .map_err(|error| before(&created, error))?
        };
        // Refuse a destination that is already a link/reparse point or a
        // directory before staging anything beside it.
        refuse_unpublishable_destination(&destination, &name)
            .map_err(|error| before(&created, error))?;
        let (staged_name, staged_file) =
            create_unique_stage(&destination).map_err(|error| before(&created, error))?;
        let mut std_file = staged_file.into_std();
        let written = std_file
            .write_all(bytes)
            .and_then(|()| std_file.flush())
            .and_then(|()| std_file.sync_all());
        drop(std_file);
        if let Err(error) = written {
            let _ = destination.dir.remove_file(&staged_name);
            return Err(before(
                &created,
                anyhow::Error::new(error).context(format!(
                    "writing staged `{}`",
                    destination.join(&staged_name).display()
                )),
            ));
        }
        if let Some(injected) = create_new::injected_pre_publication_failure(relative) {
            let _ = destination.dir.remove_file(&staged_name);
            return Err(before(&created, injected));
        }
        if let Err(error) = destination
            .dir
            .rename(&staged_name, &destination.dir, &name)
        {
            // Our own stage, created with `create_new`: removing it removes
            // nothing anyone else owns. A failed rename leaves the destination
            // untouched, so this stays `BeforePublication`.
            let _ = destination.dir.remove_file(&staged_name);
            return Err(before(
                &created,
                anyhow::Error::new(error).context(format!(
                    "publishing `{}`",
                    destination.join(&name).display()
                )),
            ));
        }
        // Past this line the destination entry may already be the new bytes.
        let possibly = |error: anyhow::Error| PublishError::possibly(created.clone(), error);
        // The window a hostile replacement of the just-published file lands
        // in: the rename is done and the verification read is the next step.
        crate::race_hook::before_publish_verify(&destination, &name);
        // The verification read is bounded by the candidate's own byte length,
        // never an unbounded read of whatever the name holds now: a racing
        // replacement larger than the candidate refuses at its own metadata —
        // spending nothing past `cap + 1` — instead of allocating a foreign
        // payload to compare against, and it never offers a prefix as success.
        match self
            .read_file_bounded_in(&destination, &name, bytes.len())
            .map_err(&possibly)?
        {
            Some(visible) if visible.as_slice() == bytes => {}
            Some(_) => {
                return Err(possibly(anyhow::anyhow!(
                    "published bytes of `{}` do not match the staged bytes",
                    destination.join(&name).display()
                )));
            }
            None => {
                return Err(possibly(anyhow::anyhow!(
                    "published file `{}` is absent immediately after publication",
                    destination.join(&name).display()
                )));
            }
        }
        // Directory sync is best-effort: some platforms do not support fsync
        // on directory handles.
        if let Ok(handle) = destination.dir.try_clone() {
            let _ = handle.into_std_file().sync_all();
        }
        // A post-publication fault still crosses the same durability attempt
        // as success. Callers may recover by re-reading the exact bytes; they
        // must not classify a branch that skipped the directory sync the
        // ordinary success path performs as equivalent to that success.
        if let Some(injected) = injected_post_publication_failure(relative) {
            return Err(possibly(injected));
        }
        Ok(Published {
            created_directories: created,
        })
    }

    /// Judge a whole declared set before anything is spent: every path on its
    /// own, and then the set against itself.
    ///
    /// Two declared rows that *lexically* differ can still be one file — a
    /// case-folding volume, a hard link, a junction one level up. Where both
    /// paths already exist this proves it now, from the file identity the OS
    /// reports, instead of waiting for a post-write canonicalisation that only
    /// notices after one row has already overwritten the other.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub fn preflight_set(&self, relatives: &[String]) -> Result<()> {
        self.preflight_set_against(relatives, &[])
    }

    /// The same judgement, widened to a set of **existing** paths the declared
    /// outputs must also not turn out to be.
    ///
    /// The portable identity key is the free first gate and it stays: it
    /// refuses `Docs/A.md` against `docs/a.md` with no syscall at all. What it
    /// cannot model is an alias the *host* invents — a Win32 8.3 short
    /// spelling, a Unix bind mount, a case-insensitive volume mounted inside a
    /// case-sensitive one, a filesystem alias that does not exist yet. Only the
    /// OS knows those, so where both paths exist the OS is asked, before the
    /// caller spends anything.
    ///
    /// `prior` rows are compared, never validated as destinations: they already
    /// exist and are somebody else's output. They are still opened through this
    /// project's capability and no-follow, so a `prior` row cannot be used to
    /// reach outside the project — a caller must not pass one that is.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub fn preflight_set_against(&self, relatives: &[String], prior: &[String]) -> Result<()> {
        let mut earlier_prior: Vec<(FileIdentity, &str)> = Vec::new();
        for relative in prior {
            if let Some(identity) = self.comparable_identity(relative)? {
                earlier_prior.push((identity, relative));
            }
        }
        let mut seen: Vec<(FileIdentity, &str)> = Vec::new();
        for relative in relatives {
            self.preflight(relative)?;
            let Some(identity) = self.file_identity(relative)? else {
                continue;
            };
            if let Some((_, earlier)) = seen.iter().find(|(known, _)| *known == identity) {
                bail!(
                    "declared outputs `{earlier}` and `{relative}` are the same physical \
                     file on this filesystem; each declared output is one distinct file \
                     written exactly once"
                );
            }
            if let Some((_, earlier)) = earlier_prior.iter().find(|(known, _)| *known == identity) {
                bail!(
                    "declared output `{relative}` is the same physical file as `{earlier}`, \
                     which an earlier phase already produced; writing it would destroy that \
                     artifact under a different name"
                );
            }
            seen.push((identity, relative));
        }
        Ok(())
    }

    /// The identity of an **already existing** row that is being compared
    /// rather than written: `None` when it has none this project can read.
    ///
    /// A prior artifact is not a destination. It may be a directory, a device,
    /// something whose permissions this process does not hold — none of which
    /// is a reason to refuse the run, and Windows reports several of them as a
    /// hard `Access is denied` rather than as "not a file". Skipping such a row
    /// costs nothing, because the alias it could hide is caught anyway: a
    /// declared output must itself pass [`preflight`](Self::preflight), which
    /// refuses anything that is not an ordinary replaceable file, and the OS
    /// does not report one identity for an openable regular file and an
    /// unopenable non-file.
    ///
    /// A malformed relative path still propagates: that is a caller error, not
    /// a filesystem state.
    fn comparable_identity(&self, relative: &str) -> Result<Option<FileIdentity>> {
        let root = self.root_dir()?;
        let Some((holder, name)) = self.holder_of(&root, relative)? else {
            return Ok(None);
        };
        let mut options = cap_options();
        let Ok(file) = holder.dir.open_with(&name, options.read(true)) else {
            return Ok(None);
        };
        let identity = file_identity(&file.into_std(), &holder.join(&name))?;
        Ok(Some(identity::with_alias(identity, relative)))
    }

    /// The OS's own identity for a path that exists, or `None` when it does
    /// not. Never follows a link.
    fn file_identity(&self, relative: &str) -> Result<Option<FileIdentity>> {
        let root = self.root_dir()?;
        let Some((holder, name)) = self.holder_of(&root, relative)? else {
            return Ok(None);
        };
        let mut options = cap_options();
        match holder.dir.open_with(&name, options.read(true)) {
            Ok(file) => {
                let identity = file_identity(&file.into_std(), &holder.join(&name))?;
                Ok(Some(identity::with_alias(identity, relative)))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(anyhow::Error::new(error)
                .context(format!("inspecting `{}`", holder.join(&name).display()))),
        }
    }

    /// Judge a project-relative path without creating anything, mutating
    /// anything, or reading a credential — the preflight a paid caller runs
    /// before it spends. Every ancestor that *currently exists* and the final
    /// path itself are opened no-follow: a link, reparse point, junction, a
    /// non-directory ancestor, a directory or device where a file is declared,
    /// or a hard-linked final file refuses here. Missing ancestors are legal;
    /// the mutation path rechecks them, because only that check sees a race.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub fn preflight(&self, relative: &str) -> Result<()> {
        let (parents, name) = split_relative(relative)?;
        let mut current = self.root_dir()?;
        for component in &parents {
            match current.open_child_checked(component) {
                Ok(Some(child)) => current = child,
                // A missing ancestor is not a problem: it will be created, and
                // the creation itself is no-follow.
                Ok(None) => return Ok(()),
                Err(error) => {
                    return Err(error.context(format!(
                        "the declared output `{relative}` cannot use `{}` as a link-free \
                         directory",
                        current.join(component).display()
                    )));
                }
            }
        }
        let mut options = cap_options();
        match current.dir.open_with(&name, options.read(true)) {
            Ok(file) => verify_regular_single_link(&file.into_std(), &current.join(&name)).context(
                format!("the declared output `{relative}` is not a replaceable regular file"),
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(anyhow::Error::new(error).context(format!(
                "the declared output `{relative}` cannot be opened without following a link"
            ))),
        }
    }

    /// Read one file at a project-relative path, or `None` when absent.
    pub fn read_file(&self, relative: &str) -> Result<Option<Vec<u8>>> {
        let root = self.root_dir()?;
        self.read_file_in(&root, relative)
    }

    /// Read one file at a relative path below `directory`, or `None` when
    /// absent. A link, a non-regular file, or a hard link count != 1 refuses.
    pub fn read_file_in(&self, directory: &Pinned, relative: &str) -> Result<Option<Vec<u8>>> {
        let Some((holder, name)) = self.holder_of(directory, relative)? else {
            return Ok(None);
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

    /// Judge one project-relative path without reading it: the credential-free
    /// probe a freshness check needs. An unsafe component is [`Presence::Unusable`]
    /// rather than an error, because a probe answers "may this be reused", never
    /// "is this legal to declare" — that question was already answered.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
    pub fn probe_regular_nonempty(&self, relative: &str) -> Presence {
        let Ok(root) = self.root_dir() else {
            return Presence::Unusable;
        };
        let Ok(Some((holder, name))) = self.holder_of(&root, relative) else {
            return Presence::Absent;
        };
        let mut options = cap_options();
        match holder.dir.open_with(&name, options.read(true)) {
            Ok(file) => {
                let std_file = file.into_std();
                if verify_regular_single_link(&std_file, &holder.join(&name)).is_err() {
                    return Presence::Unusable;
                }
                match std_file.metadata() {
                    Ok(metadata) if metadata.len() > 0 => Presence::RegularNonEmpty,
                    Ok(_) => Presence::Unusable,
                    Err(_) => Presence::Unusable,
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Presence::Absent,
            Err(_) => Presence::Unusable,
        }
    }

    /// Remove one file at a relative path below `directory`; `Ok(false)` when
    /// absent. Links and non-regular files refuse.
    pub fn remove_file_in(&self, directory: &Pinned, relative: &str) -> Result<bool> {
        let Some((holder, name)) = self.holder_of(directory, relative)? else {
            return Ok(false);
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

    /// Resolve the pinned directory that should hold `relative`'s final
    /// component, or `None` when an ancestor is absent.
    fn holder_of(&self, directory: &Pinned, relative: &str) -> Result<Option<(Pinned, String)>> {
        let (parents, name) = split_relative(relative)?;
        if parents.is_empty() {
            return Ok(Some((directory.shallow_clone()?, name)));
        }
        let chain = parents.iter().map(String::as_str).collect::<Vec<_>>();
        match descend(directory, &chain) {
            Ok(Some(holder)) => Ok(Some((holder, name))),
            Ok(None) => Ok(None),
            Err(error) => Err(anyhow::Error::new(error).context(format!(
                "opening no-follow directory below `{}`",
                directory.path().display()
            ))),
        }
    }
}

/// Refuse to publish over anything that is not an ordinary replaceable file.
fn refuse_unpublishable_destination(destination: &Pinned, name: &str) -> Result<()> {
    let mut options = cap_options();
    match destination.dir.open_with(name, options.read(true)) {
        Ok(file) => {
            let std_file = file.into_std();
            verify_regular_single_link(&std_file, &destination.join(name))
        }
        // `NotFound` is the ordinary create case. A no-follow open of an
        // existing link answers with a link-ish error rather than the target,
        // which is exactly the refusal we want.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(anyhow::Error::new(error).context(format!(
            "the declared output `{}` cannot be opened without following a link",
            destination.join(name).display()
        ))),
    }
}

/// Claim a staging name we provably own. `create_new` fails on any existing
/// entry, so a leftover or a neighbour's stage is stepped over, never reused
/// and never removed.
fn create_unique_stage(destination: &Pinned) -> Result<(String, cap_std::fs::File)> {
    let pid = std::process::id();
    for attempt in 0..STAGE_ATTEMPTS {
        let name = format!("{STAGE_PREFIX}{pid}-{attempt}");
        let mut options = cap_options();
        match destination
            .dir
            .open_with(&name, options.read(true).write(true).create_new(true))
        {
            Ok(file) => return Ok((name, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(anyhow::Error::new(error)
                    .context(format!("staging beside `{}`", destination.path().display())));
            }
        }
    }
    bail!(
        "could not claim an unowned staging name in `{}` after {STAGE_ATTEMPTS} attempts",
        destination.path().display()
    )
}

/// Injection point for a failure that happens **after** a successful rename,
/// so the possibly-published branch has a deterministic counterexample instead
/// of one that depends on winning a race. Compiled out entirely unless the
/// `inject-failures` feature is on, and it reads no environment: a gated crate
/// that grew an ambient-env read for a test would be trading one honesty for
/// another.
#[cfg(any(test, feature = "inject-failures"))]
mod inject {
    use std::cell::RefCell;

    // Thread-local, not global: the test harness runs one test per thread, so
    // a process-wide switch would arm every concurrently running test instead
    // of the one that asked for it.
    thread_local! {
        static AFTER_PUBLISH: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    /// Make the next publication of `relative` **on this thread** fail after
    /// its rename. Pass `None` to disarm.
    pub fn fail_after_publish(relative: Option<&str>) {
        AFTER_PUBLISH.with(|armed| *armed.borrow_mut() = relative.map(str::to_string));
    }

    pub(super) fn armed_for(name: &str) -> Option<anyhow::Error> {
        AFTER_PUBLISH.with(|armed| {
            armed
                .borrow()
                .as_deref()
                .filter(|target| *target == name)
                .map(|_| anyhow::anyhow!("injected post-publication failure for `{name}`"))
        })
    }
}

#[cfg(any(test, feature = "inject-failures"))]
pub use inject::fail_after_publish;

#[cfg(any(test, feature = "inject-failures"))]
fn injected_post_publication_failure(name: &str) -> Option<anyhow::Error> {
    inject::armed_for(name)
}

#[cfg(not(any(test, feature = "inject-failures")))]
fn injected_post_publication_failure(_name: &str) -> Option<anyhow::Error> {
    None
}

pub(crate) fn cap_options() -> cap_std::fs::OpenOptions {
    let mut options = cap_std::fs::OpenOptions::new();
    options.follow(FollowSymlinks::No);
    options
}

/// Refuse anything but a regular single-link file: no directories, no devices,
/// no symlink/junction objects, no hard links shared with another name.
pub(crate) fn verify_regular_single_link(file: &std::fs::File, display: &Path) -> Result<()> {
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

#[cfg(test)]
#[path = "file/bounded_tests.rs"]
mod bounded_tests;

#[cfg(test)]
#[path = "file/identity_tests.rs"]
mod identity_tests;

#[cfg(test)]
#[path = "file/publish_verify_tests.rs"]
mod publish_verify_tests;
