//! `vibe dl` (PROP-068 `#dl`): one HTTP(S) download through vibe's own
//! network stack, with every phase of the route described.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#dl");

use std::path::PathBuf;

/// The target `--test-proxy` checks when none is given.
pub const DEFAULT_TEST_TARGET: &str = "https://gmail.com";

/// Arguments of `vibe dl`.
///
/// Exit codes: 0 done · 1 not HTTP / unclassified · 2 usage · 3 `--offline`
/// · 4 source policy · 5 px · 6 DNS · 7 connect timeout (likely a direct
/// route where only a proxy may leave) · 8 connection refused · 9 proxy
/// wants authentication (407) · 10 CONNECT refused · 11 untrusted
/// certificate · 12 certificate name mismatch · 13 other TLS · 14 HTTP 4xx
/// · 15 HTTP 5xx · 16 SHA-256 mismatch · 17 destination exists · 18 local
/// I/O · 19 binary body refused on a terminal · 20 redirect refused · 21
/// `--test-proxy`: proxy problem · 22 `--test-proxy`: target problem · 130
/// stopped.
#[derive(Debug, Clone, clap::Args)]
#[command(after_long_help = "\
Exit codes:
  0 done              1 not HTTP / other     2 usage
  3 --offline         4 source policy        5 px failed
  6 DNS               7 connect timeout      8 connection refused
  9 proxy 407         10 CONNECT refused     11 untrusted certificate
  12 name mismatch    13 other TLS           14 HTTP 4xx
  15 HTTP 5xx         16 SHA-256 mismatch    17 destination exists
  18 local I/O        19 binary on terminal  20 redirect refused
  21 --test-proxy: proxy problem             22 --test-proxy: target problem
  130 stopped (--stop-file, Ctrl-C)

Without --proxy the configured route is used: with nothing configured,
no proxy. Remap rows are not applied: the URL is fetched as given.")]
pub struct DlArgs {
    /// The `http://` or `https://` URL to fetch (with `--test-proxy`: the
    /// target to test, default https://gmail.com).
    pub url: Option<String>,

    /// Where to write it: a file path, or an existing directory (the
    /// URL's last segment names the file). Not with `--show` / `--headers`.
    pub dest: Option<PathBuf>,

    /// Replace an existing destination; with `--show`, write a binary body
    /// to a terminal anyway.
    #[arg(long)]
    pub force: bool,

    /// Verify the body's SHA-256 (hex); a mismatch keeps nothing.
    #[arg(long, value_name = "HEX")]
    pub sha256: Option<String>,

    /// Print the response body to stdout instead of writing a file.
    #[arg(long)]
    pub show: bool,

    /// Print the response status line and headers to stdout (before the
    /// body with `--show`).
    #[arg(long)]
    pub headers: bool,

    /// Use a proxy for this command only: bare `--proxy` runs px with the
    /// configured `pac` / `upstream`; `--proxy <url>` uses that proxy;
    /// `--proxy px`; `--proxy pac=<url>`.
    #[arg(long, value_name = "VALUE", num_args = 0..=1, default_missing_value = "")]
    pub proxy: Option<String>,

    /// Check the configured proxy against a target, step by step, instead
    /// of downloading (turns the proxy on by itself).
    #[arg(long)]
    pub test_proxy: bool,

    /// With `--test-proxy`: test the explicit / upstream proxy without px.
    #[arg(long, requires = "test_proxy")]
    pub no_px: bool,
}
