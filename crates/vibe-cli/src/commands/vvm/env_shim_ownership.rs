//! Who owns a file in the shim directory (PROP-019 §2.4, §2.21).
//!
//! `<base>/opt/bin` goes on PATH, and on a network that allows executables
//! only under one directory (§2.21) that base is the directory the operator
//! already keeps their tools in — so the shim dir is routinely **shared**
//! with files vibe never wrote. Vibe must therefore be able to tell its own
//! shims from the directory's other tenants, and it must never delete,
//! overwrite or claim one of theirs.
//!
//! Recognition has two tiers, because they authorise different things:
//!
//! * **Recorded** — the inventory below names the file and its digest still
//!   matches, so vibe wrote exactly these bytes and nothing changed them
//!   since. This is proof of authorship: the only tier that may ever be
//!   deleted.
//! * **Recognised** — no record, but the content carries a signature only a
//!   vibe shim has: the marker vibe writes, or a reference to vibe's own
//!   `../vibevm/current` pointer. That is an older vibe's shim, or an edited
//!   one; it may be rewritten in place, never deleted.
//!
//! Anything else at a shim's name is foreign and refused. The signature tier
//! is what keeps the inventory backward-compatible: a machine whose shims
//! predate the inventory keeps upgrading normally, because its shims still
//! identify themselves by content.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#layout");

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use specmark::spec;
use thiserror::Error;

use crate::commands::vvm::store::{StoreError, VersionStore, atomic_replace};

/// `<root>/vibevm/shims.toml` — the inventory, beside `state.toml` in vibe's
/// own data directory, never in the shared shim directory itself.
const SHIM_INVENTORY_FILE: &str = "shims.toml";

const SHIM_INVENTORY_SCHEMA: u32 = 1;

/// An inventory of four tiny records; anything larger is not one.
const INVENTORY_MAX_BYTES: u64 = 64 * 1024;

/// A shim is a few hundred bytes. A larger file at a shim's name belongs to
/// somebody else, whatever it holds.
const SHIM_MAX_BYTES: u64 = 64 * 1024;

/// The marker vibe's POSIX shim carries.
const SHIM_MARKER: &str = "vibevm (VVM) shim";

/// The layout reference every shim vibe has ever written carries: a foreign
/// tool does not resolve vibe's own `current` pointer.
const LAYOUT_MARKERS: [&str; 2] = ["../vibevm/current", "..\\vibevm\\current"];

/// How far vibe may go with a file that sits at one of its shim names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShimOwnership {
    /// Nothing is there: vibe may create it.
    Absent,
    /// Vibe wrote exactly these bytes and nothing changed them since
    /// (inventory digest match) — the only tier a removal may delete.
    Recorded,
    /// Vibe's own shim by content, from another version or hand-edited: it
    /// may be rewritten, never deleted.
    Recognised,
    /// A file vibe cannot claim.
    Foreign,
}

impl ShimOwnership {
    /// Whether vibe may replace the file in place. Deletion needs the
    /// stricter [`ShimOwnership::Recorded`]; no command removes a shim
    /// today, and one that ever does must require exactly that tier.
    pub(super) fn is_writable(self) -> bool {
        !matches!(self, ShimOwnership::Foreign)
    }
}

/// The shim-ownership failure surface (PROP-019 §2.4): a shim directory that
/// already holds somebody else's file at a name vibe publishes, and the
/// inventory's own IO.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#layout")]
pub(crate) enum ShimOwnershipError {
    #[error(
        "`{path}` is not a file vibe wrote, and `{name}` is the name of a vibe shim in `{dir}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: remove or rename `{path}` yourself, or install under a directory of vibe's own \
          (`VIBEVM_INSTALL_ROOT`, or `[self] install_root` in the site profile), then re-run)"
    )]
    Foreign {
        dir: PathBuf,
        name: String,
        path: PathBuf,
    },

    #[error(
        "reading the shim directory `{path}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: ensure the shim directory is readable by this user)"
    )]
    ReadShimDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "reading the shim inventory `{path}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: ensure the install root is readable, or delete the file to forget the inventory)"
    )]
    ReadInventory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "writing the shim inventory `{path}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: ensure the install root is writable)"
    )]
    WriteInventory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "serialising the shim inventory failed: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: report this — the in-memory inventory is malformed)"
    )]
    SerialiseInventory { detail: String },

    #[error(transparent)]
    Store(#[from] StoreError),
}

/// One shim vibe wrote, by the name it wrote it under.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ShimRecord {
    name: String,
    sha256: String,
}

/// What vibe wrote into the shim directory.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct ShimInventory {
    schema: u32,
    vibe_version: String,
    #[serde(default, rename = "shim")]
    shims: Vec<ShimRecord>,
}

impl ShimInventory {
    fn empty() -> Self {
        ShimInventory {
            schema: SHIM_INVENTORY_SCHEMA,
            vibe_version: env!("CARGO_PKG_VERSION").to_owned(),
            shims: Vec::new(),
        }
    }

    fn recorded_digest(&self, name: &str) -> Option<&str> {
        self.shims
            .iter()
            .find(|record| same_name(&record.name, name))
            .map(|record| record.sha256.as_str())
    }

    /// How far vibe may go with the file at `path`, which the filesystem
    /// resolves as the shim name `name`.
    pub(super) fn classify(&self, name: &str, path: &Path) -> ShimOwnership {
        let bytes = match read_bounded(path, SHIM_MAX_BYTES) {
            Ok(None) => return ShimOwnership::Absent,
            Ok(Some(bytes)) => bytes,
            // A directory, a link, a device, an oversized file or one this
            // user cannot read is not a shim vibe wrote.
            Err(_) => return ShimOwnership::Foreign,
        };
        if self
            .recorded_digest(name)
            .is_some_and(|recorded| recorded == digest_of(&bytes))
        {
            return ShimOwnership::Recorded;
        }
        if carries_signature(&bytes) {
            ShimOwnership::Recognised
        } else {
            ShimOwnership::Foreign
        }
    }

    /// Refuse the write unless every file the filesystem resolves as `name`
    /// in `dir` is vibe's own. Called for every planned shim before the
    /// first one is written, so a refusal leaves the directory untouched.
    pub(super) fn refuse_foreign(&self, dir: &Path, name: &str) -> Result<(), ShimOwnershipError> {
        for path in resolving_entries(dir, name, case_insensitive_names())? {
            if !self.classify(name, &path).is_writable() {
                return Err(ShimOwnershipError::Foreign {
                    dir: dir.to_path_buf(),
                    name: name.to_owned(),
                    path,
                });
            }
        }
        Ok(())
    }

    /// Record the bytes vibe has just written as `name`.
    pub(super) fn record(&mut self, name: &str, bytes: &[u8]) {
        let sha256 = digest_of(bytes);
        match self
            .shims
            .iter_mut()
            .find(|record| same_name(&record.name, name))
        {
            Some(record) => {
                record.name = name.to_owned();
                record.sha256 = sha256;
            }
            None => self.shims.push(ShimRecord {
                name: name.to_owned(),
                sha256,
            }),
        }
        self.vibe_version = env!("CARGO_PKG_VERSION").to_owned();
        self.shims.sort_by(|left, right| left.name.cmp(&right.name));
    }
}

/// Read the inventory, defaulting to empty on a machine that has none.
///
/// A damaged inventory claims nothing: its shims are then recognised by
/// their content alone, which is enough to rewrite one but never to delete
/// one. That is also what a pre-inventory installation looks like.
pub(super) fn load(store: &VersionStore) -> Result<ShimInventory, ShimOwnershipError> {
    let path = inventory_path(store);
    let bytes = read_bounded(&path, INVENTORY_MAX_BYTES).map_err(|source| {
        ShimOwnershipError::ReadInventory {
            path: path.clone(),
            source,
        }
    })?;
    let Some(bytes) = bytes else {
        return Ok(ShimInventory::empty());
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return Ok(ShimInventory::empty());
    };
    Ok(toml::from_str(&text).unwrap_or_else(|_| ShimInventory::empty()))
}

/// Store the inventory atomically in vibe's own data directory.
pub(super) fn save(
    store: &VersionStore,
    inventory: &ShimInventory,
) -> Result<(), ShimOwnershipError> {
    let dir = store.data_dir();
    store.guard_mutation_path(&dir)?;
    fs::create_dir_all(&dir).map_err(|source| ShimOwnershipError::WriteInventory {
        path: dir.clone(),
        source,
    })?;
    let path = inventory_path(store);
    store.guard_mutation_path(&path)?;
    let text =
        toml::to_string(inventory).map_err(|error| ShimOwnershipError::SerialiseInventory {
            detail: error.to_string(),
        })?;
    atomic_replace(&path, text.as_bytes())
        .map_err(|source| ShimOwnershipError::WriteInventory { path, source })?;
    Ok(())
}

fn inventory_path(store: &VersionStore) -> PathBuf {
    store.data_dir().join(SHIM_INVENTORY_FILE)
}

/// Whether the filesystem under the shim directory treats `vibe.cmd` and
/// `VIBE.CMD` as one file. Windows does, so a write to `vibe.cmd` there
/// replaces whichever spelling is already on disk, and every spelling has to
/// be classified before the write.
fn case_insensitive_names() -> bool {
    cfg!(windows)
}

fn same_name(left: &str, right: &str) -> bool {
    if case_insensitive_names() {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

/// Every existing entry of `dir` that the filesystem resolves as `name` —
/// `name` itself, plus, where names are case-insensitive, its other
/// spellings. Returns the plain `dir/name` when nothing is there, so an
/// absent shim is classified like any other.
///
/// Shim names are ASCII, so ASCII case folding decides: a name that differs
/// from one of them outside ASCII is a different name on any filesystem.
fn resolving_entries(
    dir: &Path,
    name: &str,
    case_insensitive: bool,
) -> Result<Vec<PathBuf>, ShimOwnershipError> {
    if !case_insensitive {
        return Ok(vec![dir.join(name)]);
    }
    let mut found = Vec::new();
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.map_err(|source| ShimOwnershipError::ReadShimDir {
                    path: dir.to_path_buf(),
                    source,
                })?;
                if entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(name)
                {
                    found.push(entry.path());
                }
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(ShimOwnershipError::ReadShimDir {
                path: dir.to_path_buf(),
                source,
            });
        }
    }
    if found.is_empty() {
        found.push(dir.join(name));
    }
    found.sort();
    Ok(found)
}

/// Read a small regular file whole, refusing links, directories, special
/// files and anything over `maximum`; `None` when it is not there.
fn read_bounded(path: &Path, maximum: u64) -> io::Result<Option<Vec<u8>>> {
    let (mut file, metadata) = match crate::commands::vvm::store::open_regular_no_follow(path) {
        Ok(opened) => opened,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if metadata.len() > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("`{}` exceeds its {maximum}-byte bound", path.display()),
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes)?;
    Ok(Some(bytes))
}

fn digest_of(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Whether the content identifies itself as one of vibe's shims.
fn carries_signature(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    text.contains(SHIM_MARKER) || LAYOUT_MARKERS.iter().any(|marker| text.contains(marker))
}

#[cfg(test)]
#[path = "env_shim_ownership_tests.rs"]
mod tests;
