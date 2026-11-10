//! This invocation's one template layer (PROP-075
//! `#TEMPLATE-DOMAIN-SEAM`, `#TEMPLATE-CACHE`).
//!
//! Built and installed here, once, after the remap Locator — a template's
//! version list and digest documents are fetched **through** the Locator,
//! so installing the layer before it would give the first fetch a
//! different policy than the second. Write-once, like the Locator itself,
//! and with a total default: an invocation that installs none behaves
//! exactly as it did before PROP-075, because every one of the four call
//! sites reads `templates::active()` and does nothing when it is `None`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CACHE");

use std::sync::Arc;

use vibe_core::user_config::UserConfig;
use vibe_registry::templates::{Posture, TemplateLayer};

/// Install the invocation's template layer from the root flags, resolving
/// the offline ladder (the flag, `VIBE_OFFLINE`, then `[net] offline`) and
/// the user's `[template] version_list_ttl` here rather than in `main`.
pub fn install_for(cli_offline: bool, refresh: bool) {
    install(
        &UserConfig::load().unwrap_or_default(),
        super::vvm_offline(cli_offline),
        refresh,
    );
}

/// Install the invocation's template layer, unless one is already there.
///
/// `offline` is the whole offline ladder as the caller already resolved
/// it; `refresh` is `--refresh`. **`offline` wins**: a posture that
/// forbids the network outranks a request to use it.
pub fn install(config: &UserConfig, offline: bool, refresh: bool) {
    let Some(settings) = vibe_core::settings::settings_dir() else {
        // No settings root means no cache to read or write, and a layer
        // with nowhere to cache would fetch on every resolution. The
        // honest answer is the pre-PROP-075 behaviour: no templates.
        return;
    };
    let posture = posture_of(offline, refresh);
    let layer = TemplateLayer::http(&settings, config.template.version_list_ttl(), posture);
    let _ = vibe_registry::templates::install(Arc::new(layer));
}

/// The posture the two flags name, with `offline` outranking `refresh`.
///
/// ```
/// # use vibe_cli::composition::templates::posture_of;
/// # use vibe_registry::templates::Posture;
/// assert_eq!(posture_of(false, false), Posture::Cached);
/// assert_eq!(posture_of(false, true), Posture::Refresh);
/// assert_eq!(posture_of(true, false), Posture::Offline);
/// assert_eq!(posture_of(true, true), Posture::Offline);
/// ```
pub fn posture_of(offline: bool, refresh: bool) -> Posture {
    match (offline, refresh) {
        (true, _) => Posture::Offline,
        (false, true) => Posture::Refresh,
        (false, false) => Posture::Cached,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_outranks_refresh() {
        assert_eq!(posture_of(true, true), Posture::Offline);
        assert_eq!(posture_of(true, false), Posture::Offline);
    }

    #[test]
    fn refresh_alone_refetches_and_neither_reads_the_cache() {
        assert_eq!(posture_of(false, true), Posture::Refresh);
        assert_eq!(posture_of(false, false), Posture::Cached);
    }
}
