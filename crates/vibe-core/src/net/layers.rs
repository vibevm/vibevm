//! Which layer decided each part of the route (`PROP-068#proxy`): the
//! mode, the PAC location and the revocation setting are each switched at
//! every layer — the environment over the user's file over the site
//! profile — and a report names the layer that won, so a user who
//! disagrees with the route knows which file to edit.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy");

use crate::user_config::{ProxyConfig, ProxyMode, RevocationCheck};

/// Which layer decided the effective `[proxy] mode` (`PROP-068#proxy`).
///
/// ```
/// use vibe_core::net::ModeOrigin;
/// assert_eq!(ModeOrigin::Env.as_str(), "VIBEVM_PROXY_MODE");
/// assert_eq!(ModeOrigin::Site.as_str(), "site profile");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeOrigin {
    /// Nothing configured a mode: the default (`explicit` when a proxy URL
    /// is set, else no proxy).
    Default,
    /// The applied site profile.
    Site,
    /// The user's `config.toml`.
    User,
    /// `VIBEVM_PROXY_MODE` in the environment.
    Env,
    /// A one-shot command-line override (`vibe dl --no-px`).
    Flag,
}

/// The environment switch of the `[proxy] mode` axis.
pub const MODE_ENV: &str = "VIBEVM_PROXY_MODE";
/// The environment switch of the `[proxy] pac` axis.
pub const PAC_ENV: &str = "VIBEVM_PROXY_PAC";
/// The environment switch of the `[proxy] revocation` axis.
pub const REVOCATION_ENV: &str = "VIBEVM_PROXY_REVOCATION";

impl ModeOrigin {
    /// The layer's name for a report, where `env_var` is the environment
    /// switch of the axis being reported: one origin serves the mode, the
    /// PAC location and the revocation setting, and each has its own
    /// variable, so the axis names it rather than the enum guessing.
    ///
    /// ```
    /// use vibe_core::net::{ModeOrigin, PAC_ENV, REVOCATION_ENV};
    /// assert_eq!(ModeOrigin::Env.label(PAC_ENV), "VIBEVM_PROXY_PAC");
    /// assert_eq!(ModeOrigin::Env.label(REVOCATION_ENV), "VIBEVM_PROXY_REVOCATION");
    /// // Every other layer reads the same whichever axis asks.
    /// assert_eq!(ModeOrigin::Site.label(PAC_ENV), "site profile");
    /// ```
    pub fn label(self, env_var: &'static str) -> &'static str {
        match self {
            ModeOrigin::Env => env_var,
            other => other.as_str(),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ModeOrigin::Default => "default",
            ModeOrigin::Site => "site profile",
            ModeOrigin::User => "user config",
            ModeOrigin::Env => "VIBEVM_PROXY_MODE",
            ModeOrigin::Flag => "command line",
        }
    }
}

/// The effective `[proxy] mode` and the layer that set it, highest first:
/// environment, the user's file, the site profile.
///
/// ```
/// use vibe_core::net::{ModeOrigin, effective_mode};
/// use vibe_core::user_config::{ProxyConfig, ProxyMode};
/// let site: ProxyConfig = toml::from_str("mode='px'\npac='https://pac.example.corp/p.pac'").unwrap();
/// let user: ProxyConfig = toml::from_str("mode='none'").unwrap();
/// // The user turns the site's px off.
/// assert_eq!(effective_mode(Some(&site), &user, None), (Some(ProxyMode::None), ModeOrigin::User));
/// // The environment beats both.
/// assert_eq!(effective_mode(Some(&site), &user, Some(ProxyMode::Explicit)), (Some(ProxyMode::Explicit), ModeOrigin::Env));
/// assert_eq!(effective_mode(Some(&site), &ProxyConfig::default(), None), (Some(ProxyMode::Px), ModeOrigin::Site));
/// assert_eq!(effective_mode(None, &ProxyConfig::default(), None), (None, ModeOrigin::Default));
/// ```
/// The effective `[proxy] pac` and its layer: `VIBEVM_PROXY_PAC`, then the
/// user's file, then the site profile.
///
/// ```
/// use vibe_core::net::{ModeOrigin, effective_pac};
/// use vibe_core::user_config::ProxyConfig;
/// let site: ProxyConfig = toml::from_str("pac='https://pac.example.corp/p.pac'").unwrap();
/// assert_eq!(effective_pac(Some(&site), &ProxyConfig::default(), None).1, ModeOrigin::Site);
/// assert_eq!(effective_pac(Some(&site), &ProxyConfig::default(), Some("system")),
///            (Some("system".to_owned()), ModeOrigin::Env));
/// ```
pub fn effective_pac(
    site: Option<&ProxyConfig>,
    user: &ProxyConfig,
    env: Option<&str>,
) -> (Option<String>, ModeOrigin) {
    if let Some(pac) = env {
        return (Some(pac.to_owned()), ModeOrigin::Env);
    }
    if let Some(pac) = &user.pac {
        return (Some(pac.clone()), ModeOrigin::User);
    }
    match site.and_then(|site| site.pac.clone()) {
        Some(pac) => (Some(pac), ModeOrigin::Site),
        None => (None, ModeOrigin::Default),
    }
}

/// The effective `[proxy] revocation` and its layer: `VIBEVM_PROXY_REVOCATION`,
/// then the user's file, then the site profile. `check` is the default
/// and exactly what vibe did before the key existed.
///
/// ```
/// use vibe_core::net::{ModeOrigin, effective_revocation};
/// use vibe_core::user_config::{ProxyConfig, RevocationCheck};
/// let site: ProxyConfig = toml::from_str("revocation = 'off'").unwrap();
/// let user: ProxyConfig = toml::from_str("revocation = 'best-effort'").unwrap();
/// assert_eq!(
///     effective_revocation(Some(&site), &ProxyConfig::default(), None),
///     (RevocationCheck::Off, ModeOrigin::Site)
/// );
/// // The user's file outranks the site, and the environment outranks both.
/// assert_eq!(
///     effective_revocation(Some(&site), &user, None),
///     (RevocationCheck::BestEffort, ModeOrigin::User)
/// );
/// assert_eq!(
///     effective_revocation(Some(&site), &user, Some(RevocationCheck::Check)),
///     (RevocationCheck::Check, ModeOrigin::Env)
/// );
/// assert_eq!(
///     effective_revocation(None, &ProxyConfig::default(), None),
///     (RevocationCheck::Check, ModeOrigin::Default)
/// );
/// ```
pub fn effective_revocation(
    site: Option<&ProxyConfig>,
    user: &ProxyConfig,
    env: Option<RevocationCheck>,
) -> (RevocationCheck, ModeOrigin) {
    if let Some(choice) = env {
        return (choice, ModeOrigin::Env);
    }
    if let Some(choice) = user.revocation {
        return (choice, ModeOrigin::User);
    }
    match site.and_then(|site| site.revocation) {
        Some(choice) => (choice, ModeOrigin::Site),
        None => (RevocationCheck::default(), ModeOrigin::Default),
    }
}

pub fn effective_mode(
    site: Option<&ProxyConfig>,
    user: &ProxyConfig,
    env: Option<ProxyMode>,
) -> (Option<ProxyMode>, ModeOrigin) {
    if let Some(mode) = env {
        return (Some(mode), ModeOrigin::Env);
    }
    if let Some(mode) = user.mode {
        return (Some(mode), ModeOrigin::User);
    }
    if let Some(mode) = site.and_then(|site| site.mode) {
        return (Some(mode), ModeOrigin::Site);
    }
    (None, ModeOrigin::Default)
}
