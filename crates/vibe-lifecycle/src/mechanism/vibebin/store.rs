//! The content-addressed payload store and the one publish primitive every
//! vibe-bin write goes through.
//!
//! §7.1.0 ruling 4 in code: **a CAS payload is write-once and idempotent to
//! re-write**, which is what makes an apply §7.2-recoverable for free. A
//! payload already present at its own address is a checkpointed no-op, not
//! an overwrite and not a refusal — a second generation that deploys bytes
//! the store already holds must cost nothing and must not disturb what an
//! earlier generation still names.
//!
//! The one case that is NOT benign is a store entry whose bytes are not the
//! bytes its address claims. A content-addressed name is a promise about
//! content; when it is broken this cell refuses by name rather than
//! repairing silently, because silently rewriting it would erase whatever a
//! prior generation's pointer is still resolving to.
//!
//! Every write is staged and renamed. The engine offers the staging
//! directory because the descriptor declares `atomic_replacement` (§7.2:
//! "staging where the destination supports atomic replacement"); this cell
//! never chooses it and never invents one.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::path::{Path, PathBuf};

use super::launcher::LauncherFlavour;
use crate::mechanism::contain::{FileFault, digest_file, prove_regular_file};
use crate::mechanism::error::DeployProviderError;

/// The settings-relative directory holding the immutable payloads.
pub(crate) const STORE_DIR: &str = "store";

/// The settings-relative directory holding the launchers and pointers.
pub(crate) const BIN_DIR: &str = "bin";

/// What placing one CAS payload turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PayloadPlacement {
    /// The store did not hold these bytes, and now it does.
    Written,
    /// The store already held exactly these bytes — the write-once no-op
    /// that makes apply and recover the same operation.
    AlreadyPresent,
}

impl PayloadPlacement {
    /// The word this placement's evidence line spells.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Written => "written",
            Self::AlreadyPresent => "already present",
        }
    }
}

/// One payload's settings-relative, forward-slashed identity.
///
/// It is deliberately NOT a receipt resource (§7.1.0 ruling 4): naming it
/// here is for the checkpoint ledger and the evidence line, both of which
/// describe an operation rather than an ownership.
pub(crate) fn payload_relative(flavour: LauncherFlavour, digest: &str) -> String {
    format!("{STORE_DIR}/{digest}{}", flavour.payload_suffix())
}

/// Place one artifact's bytes at their own address in the store.
pub(crate) fn place_payload(
    target: &str,
    settings_root: &Path,
    staging: Option<&Path>,
    flavour: LauncherFlavour,
    artifact_path: &Path,
    artifact_digest: &str,
) -> Result<PayloadPlacement, DeployProviderError> {
    let relative = payload_relative(flavour, artifact_digest);
    let destination = join(settings_root, &relative);
    match digest_file(&destination) {
        Ok((found, _)) if found == artifact_digest => return Ok(PayloadPlacement::AlreadyPresent),
        Ok((found, _)) => {
            return Err(DeployProviderError::PayloadCorrupt {
                target: target.to_owned(),
                path: relative,
                recorded: artifact_digest.to_owned(),
                found,
            });
        }
        Err(FileFault::Missing(_)) => {}
        Err(fault) => {
            return Err(DeployProviderError::Write {
                target: target.to_owned(),
                path: relative,
                reason: fault.reason(),
            });
        }
    }
    let staged = stage_path(target, staging, &relative)?;
    copy_into(target, &relative, artifact_path, &staged)?;
    // Prove the staged copy is really the bytes the address claims BEFORE
    // it is published: a truncated copy that entered the store under a
    // correct-looking name would be a lie every later generation believes.
    let (staged_digest, _) = digest_file(&staged).map_err(|fault| DeployProviderError::Write {
        target: target.to_owned(),
        path: relative.clone(),
        reason: fault.reason(),
    })?;
    if staged_digest != artifact_digest {
        return Err(DeployProviderError::PayloadCorrupt {
            target: target.to_owned(),
            path: relative,
            recorded: artifact_digest.to_owned(),
            found: staged_digest,
        });
    }
    if flavour.needs_executable_bit() {
        make_executable(target, &relative, &staged)?;
    }
    publish(target, &relative, &staged, &destination)?;
    Ok(PayloadPlacement::Written)
}

/// Write one owned resource's exact bytes.
///
/// The launcher and the pointer are both published through here. When the
/// engine offered a staging directory the bytes are staged and renamed, so
/// a reader sees the old content or the new one and never a half-written
/// file. When it did NOT — the `remove` path, where §7.2's staging
/// sentence does not apply and the engine hands `None` rather than a
/// directory the provider would have to promise not to use — the write is
/// direct. A provider that answered `None` by minting a scratch path of
/// its own would be taking a decision §3.2 gives the engine.
pub(crate) fn place_resource(
    target: &str,
    settings_root: &Path,
    staging: Option<&Path>,
    relative: &str,
    bytes: &[u8],
    executable: bool,
) -> Result<(), DeployProviderError> {
    let destination = join(settings_root, relative);
    let refuse = |error: std::io::Error| DeployProviderError::Write {
        target: target.to_owned(),
        path: relative.to_owned(),
        reason: error.to_string(),
    };
    let Some(staging) = staging else {
        ensure_parent(target, relative, &destination)?;
        std::fs::write(&destination, bytes).map_err(refuse)?;
        if executable {
            make_executable(target, relative, &destination)?;
        }
        return Ok(());
    };
    let staged = staging.join(relative.replace('/', "_"));
    std::fs::write(&staged, bytes).map_err(refuse)?;
    if executable {
        make_executable(target, relative, &staged)?;
    }
    publish(target, relative, &staged, &destination)
}

/// Remove one owned resource; absence is success.
pub(crate) fn remove_resource(
    target: &str,
    settings_root: &Path,
    relative: &str,
) -> Result<bool, DeployProviderError> {
    let destination = join(settings_root, relative);
    match prove_regular_file(&destination) {
        Ok(_) => {}
        Err(FileFault::Missing(_)) => return Ok(false),
        Err(fault) => {
            return Err(DeployProviderError::Write {
                target: target.to_owned(),
                path: relative.to_owned(),
                reason: fault.reason(),
            });
        }
    }
    std::fs::remove_file(&destination).map_err(|error| DeployProviderError::Write {
        target: target.to_owned(),
        path: relative.to_owned(),
        reason: error.to_string(),
    })?;
    Ok(true)
}

/// One settings-relative resource's absolute path, joined component by
/// component so no separator inside the spelling can decide the result.
pub(crate) fn join(settings_root: &Path, relative: &str) -> PathBuf {
    let mut path = settings_root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
    }
    path
}

/// The staging path a payload is written at before it is published.
///
/// The payload write is the one that REQUIRES staging: it is reached only
/// from `apply` and `recover`, where the descriptor's `atomic_replacement`
/// makes the engine offer a directory. Reaching it without one is a defect
/// in this engine, and it refuses rather than minting a scratch path §3.2
/// gives the engine to choose.
fn stage_path(
    target: &str,
    staging: Option<&Path>,
    relative: &str,
) -> Result<PathBuf, DeployProviderError> {
    let Some(staging) = staging else {
        return Err(DeployProviderError::Staging {
            target: target.to_owned(),
            path: relative.to_owned(),
        });
    };
    Ok(staging.join(relative.replace('/', "_")))
}

/// Copy one proven artifact's bytes into the staging directory.
fn copy_into(
    target: &str,
    relative: &str,
    from: &Path,
    to: &Path,
) -> Result<(), DeployProviderError> {
    std::fs::copy(from, to).map_err(|error| DeployProviderError::Write {
        target: target.to_owned(),
        path: relative.to_owned(),
        reason: error.to_string(),
    })?;
    Ok(())
}

/// Rename a staged file onto its destination, creating the parent first.
fn publish(
    target: &str,
    relative: &str,
    staged: &Path,
    destination: &Path,
) -> Result<(), DeployProviderError> {
    ensure_parent(target, relative, destination)?;
    std::fs::rename(staged, destination).map_err(|error| DeployProviderError::Write {
        target: target.to_owned(),
        path: relative.to_owned(),
        reason: error.to_string(),
    })
}

/// Create one destination's parent directory.
fn ensure_parent(
    target: &str,
    relative: &str,
    destination: &Path,
) -> Result<(), DeployProviderError> {
    let Some(parent) = destination.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).map_err(|error| DeployProviderError::Write {
        target: target.to_owned(),
        path: relative.to_owned(),
        reason: error.to_string(),
    })
}

/// Give a staged file the executable bit before it is published.
#[cfg(unix)]
fn make_executable(target: &str, relative: &str, staged: &Path) -> Result<(), DeployProviderError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(staged, std::fs::Permissions::from_mode(0o755)).map_err(|error| {
        DeployProviderError::Write {
            target: target.to_owned(),
            path: relative.to_owned(),
            reason: error.to_string(),
        }
    })
}

/// Windows has no executable bit: a `.cmd` and a `.exe` are executable by
/// their names, which is the same fact the flavour already carries.
#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the unix twin is fallible; one signature, two platforms"
)]
fn make_executable(
    _target: &str,
    _relative: &str,
    _staged: &Path,
) -> Result<(), DeployProviderError> {
    Ok(())
}
