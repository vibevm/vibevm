//! The user's own config layered over the applied site profile, with the
//! provenance each effective value reports (`PROP-019#site-profile`).
//!
//! `vibe_core::user_config::UserConfig::load` already returns the merged
//! values; what it cannot say is *which file* decided — and the release
//! source and the package drop name their file in `vibe self source show`,
//! `vibe remap explain` and every refusal. This view keeps both layers.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use std::path::{Path, PathBuf};

use vibe_core::site_profile::InstalledSiteProfile;
use vibe_core::user_config::{InstallRootError, UserConfig};

use crate::commands::vvm::bundle::release_source::{self, EffectiveReleaseSource};

/// What decided the install root (`PROP-019#install-root`).
///
/// The distinction is not decoration: a base the environment already names,
/// or the built-in default, needs no durable write — only a base a FILE
/// declares has to survive the terminal that applied it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InstallBaseOrigin {
    /// `$VIBEVM_INSTALL_ROOT`, inherited by this invocation.
    Environment,
    /// `[self] install_root` in the named file — the user's own
    /// `config.toml` or the stored site profile.
    Declared(PathBuf),
    /// Nothing named one: `~/.vibe`.
    BuiltIn,
}

impl InstallBaseOrigin {
    /// The wire and human spelling the report carries.
    pub(crate) fn describe(&self) -> String {
        match self {
            InstallBaseOrigin::Environment => {
                format!("${}", crate::commands::vvm::VIBEVM_INSTALL_ROOT_ENV)
            }
            InstallBaseOrigin::Declared(file) => {
                format!("[self] install_root in {}", file.display())
            }
            InstallBaseOrigin::BuiltIn => "built-in default".to_owned(),
        }
    }

    /// Whether a durable `$VIBEVM_INSTALL_ROOT` is needed for the next
    /// terminal to resolve the same installation.
    pub(crate) fn needs_durable_write(&self) -> bool {
        matches!(self, InstallBaseOrigin::Declared(_))
    }
}

/// The two configuration layers of one machine.
#[derive(Debug, Clone, Default)]
pub(crate) struct ConfigLayers {
    /// `~/.vibe/config.toml` as the user wrote it.
    pub(crate) own: UserConfig,
    pub(crate) own_path: Option<PathBuf>,
    /// The applied site profile, when one is stored.
    pub(crate) site: Option<InstalledSiteProfile>,
}

impl ConfigLayers {
    /// Read both layers. An unreadable layer is a warning and an absent
    /// layer — the user file's into `own_warnings`, the site profile's into
    /// `site_warnings` — so the composition root never fails a command over
    /// an optional file and can report each once.
    /// `home` resolves the per-user paths a layer may root at `~` — the
    /// artifact drops, `[self] path`, `[self] install_root` — against the
    /// same home directory `~/.vibe` comes from, so one site profile can
    /// name each reader's own directories (`PROP-019#install-root`,
    /// `PROP-066#drops`). The files keep the `~` they were written with.
    pub(crate) fn load(
        site_path: Option<&Path>,
        home: Option<&Path>,
        own_warnings: &mut Vec<String>,
        site_warnings: &mut Vec<String>,
    ) -> Self {
        let own_path = UserConfig::default_path();
        let own = match UserConfig::load_own() {
            Ok(config) => config,
            Err(error) => {
                own_warnings.push(error.to_string());
                UserConfig::default()
            }
        };
        let site = match site_path.map(vibe_core::site_profile::installed_at) {
            Some(Ok(site)) => site,
            Some(Err(error)) => {
                site_warnings.push(error.to_string());
                None
            }
            None => None,
        };
        Self {
            own: own.with_home(home),
            own_path,
            site: site.map(|site| InstalledSiteProfile {
                profile: site.profile.with_home(home),
                path: site.path,
            }),
        }
    }

    /// Whether the release decision comes from the site profile: the user
    /// declares neither `[self]` nor `[artifacts] vibe`, the site does.
    fn release_from_site(&self) -> Option<&InstalledSiteProfile> {
        self.site.as_ref().filter(|site| {
            self.own.self_source.release_is_default()
                && self.own.artifacts.vibe.is_none()
                && site.profile.declares_release_source()
        })
    }

    /// The install root vibe keeps itself under, and what decided it
    /// (`PROP-019#install-root`): the environment variable this invocation
    /// inherited, then the user's own `[self] install_root`, then the site
    /// profile's, then `~/.vibe`. The store lives at `<base>/opt`, so the
    /// shims an operator puts on PATH are `<base>/opt/bin`.
    ///
    /// A declared root is `~`-expanded against the same `home` the
    /// built-in `~/.vibe` default is read from, so one profile can say
    /// `install_root = "~"` and every reader gets vibe in their own
    /// `~/opt`; a declared root that names no directory — `~user`, or a
    /// relative path, read from wherever a command happens to run — is
    /// refused by name rather than silently taken as written.
    ///
    /// `Ok(None)` only when nothing names a base and no home directory
    /// resolves — the same state in which the version manager has no root
    /// at all.
    pub(crate) fn install_base(
        &self,
        env_override: Option<&Path>,
        home: Option<&Path>,
    ) -> Result<Option<(PathBuf, InstallBaseOrigin)>, InstallRootError> {
        if let Some(base) = env_override {
            return Ok(Some((base.to_path_buf(), InstallBaseOrigin::Environment)));
        }
        if let Some(base) = self.own.self_source.install_root_at(home) {
            let file = self
                .own_path
                .clone()
                .unwrap_or_else(|| PathBuf::from("config.toml"));
            return Ok(Some((base?, InstallBaseOrigin::Declared(file))));
        }
        if let Some(site) = &self.site
            && let Some(base) = site.profile.self_source.install_root_at(home)
        {
            return Ok(Some((
                base?,
                InstallBaseOrigin::Declared(site.path.clone()),
            )));
        }
        Ok(home.map(|home| (home.join(".vibe"), InstallBaseOrigin::BuiltIn)))
    }

    /// The effective release source and the file that decided it
    /// (`PROP-019#release-sources`, `#site-profile`).
    pub(crate) fn release(
        &self,
        env_origin: Option<&str>,
    ) -> Result<EffectiveReleaseSource, release_source::ReleaseSourceError> {
        match self.release_from_site() {
            Some(site) => release_source::resolve(
                env_origin,
                &site.profile.user_config_layer(),
                Some(&site.path),
            ),
            None => release_source::resolve(env_origin, &self.own, self.own_path.as_deref()),
        }
    }

    /// The effective `[artifacts] packages` drop and the file that declared
    /// it. A relative drop is refused with a warning naming that file; a
    /// drop written `~/<rest>` was resolved when the layer was read, so it
    /// is absolute here (`PROP-066#drops`).
    pub(crate) fn packages_drop(&self, warnings: &mut Vec<String>) -> Option<(PathBuf, String)> {
        let (drop, declared_in) = match (&self.own.artifacts.packages, &self.site) {
            (Some(drop), _) => (
                drop.clone(),
                self.own_path.as_deref().map_or_else(
                    || "config.toml".to_owned(),
                    |path| path.display().to_string(),
                ),
            ),
            (None, Some(site)) => (
                site.profile.artifacts.packages.clone()?,
                site.path.display().to_string(),
            ),
            (None, None) => return None,
        };
        if drop.is_absolute() {
            return Some((drop, declared_in));
        }
        warnings.push(format!(
            "`[artifacts] packages = \"{}\"` in `{declared_in}` is relative and was ignored \
             (violates spec://org.vibevm.core/vibevm/common/PROP-066#drops; fix: give the \
             drop as an absolute path)",
            drop.display()
        ));
        None
    }
}
