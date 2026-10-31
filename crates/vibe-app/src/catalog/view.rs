//! What the catalog listing reads beside the index
//! (`PROP-059#launcher-catalog`): the user layer, the usage facts, the home
//! and the revision, loaded once per answer so a document of a hundred
//! launchers reads each of the three files exactly once.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog");

use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::LauncherDecl;
use vibe_wire::generated::application_launchers as wire;

use super::usage::Usage;
use super::user::{PinTarget, UserLaunchers};
use crate::error::StateError;
use crate::index::Installation;
use crate::settings::SettingsRoot;

/// The per-launcher fields the user layer and the usage facts contribute to
/// one row of the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub(crate) struct CatalogFields {
    pub(crate) key: String,
    pub(crate) collections: Vec<String>,
    pub(crate) pinned: Vec<wire::PinTarget>,
    pub(crate) hidden_by: Option<wire::HiddenBy>,
    pub(crate) display_name: String,
    pub(crate) display_icon: Option<String>,
    pub(crate) usage: wire::LauncherUsage,
}

/// The catalog state of one home: its user layer, its usage facts, its path
/// and the revision at the moment of reading.
///
/// ```
/// use vibe_app::{LauncherCatalog, SettingsRoot};
///
/// let dir = tempfile::tempdir().unwrap();
/// let settings = SettingsRoot::new(dir.path());
/// let catalog = LauncherCatalog::load(&settings).unwrap();
/// assert_eq!(catalog.revision(), 0);
/// assert_eq!(catalog.home(), dir.path());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub struct LauncherCatalog {
    home: PathBuf,
    revision: u64,
    layer: UserLaunchers,
    usage: Usage,
}

impl LauncherCatalog {
    /// Read the three files of one home. An invalid user layer refuses the
    /// listing rather than answering an arrangement that is not the user's.
    pub fn load(settings: &SettingsRoot) -> Result<Self, StateError> {
        Ok(Self {
            home: settings.path().to_path_buf(),
            revision: super::revision(settings),
            layer: UserLaunchers::load(settings)?,
            usage: Usage::load(settings)?,
        })
    }

    /// The application home these facts belong to: `$vibe_home` of the
    /// launcher templates and the owner of the icon copies.
    pub fn home(&self) -> &Path {
        &self.home
    }

    /// The revision at the moment of reading.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The user's arrangement, for a caller that wants the layer itself.
    pub fn layer(&self) -> &UserLaunchers {
        &self.layer
    }

    /// The fields one launcher of `package_key` contributes beyond its own
    /// declaration. `icon` is the launcher's own resolved icon file, which
    /// is the effective icon while the user has named none of their own.
    pub(crate) fn fields(
        &self,
        package_key: &str,
        launcher: &LauncherDecl,
        icon: Option<&str>,
    ) -> CatalogFields {
        let key = super::launcher_key(package_key, &launcher.id);
        let own = self.layer.launcher(&key);
        let usage = self.usage.of(&key);
        CatalogFields {
            collections: self.layer.collections_of(&key),
            pinned: self
                .layer
                .pins_of(&key)
                .into_iter()
                .map(|target| match target {
                    PinTarget::Taskbar => wire::PinTarget::Taskbar,
                    PinTarget::Favorites => wire::PinTarget::Favorites,
                })
                .collect(),
            hidden_by: own.filter(|row| row.hidden).map(|_| wire::HiddenBy::User),
            display_name: own
                .and_then(|row| row.name.clone())
                .unwrap_or_else(|| launcher.name.clone()),
            display_icon: own
                .and_then(|row| row.icon.clone())
                .or_else(|| icon.map(str::to_owned)),
            usage: wire::LauncherUsage {
                starts: usage.starts,
                last_started: usage.last_started,
            },
            key,
        }
    }

    /// The state of one installation: its exposure, its update hold, and
    /// whether a newer version exists — which no listing answers, because
    /// finding out needs a registry call (`vibe app outdated` makes it).
    pub(crate) fn state(installation: &Installation) -> wire::InstallationState {
        wire::InstallationState {
            hidden: installation.lifecycle.hidden,
            held: installation.lifecycle.held,
            update_available: None,
        }
    }
}
