//! `[proxy] pac = "system"` resolved by vibe, not by px
//! (`PROP-068#proxy-system`).
//!
//! px given no `--pac` auto-detects the proxy configuration itself, and
//! on a managed desktop that detection fails: on Windows the PAC URL
//! lives inside the `DefaultConnectionSettings` blob of the user's
//! `Connections` key and the plain `AutoConfigURL` value is often absent,
//! and off Windows px does not read the desktop's settings at all — so px
//! finds nothing and every request goes nowhere. vibe therefore reads the
//! configuration itself and hands px an explicit `--pac=<url>` (or
//! `--proxy=<host:port>` when the configuration names a fixed proxy
//! instead), so px never has to detect anything.
//!
//! Auto (`pac = "system"`) therefore means something on every host vibe
//! runs on: the Windows registry / WinHTTP order
//! ([`vibe_core::net::resolve_with`]), the macOS network settings of the
//! current network service ([`vibe_core::net::resolve_mac_with`]), and
//! the Linux desktops then the environment
//! ([`vibe_core::net::resolve_unix_with`]). All three orders, their
//! parsers and their string reductions are pure functions over an
//! injected settings seam, tested on every host; this module is only the
//! readers behind those seams — the WinHTTP and registry calls, which
//! need `unsafe`, and the `gsettings` / `kioslaverc` / `scutil` /
//! environment / WPAD reads — plus the one entry point that picks
//! between them and an installed test source.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system");

use vibe_core::net::system_proxy::{
    SystemProxy, SystemProxyError, UnixProxySource, WindowsProxySource, installed_mac_source,
    installed_source, installed_unix_source, resolve_mac_with, resolve_unix_with, resolve_with,
};

#[cfg(windows)]
mod windows;

mod unix;
pub use unix::{FIXTURE_ENV, HostKind, KIND_ENV};

/// The route this host's own proxy configuration describes, resolved now
/// — the settings may have changed since the last operation, and the
/// reads are a few registry lookups and one WinHTTP call (Windows), one
/// `scutil` run (macOS) or one `gsettings` run and one file read (Linux).
///
/// A source installed through
/// [`vibe_core::net::system_proxy::install_source`] (Windows),
/// [`install_unix_source`](vibe_core::net::system_proxy::install_unix_source)
/// or
/// [`install_mac_source`](vibe_core::net::system_proxy::install_mac_source)
/// answers instead of the host (the test seam); otherwise the host reads
/// its own settings in the order its kind prescribes, and a host of no
/// kind vibe knows refuses with [`SystemProxyError::Unsupported`].
///
/// ```
/// // Every host resolves or refuses with a message naming the fix.
/// let answer = vibe_net::system_proxy::resolve();
/// if let Err(error) = answer {
///     assert!(error.to_string().contains("fix:"), "{error}");
/// }
/// ```
pub fn resolve() -> Result<SystemProxy, SystemProxyError> {
    if let Some(source) = installed_source() {
        return resolve_with(source.as_ref());
    }
    if let Some(source) = installed_mac_source() {
        return resolve_mac_with(source.as_ref());
    }
    if let Some(source) = installed_unix_source() {
        return resolve_unix_with(source.as_ref());
    }
    match HostKind::current() {
        HostKind::Windows => windows_resolve(),
        HostKind::MacOs => resolve_mac_with(&unix::Reader::current()),
        HostKind::Unix => resolve_unix_with(&unix::Reader::current()),
        HostKind::Other => Err(SystemProxyError::Unsupported),
    }
}

/// The Windows order against this host's registry and WinHTTP. A host
/// that is not Windows has no such reader, so the kind cannot be forced
/// onto it.
fn windows_resolve() -> Result<SystemProxy, SystemProxyError> {
    #[cfg(windows)]
    {
        resolve_with(&windows::Reader)
    }
    #[cfg(not(windows))]
    {
        Err(SystemProxyError::Unsupported)
    }
}

/// This host's own Windows reader, for a diagnostic that wants one
/// setting rather than the resolution's verdict. `None` off Windows.
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

/// This host's own non-Windows reader, for the same kind of diagnostic:
/// the desktop, macOS and environment settings as they are on this
/// machine.
pub fn unix_host_source() -> impl UnixProxySource {
    unix::Reader::current()
}
