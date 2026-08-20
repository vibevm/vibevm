//! B-072's measuring half — byte-comparison of two projected catalogs.
//!
//! Split from `memory.rs` along its one clean seam (the file hit the
//! 600-line budget): [`Index::write_to`]'s idempotence gate projects
//! into a scratch directory and asks THIS module whether the disk
//! already holds exactly that projection. Pure filesystem comparison —
//! no `Index` state, no clock.
//!
//! [`Index::write_to`]: super::Index::write_to

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity");

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::index::{by_name, inverted, primary, repomd};

/// B-072 — whether two data directories hold the SAME projection,
/// byte for byte, over exactly the file set [`Index::project`]
/// writes: the four root files and the three projection directories.
/// The `state/` journal and any `.git` working copy are deliberately
/// outside the set — they are truth and transport, not projection,
/// and the journal legitimately grows on a mutation that changes no
/// catalog byte. A projection directory missing on one side counts as
/// an empty set: the projected CONTENT is equal either way.
///
/// [`Index::project`]: super::Index
pub(super) fn projection_matches(scratch: &Path, disk: &Path) -> Result<bool> {
    const FILES: [&str; 4] = [
        repomd::FILENAME,
        "hello.json",
        primary::FILENAME,
        primary::FILENAME_GZ,
    ];
    const DIRS: [&str; 3] = [
        by_name::DIRNAME,
        inverted::BY_CAP_DIRNAME,
        inverted::BY_PURL_DIRNAME,
    ];
    for name in FILES {
        let fresh = std::fs::read(scratch.join(name));
        let old = std::fs::read(disk.join(name));
        match (fresh, old) {
            (Ok(fresh), Ok(old)) if fresh == old => {}
            _ => return Ok(false),
        }
    }
    for name in DIRS {
        let fresh = rel_files(&scratch.join(name))?;
        let old = rel_files(&disk.join(name))?;
        if fresh != old {
            return Ok(false);
        }
        for rel in fresh {
            let left = scratch.join(name).join(&rel);
            let right = disk.join(name).join(&rel);
            if read_bytes(&left)? != read_bytes(&right)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// `std::fs::read` in the crate's error vocabulary — the comparator's
/// twin reads must land in `Error::Io` like every other fs call here.
fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| Error::Io {
        path: path.to_path_buf(),
        message: e.to_string(),
    })
}

/// Every file under `dir`, as sorted relative paths. A missing
/// directory is the empty set (see [`projection_matches`]).
fn rel_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current).map_err(|e| Error::Io {
            path: current.clone(),
            message: e.to_string(),
        })? {
            let entry = entry.map_err(|e| Error::Io {
                path: current.clone(),
                message: e.to_string(),
            })?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(
                    path.strip_prefix(dir)
                        .map_err(|e| Error::Io {
                            path: path.clone(),
                            message: e.to_string(),
                        })?
                        .to_path_buf(),
                );
            }
        }
    }
    out.sort();
    Ok(out)
}
