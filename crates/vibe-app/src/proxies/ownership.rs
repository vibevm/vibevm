//! Who owns a file at a PATH proxy's name in the shared `opt/bin`
//! (PROP-059 `#proxy`, the rules of PROP-019 §2.4 shim ownership).
//!
//! `<settings>/opt/bin` is the shim directory by default, and with
//! `[self] install_root` it is a directory the operator already keeps their
//! own tools in. `vibe app` therefore never overwrites or deletes a file it
//! did not write:
//!
//! * **Recorded** — `applications/proxies.toml` names the file with the
//!   digest it still has, or its bytes are exactly the proxy image being
//!   written: vibe wrote it. Only this tier may be deleted.
//! * **Recognised** — no record, but the content is a vibe-app-proxy image
//!   (it carries the proxy's protocol name): an older vibe's proxy. It may
//!   be rewritten, never deleted.
//! * **Foreign** — anything else: a write refuses before any file changes,
//!   and a removal leaves it in place.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#proxy");

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::settings::{SettingsRoot, io};

const INVENTORY_SCHEMA: u32 = 1;
/// A record per proxy name: far below this.
const INVENTORY_MAX_BYTES: u64 = 256 * 1024;
/// Every vibe-app-proxy image answers `--supports-index` with this protocol
/// family, so the name is in its bytes.
const PROXY_SIGNATURE: &[u8] = b"vibe-user-applications/";
/// A proxy image is a native executable of a few megabytes; the read that
/// classifies a file stops well above that.
const PROXY_MAX_BYTES: u64 = 256 * 1024 * 1024;

/// How far `vibe app` may go with the file at one proxy name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ownership {
    Absent,
    Recorded,
    Recognised,
    Foreign,
}

impl Ownership {
    pub(crate) fn writable(self) -> bool {
        !matches!(self, Ownership::Foreign)
    }
    pub(crate) fn removable(self) -> bool {
        matches!(self, Ownership::Recorded)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Record {
    name: String,
    sha256: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct File {
    schema: u32,
    #[serde(default, rename = "proxy")]
    proxies: Vec<Record>,
}

/// What `vibe app` wrote into `opt/bin`, by file name.
pub(crate) struct Inventory {
    path: PathBuf,
    records: BTreeMap<String, String>,
}

/// `<home>/applications/proxies.toml`, in vibe's own directory, never in
/// the shared `opt/bin`.
pub(crate) fn inventory_path(settings: &SettingsRoot) -> PathBuf {
    settings.applications_dir().join("proxies.toml")
}

impl Inventory {
    /// Read the inventory; a missing or damaged one records nothing, so
    /// its proxies are recognised by content alone — enough to rewrite one,
    /// never to delete one.
    pub(crate) fn load(settings: &SettingsRoot) -> Result<Self, AppError> {
        let path = inventory_path(settings);
        let records = match read_bounded(&path, INVENTORY_MAX_BYTES) {
            Ok(Some(bytes)) => String::from_utf8(bytes)
                .ok()
                .and_then(|text| toml::from_str::<File>(&text).ok())
                .map(|file| {
                    file.proxies
                        .into_iter()
                        .map(|record| (record.name, record.sha256))
                        .collect()
                })
                .unwrap_or_default(),
            Ok(None) => BTreeMap::new(),
            Err(source) => return Err(io("reading the proxy inventory", &path, source).into()),
        };
        Ok(Self { path, records })
    }

    /// Classify the file at `path`, written as `name`; `image` is the proxy
    /// image this operation writes, when it has one.
    pub(crate) fn classify(&self, name: &str, path: &Path, image: Option<&[u8]>) -> Ownership {
        let bytes = match read_bounded(path, PROXY_MAX_BYTES) {
            Ok(None) => return Ownership::Absent,
            Ok(Some(bytes)) => bytes,
            // A directory, a link, a device or an unreadable file is not a
            // proxy vibe wrote.
            Err(_) => return Ownership::Foreign,
        };
        if image.is_some_and(|image| image == bytes.as_slice())
            || self
                .records
                .get(name)
                .is_some_and(|recorded| *recorded == digest(&bytes))
        {
            return Ownership::Recorded;
        }
        if bytes
            .windows(PROXY_SIGNATURE.len())
            .any(|window| window == PROXY_SIGNATURE)
        {
            Ownership::Recognised
        } else {
            Ownership::Foreign
        }
    }

    pub(crate) fn record(&mut self, name: &str, bytes: &[u8]) {
        self.records.insert(name.to_owned(), digest(bytes));
    }

    pub(crate) fn forget(&mut self, name: &str) {
        self.records.remove(name);
    }

    pub(crate) fn save(&self) -> Result<(), AppError> {
        let file = File {
            schema: INVENTORY_SCHEMA,
            proxies: self
                .records
                .iter()
                .map(|(name, sha256)| Record {
                    name: name.clone(),
                    sha256: sha256.clone(),
                })
                .collect(),
        };
        let text = toml::to_string(&file).map_err(|error| {
            io(
                "serialising the proxy inventory",
                &self.path,
                std::io::Error::other(error),
            )
        })?;
        crate::settings::write_atomic(&self.path, text.as_bytes())?;
        Ok(())
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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
