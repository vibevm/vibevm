//! `[proxy] pac = "system"` resolved by vibe, not by px
//! (`PROP-068#proxy-system`).
//!
//! px given no `--pac` auto-detects the Windows proxy configuration
//! itself, and on a managed desktop that detection fails: the PAC URL
//! lives inside the `DefaultConnectionSettings` blob of the user's
//! `Connections` key and the plain `AutoConfigURL` value is often absent,
//! so px finds nothing and every request goes nowhere. vibe therefore
//! reads the configuration itself and hands px an explicit `--pac=<url>`
//! (or `--proxy=<host:port>` when the configuration names a fixed proxy
//! instead), so px never has to detect anything.
//!
//! The resolution order, the `DefaultConnectionSettings` parser and the
//! proxy-string reduction are [`vibe_core::net::system_proxy`]: pure
//! functions over the [`WindowsProxySource`] seam, tested on every host.
//! This module is only the Windows reader behind that seam — the WinHTTP
//! and registry calls, which need `unsafe` — and the one entry point that
//! picks between it and an installed test source.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system");

use vibe_core::net::system_proxy::{
    SystemProxy, SystemProxyError, WindowsProxySource, installed_source, resolve_with,
};

#[cfg(windows)]
mod windows;

/// The route this host's Windows proxy configuration describes, resolved
/// now — the settings may have changed since the last operation, and the
/// reads are a few registry lookups and one WinHTTP call.
///
/// A source installed through [`vibe_core::net::system_proxy::install_source`]
/// answers instead of the host (the test seam); otherwise a Windows host
/// reads its own settings, and every other host refuses with
/// [`SystemProxyError::NotWindows`] — `system` has nothing to mean there.
///
/// ```
/// // Off Windows the refusal names the key and the two ways out.
/// # if !cfg!(windows) {
/// let error = vibe_net::system_proxy::resolve().unwrap_err().to_string();
/// assert!(error.contains("not Windows") && error.contains("fix:"), "{error}");
/// # }
/// ```
pub fn resolve() -> Result<SystemProxy, SystemProxyError> {
    if let Some(source) = installed_source() {
        return resolve_with(source.as_ref());
    }
    #[cfg(windows)]
    {
        resolve_with(&windows::Reader)
    }
    #[cfg(not(windows))]
    {
        Err(SystemProxyError::NotWindows)
    }
}

/// This host's own reader, for a diagnostic that wants one setting rather
/// than the resolution's verdict. `None` off Windows.
pub fn host_source() -> Option<&'static dyn WindowsProxySource> {
    #[cfg(windows)]
    {
        Some(&windows::Reader)
    }
    #[cfg(not(windows))]
    {
        None
    }
}
