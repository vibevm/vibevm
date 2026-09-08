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

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NO-FOLLOW");

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, bail};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use sha2::Digest as _;
use specmark::spec;

use crate::component::{STAGE_PREFIX, split_relative};
use crate::project::{Pinned, Project, descend};
use crate::publish::{PublishError, Published};

mod bounded;
mod create_new;
pub(crate) mod identity;
mod stable;
mod stream;
pub use bounded::StableFileSnapshot;
#[cfg(any(test, feature = "inject-failures"))]
pub use create_new::{fail_before_publish, fail_before_stage_cleanup};
pub use identity::FileIdentity;
pub(crate) use identity::is_not_empty;
use identity::{file_identity, number_of_links};
pub use stable::StableFileState;
pub(crate) use stable::unix_mode;
pub use stream::ContentDigest;

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

include!("file/project_methods.rs");

impl Project {
    project_file_methods!();
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

fn validate_transaction_stage_name(name: &str) -> Result<()> {
    let Some(suffix) = name.strip_prefix(".vibe-stage-tx-") else {
        anyhow::bail!("transaction stage must use the reserved .vibe-stage-tx- prefix");
    };
    if suffix.len() != 32
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        anyhow::bail!("transaction stage suffix must be 32 lowercase hex characters");
    }
    Ok(())
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
#[path = "file/stream_tests.rs"]
mod stream_tests;

#[cfg(test)]
#[path = "file/stream_shape_tests.rs"]
mod stream_shape_tests;

#[cfg(test)]
#[path = "file/identity_tests.rs"]
mod identity_tests;

#[cfg(test)]
#[path = "file/publish_verify_tests.rs"]
mod publish_verify_tests;

#[cfg(test)]
#[path = "file/stable_tests.rs"]
mod stable_tests;
