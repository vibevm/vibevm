//! The artifact drops and release source of this invocation, read from
//! the user config and the applied site profile beneath it for the Locator
//! (`PROP-066#drops`, `PROP-019#release-sources`, `PROP-019#site-profile`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#drops");

use std::path::{Path, PathBuf};

/// Read `[self]` and `[artifacts]` from the user config — over the site
/// profile's — and resolve the release source once
/// (`PROP-019#release-sources`, `PROP-066#drops`). An unreadable config or
/// an unusable source is a warning for this run and is recorded, so `vibe
/// self` refuses with the reason instead of quietly going back to the
/// public host. An unreadable site profile is reported by the Locator
/// build, once.
pub(super) fn artifact_drops(
    release_origin: Option<&str>,
    site_profile: Option<&Path>,
    warnings: &mut Vec<String>,
) -> (
    Option<crate::commands::vvm::bundle::release_source::EffectiveReleaseSource>,
    Option<(PathBuf, String)>,
) {
    use crate::commands::vvm::bundle::release_source;
    // The site profile's warning is the Locator's to report: it reads the
    // same file for its rows.
    let layers =
        crate::commands::vvm::site::ConfigLayers::load(site_profile, warnings, &mut Vec::new());
    let resolved = layers.release(release_origin);
    if let Err(error) = &resolved {
        warnings.push(error.to_string());
    }
    release_source::install(resolved.clone());
    let packages = layers.packages_drop(warnings);
    (resolved.ok(), packages)
}

/// The applied site profile, when one is stored; an unreadable one is a
/// warning and an absent layer, like every optional settings layer.
pub(super) fn site_layer(
    env: &super::RemapEnv,
    warnings: &mut Vec<String>,
) -> Option<vibe_core::site_profile::InstalledSiteProfile> {
    let path = env.site_profile.as_deref()?;
    match vibe_core::site_profile::installed_at(path) {
        Ok(site) => site,
        Err(error) => {
            warnings.push(error.to_string());
            None
        }
    }
}
