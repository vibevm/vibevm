//! One canonical root, the project identity taken from it, and the filename
//! ceiling it leaves.
//!
//! The root arrives as an absolute path a caller resolved. That is not yet an
//! IDENTITY: `…/project`, `…/project/members/..` and a symlink pointing at the
//! same tree are one project spelled three ways, and a writer that hashed the
//! spelling would call a reopened run somebody else's. So the root is
//! canonicalised exactly ONCE, at open, and that single canonical path is what
//! everything downstream uses — the digest, the `.vibe/trace/<run-id>` path,
//! the capability, and the path-length pressure the filename ceiling is
//! measured against. Nothing here ever mixes two spellings of one root.
//!
//! On Windows `canonicalize` returns the verbatim `\\?\` form. It is kept as
//! it comes: it is one consistent spelling, it is what the OS itself calls the
//! path, and re-deriving a "prettier" one would put the writer back in the
//! business of choosing between spellings. The measurement below is therefore
//! slightly conservative — a verbatim path is exempt from `MAX_PATH` — which
//! is the safe direction to be wrong in for a diagnostic file.
//!
//! The digest deliberately does NOT go through `to_string_lossy`. A path is a
//! sequence of OS units — bytes on Unix, UTF-16 code units on Windows — and a
//! lossy rendering maps distinct roots onto one string, which is exactly what
//! an identity may not do. Each platform hashes its own units under its own
//! tag, framed by its own unit COUNT (code units on Windows, bytes on Unix),
//! so neither can be confused with the other and no tag-plus-suffix can be
//! reassembled into a different root's preimage.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use vibe_wire::behaviour::compiler_trace_index::SNAPSHOT_NAME_CAP;
use vibe_wire::generated::compiler_trace_index::e1::index::ProjectIdentity;

use super::{TraceOpenError, bounded};

/// The epoch-1 spelling of `project.display`: one project per run, and the
/// outer node/unit labels live in the scopes.
const ROOT_DISPLAY: &str = ".";

/// The total path units a snapshot may cost, name included. Windows' classic
/// `MAX_PATH` is 260; the margin below it is deliberate — a diagnostic file is
/// not the thing that should discover the exact ceiling.
const PATH_UNIT_BUDGET: usize = 250;

/// Below this many units left for a filename the writer refuses to open at
/// all. The shortest canonical spelling is 31 units, so a run directory with
/// less room than this cannot publish even one snapshot, and pretending the
/// floor fits would just move the failure to every event.
const MIN_NAME_UNITS: usize = 32;

/// The domain this digest belongs to. A digest with no domain is a digest that
/// can be replayed from somewhere else.
const DIGEST_DOMAIN: &[u8] = b"vibevm/compile-trace/project-root/e1\n";

/// Resolve the supplied absolute root to the ONE canonical path this run uses
/// for everything.
///
/// A relative root is refused before the filesystem is touched. A root that
/// cannot be canonicalised — it does not exist, or is unreachable — is an open
/// failure, which means the caller compiles untraced.
pub(super) fn canonical_root(root: &Path) -> Result<PathBuf, TraceOpenError> {
    if !root.is_absolute() {
        return Err(TraceOpenError::RelativeRoot {
            root: bounded::path(root),
        });
    }
    std::fs::canonicalize(root).map_err(|error| TraceOpenError::Directory {
        reason: bounded::diagnostic(format_args!(
            "resolving the project root `{}`: {error}",
            root.display()
        )),
    })
}

/// Exactly 32 lowercase hex characters — the lifecycle run id, spelled the one
/// way the trace epoch's validator admits.
pub(super) fn checked_run_id(run_id: &str) -> Result<String, TraceOpenError> {
    if is_run_id(run_id) {
        Ok(run_id.to_string())
    } else {
        Err(TraceOpenError::RunId {
            run_id: bounded::preview(run_id),
        })
    }
}

/// Exactly 32 lowercase hex characters. Shared with retention, which inspects
/// nothing else.
pub(super) fn is_run_id(name: &str) -> bool {
    name.len() == 32
        && name
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// The project identity this run records: the epoch's `"."` display and a
/// domain-separated SHA-256 over the canonical root's own OS path units.
pub(super) fn project_identity(canonical: &Path) -> ProjectIdentity {
    let mut hasher = Sha256::new();
    hasher.update(DIGEST_DOMAIN);
    hasher.update(PLATFORM_TAG);
    // The UNIT count, not the byte-vector length: on Windows one unit is two
    // bytes, and framing the byte length there would describe a different
    // sequence than the one being hashed.
    hasher.update((unit_count(canonical) as u64).to_le_bytes());
    hasher.update(unit_bytes(canonical));
    let mut digest = String::with_capacity(7 + 64);
    digest.push_str("sha256:");
    for byte in hasher.finalize() {
        digest.push(char::from(HEX[usize::from(byte >> 4)]));
        digest.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    ProjectIdentity {
        display: ROOT_DISPLAY.to_string(),
        root_digest: digest,
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

/// How many units of filename the run directory leaves, taken together with
/// the epoch's own 96-byte cap.
///
/// Both canonical spellings are ASCII, so one byte is one UTF-16 code unit is
/// one path unit — the writer's ceiling and the epoch's cap are comparable
/// numbers rather than two different measures that happen to agree.
pub(super) fn filename_cap(run_dir: &Path) -> Result<usize, TraceOpenError> {
    let directory = unit_count(run_dir);
    // One unit for the separator between the directory and the name.
    let remaining = PATH_UNIT_BUDGET.saturating_sub(directory).saturating_sub(1);
    if remaining < MIN_NAME_UNITS {
        return Err(TraceOpenError::RunDirectoryTooDeep {
            directory_units: directory,
            remaining,
            floor: MIN_NAME_UNITS,
        });
    }
    Ok(remaining.min(SNAPSHOT_NAME_CAP))
}

#[cfg(unix)]
const PLATFORM_TAG: &[u8] = b"unix-bytes\n";

/// The lossless unit bytes of a path: on Unix the units ARE bytes.
#[cfg(unix)]
fn unit_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

/// The number of OS units — raw bytes on Unix.
#[cfg(unix)]
pub(super) fn unit_count(path: &Path) -> usize {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().len()
}

#[cfg(windows)]
const PLATFORM_TAG: &[u8] = b"windows-utf16le\n";

/// The lossless unit bytes of a path: on Windows each UTF-16 code unit is
/// serialised little-endian, so the byte vector is twice the unit count.
#[cfg(windows)]
fn unit_bytes(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;
    let mut bytes = Vec::new();
    for unit in path.as_os_str().encode_wide() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes
}

/// The number of OS units — UTF-16 CODE UNITS on Windows, never the byte
/// length of their serialisation.
#[cfg(windows)]
pub(super) fn unit_count(path: &Path) -> usize {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The framing contract: the count is UNITS and the payload is their
    /// lossless serialisation, so on Windows the payload is exactly twice the
    /// framed count and on Unix exactly equal to it.
    #[test]
    fn the_framed_count_is_units_and_the_payload_is_their_bytes() {
        let path = Path::new(if cfg!(windows) {
            r"C:\projects\demo"
        } else {
            "/projects/demo"
        });
        let units = unit_count(path);
        let bytes = unit_bytes(path);
        if cfg!(windows) {
            assert_eq!(bytes.len(), units * 2, "one code unit is two bytes");
        } else {
            assert_eq!(bytes.len(), units, "one byte is one unit");
        }
        assert_eq!(units, path.as_os_str().to_string_lossy().chars().count());
    }

    /// A non-ASCII path is where a byte count and a unit count part company,
    /// and where a lossy rendering would lose the distinction entirely.
    #[test]
    fn a_non_ascii_path_frames_units_not_bytes() {
        let path = Path::new(if cfg!(windows) {
            r"C:\проект\☃"
        } else {
            "/проект/☃"
        });
        let units = unit_count(path);
        let bytes = unit_bytes(path);
        if cfg!(windows) {
            assert_eq!(bytes.len(), units * 2);
            // 3 ASCII separators/drive + 6 Cyrillic + 1 snowman: every one of
            // them is a single UTF-16 code unit, so the count is characters.
            assert_eq!(units, path.as_os_str().to_string_lossy().chars().count());
        } else {
            assert_eq!(bytes.len(), units);
            assert!(
                units > path.as_os_str().to_string_lossy().chars().count(),
                "UTF-8 bytes outnumber characters here",
            );
        }
    }

    /// Distinct roots never share a digest, and the same root always yields
    /// the same one.
    #[test]
    fn the_digest_is_stable_and_discriminating() {
        let a = project_identity(Path::new(if cfg!(windows) { r"C:\a" } else { "/a" }));
        let b = project_identity(Path::new(if cfg!(windows) { r"C:\b" } else { "/b" }));
        assert_eq!(a.display, ".");
        assert!(a.root_digest.starts_with("sha256:"));
        assert_eq!(a.root_digest.len(), 7 + 64);
        assert_ne!(a.root_digest, b.root_digest);
        assert_eq!(
            a.root_digest,
            project_identity(Path::new(if cfg!(windows) { r"C:\a" } else { "/a" })).root_digest,
        );
    }
}
