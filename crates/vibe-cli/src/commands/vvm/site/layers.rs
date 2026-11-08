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
use vibe_core::user_config::UserConfig;

use crate::commands::vvm::bundle::release_source::{self, EffectiveReleaseSource};

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
    pub(crate) fn load(
        site_path: Option<&Path>,
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
            own,
            own_path,
            site,
        }
    }

    /// Whether the release decision comes from the site profile: the user
    /// declares neither `[self]` nor `[artifacts] vibe`, the site does.
    fn release_from_site(&self) -> Option<&InstalledSiteProfile> {
        self.site.as_ref().filter(|site| {
            self.own.self_source.is_default()
                && self.own.artifacts.vibe.is_none()
                && site.profile.declares_release_source()
        })
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
    /// it. A relative drop is refused with a warning naming that file.
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
