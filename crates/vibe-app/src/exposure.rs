//! Exposure of one installation outside `opt/bin`: its public commands as
//! proxies in a directory of their own, chosen through `VIBE_APP_USE`, for
//! one shell session (`vibe shell --with`) or one requiring step's PATH
//! (`PROP-059#hidden-installations`, `#session-selection`,
//! `PROP-065#app-dependencies`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#hidden-installations");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#session-selection");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use specmark::spec;

use crate::error::AppError;
use crate::index::{Index, Installation, Location};
use crate::launch;
use crate::operations::installed;
use crate::proxies;
use crate::settings::SettingsRoot;
use crate::storage::Storage;

/// What a session or a step puts first on PATH to reach one installation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#hidden-installations")]
pub struct Exposure {
    pub slot: String,
    pub key: String,
    /// The directory of the installation's own proxies, one per public
    /// command; the proxies pick the key through `VIBE_APP_USE`.
    pub proxies: PathBuf,
    /// The `bin` directory of an owned payload, when it exists.
    pub bin: Option<PathBuf>,
}

impl Exposure {
    /// The `VIBE_APP_USE` entry `slot=key` that selects the installation.
    pub fn choice(&self) -> String {
        format!("{}={}", self.slot, self.key)
    }
}

/// Expose the installation `spelling` names (`group/name[@req]`) for one
/// session: write its proxies and return the PATH entries and the session
/// choice (`vibe shell --with`). The global default is untouched.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let image = dir.path().join("vibe-app-proxy");
/// let error = vibe_app::expose_for_session(&settings, "org.example/gcc", &image).unwrap_err();
/// assert_eq!(error.code(), "not_installed");
/// ```
pub fn expose_for_session(
    settings: &SettingsRoot,
    spelling: &str,
    proxy_image: &Path,
) -> Result<Exposure, AppError> {
    let lock = settings.lock()?;
    let index = Index::load(settings)?;
    let storage = Storage::load(settings)?;
    let (key, record) = installed(settings, &index, spelling)?;
    exposure_of(settings, &storage, key, record, proxy_image, &lock)
}

/// The exposure of one installation record; writes its proxy directory.
pub(crate) fn exposure_of(
    settings: &SettingsRoot,
    storage: &Storage,
    key: &str,
    record: &Installation,
    proxy_image: &Path,
    lock: &crate::MutationLock,
) -> Result<Exposure, AppError> {
    lock.require_home(settings)?;
    let dir = settings
        .app_proxies_dir()
        .join(record.package.directory_name())
        .join("bin");
    let commands: BTreeSet<String> = record
        .public_commands
        .iter()
        .map(|command| command.name.clone())
        .collect();
    let image = if commands.is_empty() {
        None
    } else {
        Some(proxies::CapturedImage::read(
            proxy_image,
            Index::load_with_protocol(settings)?.1,
        )?)
    };
    proxies::sync_captured(
        settings,
        &dir,
        &BTreeSet::new(),
        &commands,
        image.as_ref(),
        lock,
    )?;
    let bin = match &record.location {
        Location::Owned { .. } => launch::app_root(record, storage)
            .ok()
            .map(|root| root.join("bin"))
            .filter(|bin| bin.is_dir()),
        Location::External { .. } => None,
    };
    Ok(Exposure {
        slot: record.slot.clone(),
        key: key.to_owned(),
        proxies: dir,
        bin,
    })
}
