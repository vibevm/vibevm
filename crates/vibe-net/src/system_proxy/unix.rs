//! This host's own proxy settings, off Windows (`PROP-068#proxy-system`).
//!
//! The readers behind the [`UnixProxySource`] seam, and nothing else: one
//! `gsettings` run, one file read, one `scutil` run, the proxy variables
//! of the environment, the DNS search domains of `/etc/resolv.conf`, and
//! a bounded direct fetch of the WPAD candidates. The order they are read
//! in, and what their text means, are the pure functions of
//! [`vibe_core::net::system_proxy`], so they are tested on every host.
//!
//! Reading the environment IS this file's job — `https_proxy` and its
//! siblings are a proxy configuration source of their own, the lowest one
//! — which is why `conform.toml` sanctions the ambient reads here and
//! nowhere else in the crate.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system");

use std::{path::PathBuf, process::Command, time::Duration};

use vibe_core::net::UnixProxySource;

/// Each WPAD candidate gets this long, total, and the walk is short
/// (`vibe_core`'s candidate limit): px is waiting for the answer.
const WPAD_TIMEOUT: Duration = Duration::from_secs(3);

/// The proxy variables of the environment, in the spellings the
/// resolution looks for. Both cases of each, because the tools that
/// export them disagree about which they write.
const PROXY_VARS: &[&str] = &[
    "https_proxy",
    "HTTPS_PROXY",
    "http_proxy",
    "HTTP_PROXY",
    "all_proxy",
    "ALL_PROXY",
    "no_proxy",
    "NO_PROXY",
];

/// The test seam of `PROP-068#proxy-system`: a directory standing in for
/// this host's desktop configuration, so a test — including one that runs
/// a whole `vibe` — can state a GNOME, KDE or macOS configuration without
/// touching the real desktop. Unset, nothing changes and the host is read.
pub const FIXTURE_ENV: &str = "VIBEVM_SYSTEM_PROXY_FIXTURE";

/// The other half of the seam: which resolution order runs, so the macOS
/// order is exercisable on a Linux box. `unix`, `macos` or `windows`;
/// unset, this host's own kind.
pub const KIND_ENV: &str = "VIBEVM_SYSTEM_PROXY_OS";

/// Which order [`crate::system_proxy::resolve`] runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKind {
    /// The Windows registry / WinHTTP order.
    Windows,
    /// The macOS network settings, then the environment.
    MacOs,
    /// The Linux desktops, then the environment.
    Unix,
    /// A host with no proxy configuration vibe knows how to read.
    Other,
}

impl HostKind {
    /// This host's kind, or the one [`KIND_ENV`] names.
    pub fn current() -> Self {
        match std::env::var(KIND_ENV)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "windows" => return HostKind::Windows,
            "macos" | "darwin" => return HostKind::MacOs,
            "unix" | "linux" => return HostKind::Unix,
            _ => {}
        }
        if cfg!(windows) {
            HostKind::Windows
        } else if cfg!(target_os = "macos") {
            HostKind::MacOs
        } else if cfg!(unix) {
            HostKind::Unix
        } else {
            HostKind::Other
        }
    }
}

/// This host's own reader: the real `gsettings`, `kioslaverc`, `scutil`,
/// environment, `/etc/resolv.conf` and WPAD fetch — or, when
/// [`FIXTURE_ENV`] names a directory, the files in it instead of the
/// desktop (the environment and the fixture's recorded WPAD answer still
/// come from the process, so a test states those too).
#[derive(Debug, Default)]
pub struct Reader {
    /// The fixture directory, when the seam is installed.
    fixture: Option<PathBuf>,
}

impl Reader {
    /// The reader for this process: the host's own, or the fixture
    /// [`FIXTURE_ENV`] names.
    pub fn current() -> Self {
        Self {
            fixture: std::env::var_os(FIXTURE_ENV)
                .map(PathBuf::from)
                .filter(|path| !path.as_os_str().is_empty()),
        }
    }

    /// One fixture file, when the seam is installed: `Some(Ok(None))` for
    /// a file the fixture does not carry, `None` when there is no
    /// fixture at all and the host is to be read.
    fn fixture_file(&self, name: &str) -> Option<Result<Option<String>, String>> {
        let path = self.fixture.as_ref()?.join(name);
        Some(match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("`{}`: {error}", path.display())),
        })
    }

    /// One command's standard output, trimmed of nothing: a command that
    /// is not installed is "not configured on this host", and a command
    /// that failed is reported with its own words.
    fn output(program: &str, args: &[&str]) -> Result<Option<String>, String> {
        match Command::new(program).args(args).output() {
            Ok(done) if done.status.success() => {
                Ok(Some(String::from_utf8_lossy(&done.stdout).into_owned()))
            }
            Ok(done) => Err(format!(
                "`{program}` exited {} ({})",
                done.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&done.stderr).trim()
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("`{program}` could not be run: {error}")),
        }
    }

    /// The user's `kioslaverc`: `$XDG_CONFIG_HOME/kioslaverc`, else
    /// `$HOME/.config/kioslaverc`. vibe's own settings-dir chokepoint
    /// resolves `~/.vibe`, not the desktop's configuration directory, so
    /// the two variables are read here.
    fn kioslaverc() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty())
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .filter(|path| !path.as_os_str().is_empty())
                    .map(|home| home.join(".config"))
            })?;
        Some(base.join("kioslaverc"))
    }
}

impl UnixProxySource for Reader {
    fn gnome_settings(&self) -> Result<Option<String>, String> {
        if let Some(fixture) = self.fixture_file("gsettings") {
            return fixture;
        }
        Reader::output("gsettings", &["list-recursively", "org.gnome.system.proxy"])
    }

    fn kde_settings(&self) -> Result<Option<String>, String> {
        if let Some(fixture) = self.fixture_file("kioslaverc") {
            return fixture;
        }
        let Some(path) = Reader::kioslaverc() else {
            return Ok(None);
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("`{}`: {error}", path.display())),
        }
    }

    fn scutil_proxies(&self) -> Result<Option<String>, String> {
        if let Some(fixture) = self.fixture_file("scutil-proxy") {
            return fixture;
        }
        // `scutil --proxy` prints exactly the `SCDynamicStoreCopyProxies`
        // dictionary, which is the one place macOS keeps the answer.
        Reader::output("/usr/sbin/scutil", &["--proxy"])
    }

    fn proxy_environment(&self) -> Vec<(String, String)> {
        PROXY_VARS
            .iter()
            .filter_map(|name| {
                std::env::var(name)
                    .ok()
                    .map(|value| ((*name).to_owned(), value))
            })
            .collect()
    }

    fn dns_domains(&self) -> Result<Vec<String>, String> {
        if let Some(fixture) = self.fixture_file("dns-domains") {
            return Ok(fixture?
                .unwrap_or_default()
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect());
        }
        let text = match std::fs::read_to_string("/etc/resolv.conf") {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(format!("`/etc/resolv.conf`: {error}")),
        };
        let mut domains: Vec<String> = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            let Some((key, rest)) = line.split_once(char::is_whitespace) else {
                continue;
            };
            if !matches!(key, "search" | "domain") {
                continue;
            }
            for entry in rest.split_whitespace() {
                let entry = entry.to_owned();
                if !domains.contains(&entry) {
                    domains.push(entry);
                }
            }
        }
        Ok(domains)
    }

    fn fetch_wpad(&self, candidates: &[String]) -> Result<Option<String>, String> {
        if let Some(fixture) = self.fixture_file("wpad") {
            let answered = fixture?.map(|text| text.trim().to_owned());
            return Ok(answered
                .filter(|url| !url.is_empty())
                .filter(|url| candidates.iter().any(|candidate| candidate == url)));
        }
        // A WPAD URL must be reachable without the proxy it configures,
        // exactly like a configured PAC URL, so the fetch is direct.
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .timeout(WPAD_TIMEOUT)
            .build()
            .map_err(|error| error.to_string())?;
        for candidate in candidates {
            let answered = client.get(candidate).send().and_then(|answer| {
                let status = answer.status();
                answer.text().map(|body| (status, body))
            });
            if let Ok((status, body)) = answered
                && status.is_success()
                && vibe_core::net::looks_like_pac(&body)
            {
                return Ok(Some(candidate.clone()));
            }
        }
        // Nothing answered with a PAC file; the resolution names the
        // whole candidate list it asked.
        Ok(None)
    }
}
