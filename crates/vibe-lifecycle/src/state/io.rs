//! The capability-relative lifecycle-state I/O cell, split from `store.rs`
//! when the transaction half outgrew the 600-line budget.
//!
//! Every production byte of `.vibe/lifecycle.toml` moves through the shared
//! `vibe-safefs` project capability rooted at the canonical workspace root:
//! reads are no-follow, regular-file, single-link and bounded by
//! [`STATE_CAP`] before allocation and again at `cap + 1` while reading;
//! writes are the safefs staged atomic replace. There is no ambient
//! `read_to_string`, no `File::create`, no ambient rename, no `create_dir_all`
//! and no second path validator here — the capability cell owns the path law,
//! and this module owns only the state file's own typing.

use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_safefs::Project;
use vibe_wire::generated::lifecycle_state::LifecycleState;

use super::error::LifecycleStateError;
use super::store::LifecycleStateStore;
use super::validate::validate_state;

const SCHEMA: u32 = 1;

/// The bounded-read ceiling for `.vibe/lifecycle.toml` (PROP-054
/// `##PHASE-STATE-HOME`, R7.4 §2.2): 8 MiB. A lifecycle state is freshness
/// rows, not a stream — a state larger than this is a corrupted or hostile
/// file, and the refusal names the real length rather than allocating toward
/// it or parsing a prefix.
pub(crate) const STATE_CAP: usize = 8 * 1024 * 1024;

/// The prior state exactly as it sits on disk: the parsed generated value AND
/// the raw bounded bytes it came from. A live store keeps both, because the
/// post-publication recovery compares DISK BYTES against the exact bytes it
/// believed durable — never a reserialization guessed to be the prior file.
#[derive(Debug)]
pub(crate) struct PriorState {
    pub(crate) bytes: Vec<u8>,
    pub(crate) state: LifecycleState,
}

/// Open the pinned workspace-root capability the state file lives under.
/// `Project::open` is the safefs cell's single ambient-authority open. A root
/// that cannot be pinned — relative, missing, not a directory, unopenable —
/// is a ROOT problem, not a state-file problem: it refuses as the typed
/// [`LifecycleStateError::Root`] with a remedy naming the root, never the
/// erasable-cache remedy that would advise deleting a healthy state file.
pub(crate) fn open_project(
    root: &Path,
    _state_path: &Path,
) -> Result<Project, LifecycleStateError> {
    Project::open(root).map_err(|error| LifecycleStateError::Root {
        path: root.to_path_buf(),
        reason: format!("{error:#}"),
    })
}

/// Read and decode the prior state through the pinned capability. `Ok(None)`
/// only for genuine absence — no file and no `.vibe` ancestor. Every other
/// outcome — unsafe shape (link, hard link, directory), over-cap, non-UTF8,
/// TOML-malformed, unsupported schema, invariant-violating — is the
/// erasable-cache refusal; none is followed, partially read or silently
/// replaced.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME")]
pub(crate) fn read_prior(
    project: &Project,
    path: &Path,
) -> Result<Option<PriorState>, LifecycleStateError> {
    let Some(bytes) = read_state_bytes(project, path)? else {
        return Ok(None);
    };
    let state = decode(&bytes, path)?;
    Ok(Some(PriorState { bytes, state }))
}

/// Read the exact bounded state bytes without decoding them.
///
/// The optimistic hosted-task reader must retain the first byte string even
/// when semantic decoding fails, then compare it with a second safe read
/// before surfacing that failure. The mutating store continues through
/// [`read_prior`]; this split changes no store behavior and keeps the one
/// state filename/cap/error mapping in this I/O cell.
pub(crate) fn read_state_bytes(
    project: &Project,
    path: &Path,
) -> Result<Option<Vec<u8>>, LifecycleStateError> {
    project
        .read_file_bounded(LifecycleStateStore::FILE, STATE_CAP)
        .map_err(|error| LifecycleStateError::Read {
            path: path.to_path_buf(),
            source: std::io::Error::other(format!("{error:#}")),
        })
}

/// The same read for a caller that holds only the workspace root — the
/// lock-free read-only surfaces (`peek`, and any future reader that must not
/// create lock state) rather than a live store. Identity selection and the
/// leased `peek_with_lease` do NOT come through here: both hold a lease and
/// read through its pinned capability, never a second `Project::open`.
pub(crate) fn read_prior_state(root: &Path) -> Result<Option<LifecycleState>, LifecycleStateError> {
    let path = state_path(root);
    let project = open_project(root, &path)?;
    Ok(read_prior(&project, &path)?.map(|prior| prior.state))
}

/// Decode exact bounded bytes into a semantically valid state: UTF-8, then
/// the generated type, then the schema gate, then the lifecycle invariants.
/// Parsing happens only on bytes that already passed the bounded read, so a
/// refusal here is never a partial parse of an over-cap file.
pub(crate) fn decode(bytes: &[u8], path: &Path) -> Result<LifecycleState, LifecycleStateError> {
    let text = std::str::from_utf8(bytes).map_err(|_| LifecycleStateError::NotUtf8 {
        path: path.to_path_buf(),
    })?;
    let state: LifecycleState =
        toml::from_str(text).map_err(|error| LifecycleStateError::Malformed {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
    if state.schema != SCHEMA {
        return Err(LifecycleStateError::Unsupported {
            path: path.to_path_buf(),
            schema: state.schema,
        });
    }
    validate_state(&state).map_err(|reason| LifecycleStateError::Invariant {
        path: path.to_path_buf(),
        reason,
    })?;
    Ok(state)
}

/// Validate and encode a CANDIDATE state into the exact bytes that would be
/// published. Validation, encoding and the size gate all precede any
/// publication attempt, so an invariant refusal, an encode failure or an
/// over-cap candidate is provably invisible on disk — no stage is created,
/// and the prior bytes stay exactly current.
pub(crate) fn encode(
    candidate: &LifecycleState,
    path: &Path,
) -> Result<Vec<u8>, LifecycleStateError> {
    validate_state(candidate).map_err(|reason| LifecycleStateError::Invariant {
        path: path.to_path_buf(),
        reason,
    })?;
    let bytes = toml::to_string_pretty(candidate)
        .map(|text| text.into_bytes())
        .map_err(|error| LifecycleStateError::Encode {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
    // The read side of this same cell refuses anything over STATE_CAP, so a
    // candidate larger than the cap is a state this store could never read
    // back — its own next `begin`/`peek` would treat it as hostile. The gate
    // sits here, before publication, rather than leaving that discovery to
    // the read that follows the write.
    if bytes.len() > STATE_CAP {
        return Err(LifecycleStateError::TooLarge {
            path: path.to_path_buf(),
            size: bytes.len(),
            cap: STATE_CAP,
        });
    }
    Ok(bytes)
}

/// One failed publication attempt: how far it got, and the original failure
/// rendered verbatim. The stage is the safefs cell's own public `Copy` enum,
/// carried directly — the stage is the whole boundary (everything before the
/// rename is provably invisible, the rename and everything after it is not),
/// so it travels with the failure into the store's transaction decision.
#[derive(Debug)]
pub(crate) struct PublicationFailure {
    pub(crate) stage: vibe_safefs::PublishStage,
    pub(crate) rendered: String,
}

impl PublicationFailure {
    /// Flatten a real safefs publication failure, preserving its typed stage
    /// and its full error chain (the safefs report names the stage itself, so
    /// the rendered text keeps that fact even when read alone).
    pub(crate) fn from_publish(error: vibe_safefs::PublishError) -> Self {
        let stage = error.stage;
        let rendered = format!("{:#}", error.into_report());
        Self { stage, rendered }
    }

    /// The deterministic test seam's synthetic post-publication failure: the
    /// publication crossed the rename boundary and then failed, and the disk
    /// is left exactly as the test arranged it. Compiled out of every shipped
    /// build alongside the seam that arms it.
    #[cfg(test)]
    pub(crate) fn synthetic_possibly(reason: String) -> Self {
        Self {
            stage: vibe_safefs::PublishStage::PossiblyPublished,
            rendered: format!(
                "{reason} (injected after the rename was attempted; the destination may already \
                 hold the new bytes)"
            ),
        }
    }
}

/// The display path of the state file under `root`, for diagnostics only —
/// never a path production I/O then opens with ambient authority.
pub(crate) fn state_path(root: &Path) -> PathBuf {
    root.join(super::store::LifecycleStateStore::FILE)
}
