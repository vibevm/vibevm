//! Who owns the file or directory at a slot link's name
//! (`PROP-059#path-directories`, the rules of PROP-019 §2.4 shim ownership
//! that `proxies::ownership` applies to `opt/bin`).
//!
//! A link directory lives under an install root the operator may already
//! keep their own tools in, so `vibe app` never overwrites or deletes
//! something it did not write:
//!
//! * **Recorded** — `applications/paths.toml` names the link and a link is
//!   what is there. Only this tier may be repointed or removed.
//! * **Foreign** — a real file or directory, or a link vibe has no record
//!   of: a write refuses before any change, and a removal leaves it alone.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#path-directories");

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::link::{self, Present};
use crate::error::AppError;
use crate::settings::{SettingsRoot, io};

const INVENTORY_SCHEMA: u32 = 1;
/// A record per link: far below this.
const INVENTORY_MAX_BYTES: u64 = 256 * 1024;

/// How far `vibe app` may go with what is at one link name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ownership {
    Absent,
    /// A link vibe wrote, with the target it currently has.
    Recorded(PathBuf),
    Foreign,
}

impl Ownership {
    /// Whether vibe may write this name.
    pub(crate) fn writable(&self) -> bool {
        !matches!(self, Ownership::Foreign)
    }
}

/// One link `vibe app` wrote: the slot, the index among the slot's declared
/// directories, and the target it was pointed at.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Record {
    slot: String,
    index: u32,
    target: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct File {
    schema: u32,
    #[serde(default, rename = "link")]
    links: Vec<Record>,
}

/// What `vibe app` wrote into the link directories, by `(slot, index)`.
pub(crate) struct Inventory {
    path: PathBuf,
    records: BTreeMap<(String, u32), String>,
}

/// `<home>/applications/paths.toml`, in vibe's own directory, never in a
/// link directory.
pub(crate) fn inventory_path(settings: &SettingsRoot) -> PathBuf {
    settings.applications_dir().join("paths.toml")
}

impl Inventory {
    /// Read the inventory; a missing or damaged one records nothing, so
    /// every link name reads as foreign and nothing is replaced or removed.
    pub(crate) fn load(settings: &SettingsRoot) -> Result<Self, AppError> {
        let path = inventory_path(settings);
        let records = match read_bounded(&path, INVENTORY_MAX_BYTES) {
            Ok(Some(bytes)) => String::from_utf8(bytes)
                .ok()
                .and_then(|text| toml::from_str::<File>(&text).ok())
                .map(|file| {
                    file.links
                        .into_iter()
                        .map(|record| ((record.slot, record.index), record.target))
                        .collect()
                })
                .unwrap_or_default(),
            Ok(None) => BTreeMap::new(),
            Err(source) => return Err(io("reading the link inventory", &path, source).into()),
        };
        Ok(Self { path, records })
    }

    /// Classify what is at the link of `slot`'s directory `index`.
    pub(crate) fn classify(
        &self,
        slot: &str,
        index: u32,
        path: &Path,
    ) -> Result<Ownership, AppError> {
        let present =
            link::inspect(path).map_err(|source| io("inspecting the slot link", path, source))?;
        Ok(match present {
            Present::Absent => Ownership::Absent,
            Present::Other => Ownership::Foreign,
            Present::Link(target) => {
                if self.records.contains_key(&(slot.to_owned(), index)) {
                    Ownership::Recorded(target)
                } else {
                    Ownership::Foreign
                }
            }
        })
    }

    pub(crate) fn record(&mut self, slot: &str, index: u32, target: &Path) {
        self.records.insert(
            (slot.to_owned(), index),
            target.to_string_lossy().into_owned(),
        );
    }

    pub(crate) fn forget(&mut self, slot: &str, index: u32) {
        self.records.remove(&(slot.to_owned(), index));
    }

    /// Every link recorded for `slot`, lowest index first.
    pub(crate) fn recorded_for(&self, slot: &str) -> Vec<u32> {
        self.records
            .keys()
            .filter(|(recorded, _)| recorded == slot)
            .map(|(_, index)| *index)
            .collect()
    }

    /// Every slot the inventory holds a link for.
    pub(crate) fn slots(&self) -> Vec<String> {
        let mut slots: Vec<String> = self.records.keys().map(|(slot, _)| slot.clone()).collect();
        slots.dedup();
        slots
    }

    pub(crate) fn save(&self) -> Result<(), AppError> {
        let file = File {
            schema: INVENTORY_SCHEMA,
            links: self
                .records
                .iter()
                .map(|((slot, index), target)| Record {
                    slot: slot.clone(),
                    index: *index,
                    target: target.clone(),
                })
                .collect(),
        };
        let text = toml::to_string(&file).map_err(|error| {
            io(
                "serialising the link inventory",
                &self.path,
                std::io::Error::other(error),
            )
        })?;
        crate::settings::write_atomic(&self.path, text.as_bytes())?;
        Ok(())
    }
}

/// A regular file's bytes, refusing links, directories and anything over
/// `maximum`; `None` when nothing is there.
fn read_bounded(path: &Path, maximum: u64) -> std::io::Result<Option<Vec<u8>>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("`{}` is not a bounded regular file", path.display()),
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(path)?
        .take(maximum + 1)
        .read_to_end(&mut bytes)?;
    Ok(Some(bytes))
}
