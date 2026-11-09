//! `[proxy]` — the one proxy table of the user-level config
//! (`PROP-066#locator`, `PROP-068#proxy`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy");

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// `[proxy]` section — the one place a proxy is configured
/// (`PROP-066#locator`, `PROP-068#proxy`).
///
/// A corporate network that allows one artifact store usually allows it
/// through one proxy. The three familiar keys keep their meaning:
/// [`ProxyConfig::https`] and [`ProxyConfig::http`] are proxy URLs and
/// [`ProxyConfig::no_proxy`] is the comma-separated bypass list (domain
/// suffixes, exact hosts, IP addresses, CIDR ranges; loopback always
/// bypasses). Every HTTP client vibe builds and every child `git` it
/// starts is handed this route explicitly, together with the trust set
/// (the operating system's root store plus [`ProxyConfig::ca_file`]).
///
/// `mode = "px"` asks vibe to run the px bridge itself for the operation
/// that needs the network, routed by [`ProxyConfig::pac`] or
/// [`ProxyConfig::upstream`]; px authenticates to the corporate proxy
/// with the logged-in user's single sign-on.
///
/// ```
/// use vibe_core::user_config::{ProxyConfig, ProxyMode};
///
/// let p: ProxyConfig = toml::from_str(
///     "https = \"http://proxy.corp.example:3128\"\nno_proxy = \"localhost,.corp.example\"",
/// )
/// .unwrap();
/// assert_eq!(p.https.as_deref(), Some("http://proxy.corp.example:3128"));
/// assert_eq!(p.effective_mode(), Some(ProxyMode::Explicit));
/// assert!(!p.is_default());
/// // The all-empty table is the default, and is skipped on write.
/// assert!(ProxyConfig::default().is_default());
/// assert_eq!(ProxyConfig::default().effective_mode(), None);
/// // The promoted environment is exactly the keys that are set.
/// assert_eq!(p.child_env(), [
///     ("HTTPS_PROXY", "http://proxy.corp.example:3128".to_owned()),
///     ("NO_PROXY", "localhost,.corp.example".to_owned()),
/// ]);
///
/// let px: ProxyConfig =
///     toml::from_str("mode = \"px\"\npac = \"https://pac.example.corp/proxy.pac\"").unwrap();
/// assert_eq!(px.effective_mode(), Some(ProxyMode::Px));
/// // A managed px has no fixed address to export at startup.
/// assert!(px.child_env().is_empty());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProxyConfig {
    /// `explicit` (the proxy URLs below, never px), `px` (vibe-managed px)
    /// or `none` (no proxy at all, whatever else the table or a lower layer
    /// says). Absent means `explicit` when a proxy URL is set and no proxy
    /// otherwise — px is never enabled without `mode = "px"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<ProxyMode>,
    /// Proxy for `https://` requests, exported as `HTTPS_PROXY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub https: Option<String>,
    /// Proxy for `http://` requests, exported as `HTTP_PROXY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http: Option<String>,
    /// Hosts that bypass the proxy, exported as `NO_PROXY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_proxy: Option<String>,
    /// `mode = "px"`: where the PAC file comes from — an `http(s)://` URL px
    /// fetches (it must be reachable without the proxy it configures), a
    /// local file (an absolute path or `file:///…`), or `system` (alias
    /// `windows`): whatever proxy configuration this operating system is
    /// configured with, which vibe reads itself and turns into the
    /// explicit `--pac=<url>` px is given. See [`PacSource`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pac: Option<String>,
    /// `mode = "px"`: the upstream proxy px authenticates to (`--proxy`),
    /// `host:port`; used when no PAC file decides the route.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    /// `mode = "px"`: extra px arguments a site needs (`--auth=NTLM`, …).
    /// Arguments vibe owns (listen address, port, lifetime) are refused.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub px_args: Vec<String>,
    /// `mode = "px"`: the px executable to run when this installation does
    /// not carry one in `tools/px/`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub px_path: Option<PathBuf>,
    /// `mode = "px"`: a fixed loopback port for px, for machines whose
    /// local security allows only listed ports. Absent: a free port per
    /// operation. A busy fixed port is an error, never another port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub px_port: Option<u16>,
    /// A PEM bundle of additional certificate authorities (the corporate
    /// TLS-inspection CA), trusted together with the operating system's
    /// root store. Verification is never switched off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca_file: Option<PathBuf>,
    /// Whether a child `git` using Windows schannel checks certificate
    /// revocation: `check` (the default, today's behaviour),
    /// `best-effort` or `off`. See [`RevocationCheck`]; it changes
    /// nothing on a host that is not Windows and nothing about the
    /// certificate chain itself, which is always verified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revocation: Option<RevocationCheck>,
}

/// Whether certificate revocation is checked on a route whose TLS runs
/// through Windows schannel (`PROP-068#proxy-revocation`).
///
/// A corporate TLS-inspecting proxy re-signs every certificate with its
/// own CA, and that CA's CRL and OCSP endpoints usually live outside the
/// network it inspects. Schannel then cannot complete the revocation
/// check and fails the handshake — the same failure as
/// `curl` without `--ssl-no-revoke`. The chain is still verified against
/// the trust set; only the revocation lookup is relaxed.
///
/// ```
/// use vibe_core::user_config::RevocationCheck;
/// assert_eq!(RevocationCheck::parse("best-effort"), Some(RevocationCheck::BestEffort));
/// assert_eq!(RevocationCheck::parse("BEST_EFFORT"), Some(RevocationCheck::BestEffort));
/// assert_eq!(RevocationCheck::parse("off"), Some(RevocationCheck::Off));
/// assert_eq!(RevocationCheck::parse("check"), Some(RevocationCheck::Check));
/// assert_eq!(RevocationCheck::parse("maybe"), None);
/// assert_eq!(RevocationCheck::Off.as_str(), "off");
/// // The default is exactly what vibe did before the key existed.
/// assert_eq!(RevocationCheck::default(), RevocationCheck::Check);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RevocationCheck {
    /// Check revocation and fail the handshake when the check cannot be
    /// completed — schannel's own default, and vibe's.
    #[default]
    Check,
    /// Check revocation, but accept a certificate whose revocation state
    /// could not be retrieved (`CURLSSLOPT_REVOKE_BEST_EFFORT`): a
    /// certificate the CA says is revoked is still refused.
    BestEffort,
    /// Do not check revocation at all (`CURLSSLOPT_NO_REVOKE`).
    Off,
}

impl RevocationCheck {
    pub fn as_str(self) -> &'static str {
        match self {
            RevocationCheck::Check => "check",
            RevocationCheck::BestEffort => "best-effort",
            RevocationCheck::Off => "off",
        }
    }

    /// Parse `check` / `best-effort` / `off` (the
    /// `VIBEVM_PROXY_REVOCATION` spelling; `_` reads as `-`).
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "check" => Some(RevocationCheck::Check),
            "best-effort" => Some(RevocationCheck::BestEffort),
            "off" => Some(RevocationCheck::Off),
            _ => None,
        }
    }
}

/// How the `[proxy]` table routes traffic (`PROP-068#proxy`).
///
/// ```
/// use vibe_core::user_config::ProxyMode;
/// assert_eq!(ProxyMode::Px.as_str(), "px");
/// assert_eq!(ProxyMode::Explicit.as_str(), "explicit");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    /// The `https` / `http` proxy URLs, used as written.
    Explicit,
    /// A px process vibe starts for the operation and stops after it.
    Px,
    /// No proxy: every request connects directly.
    None,
}

impl ProxyMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ProxyMode::Explicit => "explicit",
            ProxyMode::Px => "px",
            ProxyMode::None => "none",
        }
    }

    /// Parse `px` / `explicit` / `none` (the `VIBEVM_PROXY_MODE` spelling).
    ///
    /// ```
    /// use vibe_core::user_config::ProxyMode;
    /// assert_eq!(ProxyMode::parse("NONE"), Some(ProxyMode::None));
    /// assert_eq!(ProxyMode::parse("socks"), None);
    /// ```
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "explicit" => Some(ProxyMode::Explicit),
            "px" => Some(ProxyMode::Px),
            "none" => Some(ProxyMode::None),
            _ => None,
        }
    }
}

/// Where a PAC file comes from (`[proxy] pac`, `PROP-068#proxy`).
///
/// ```
/// use vibe_core::user_config::PacSource;
/// assert!(matches!(PacSource::parse("https://pac.example.corp/proxy.pac"), Ok(PacSource::Url(_))));
/// assert!(matches!(PacSource::parse("file:///etc/corp/proxy.pac"), Ok(PacSource::File(_))));
/// assert!(matches!(PacSource::parse("C:/Work/opt/proxy.pac"), Ok(PacSource::File(_))));
/// assert!(matches!(PacSource::parse("/etc/corp/proxy.pac"), Ok(PacSource::File(_))));
/// assert_eq!(PacSource::parse("SYSTEM"), Ok(PacSource::System));
/// // `windows` is an accepted alias for the same thing.
/// assert_eq!(PacSource::parse("windows"), Ok(PacSource::System));
/// assert!(PacSource::parse("proxy.pac").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacSource {
    /// Fetched by px over HTTP(S), without a proxy.
    Url(String),
    /// A local file, as px reads it (`--pac=<path or file:// URL>`).
    File(String),
    /// Whatever proxy configuration this operating system is configured
    /// with — the logged-in Windows user's Internet Options, the macOS
    /// network settings of the current network service, or the Linux
    /// desktops and then the environment — resolved by vibe into the
    /// explicit `--pac=<url>` (or `--proxy=<host:port>`) px is given, and
    /// never left to px's own detection
    /// (`vibe_core::net::system_proxy`, `PROP-068#proxy-system`).
    System,
}

impl PacSource {
    /// The spellings of [`PacSource::System`]: `system` is canonical and
    /// what every report prints; `windows` is an accepted alias, from
    /// when the Windows Internet Options were the only configuration
    /// vibe could read.
    pub const SYSTEM_SPELLINGS: &'static [&'static str] = &["system", "windows"];

    /// Parse a `pac` value; a relative path is refused (px would read it
    /// relative to its own directory).
    pub fn parse(raw: &str) -> Result<Self, &'static str> {
        let value = raw.trim();
        if Self::SYSTEM_SPELLINGS
            .iter()
            .any(|spelling| value.eq_ignore_ascii_case(spelling))
        {
            return Ok(PacSource::System);
        }
        if value.starts_with("http://") || value.starts_with("https://") {
            return Ok(PacSource::Url(value.to_owned()));
        }
        let bytes = value.as_bytes();
        let windows_absolute = bytes.len() > 2
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\');
        if value.starts_with("file://")
            || value.starts_with('/')
            || value.starts_with("\\\\")
            || windows_absolute
        {
            return Ok(PacSource::File(value.to_owned()));
        }
        Err("give an `http(s)://` URL, an absolute path or `file:///…`, or `system`")
    }

    /// The location as px is given it: a `file://` URL becomes the local
    /// path it names (px 0.11 turns `file:///srv/p.pac` into `C:/srv/p.pac`
    /// on Linux), a URL and a path stay as written.
    ///
    /// ```
    /// use vibe_core::user_config::PacSource;
    /// let file = PacSource::parse("file:///etc/corp/proxy.pac").unwrap();
    /// assert_eq!(file.px_location().as_deref(), Some("/etc/corp/proxy.pac"));
    /// let drive = PacSource::parse("file:///C:/Work/opt/proxy.pac").unwrap();
    /// assert_eq!(drive.px_location().as_deref(), Some("C:/Work/opt/proxy.pac"));
    /// assert_eq!(PacSource::System.px_location(), None);
    /// ```
    pub fn px_location(&self) -> Option<String> {
        match self {
            PacSource::Url(url) => Some(url.clone()),
            PacSource::File(location) => Some(match location.strip_prefix("file://") {
                Some(rest) => {
                    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
                    let bytes = rest.as_bytes();
                    if bytes.len() > 2
                        && bytes[0] == b'/'
                        && bytes[1].is_ascii_alphabetic()
                        && bytes[2] == b':'
                    {
                        rest[1..].to_owned()
                    } else {
                        rest.to_owned()
                    }
                }
                None => location.clone(),
            }),
            PacSource::System => None,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            PacSource::Url(_) => "url",
            PacSource::File(_) => "file",
            PacSource::System => "system",
        }
    }
}

/// px arguments vibe sets itself; a `px_args` entry naming one is refused.
pub const PX_OWNED_ARGS: &[&str] = &[
    "--listen",
    "--port",
    "--gateway",
    "--hostonly",
    "--allow",
    "--foreground",
    "--pac",
    "--proxy",
    "--server",
    "--noproxy",
    "--quit",
    "--restart",
    "--install",
    "--uninstall",
    "--save",
    "--password",
    "--client-password",
    "--test",
];

impl ProxyConfig {
    /// `true` for the all-empty table — lets the serializer skip
    /// `[proxy]` entirely on a config that never set it.
    pub fn is_default(&self) -> bool {
        *self == ProxyConfig::default()
    }

    /// The mode in force: the declared one, else `explicit` when a proxy
    /// URL is set, else none (no proxy; `ca_file` alone still applies).
    /// A table switched to `none` or `explicit` keeps its px keys — the
    /// switch is reversible — and they are simply not used.
    pub fn effective_mode(&self) -> Option<ProxyMode> {
        self.mode.or_else(|| {
            (self.https.is_some() || self.http.is_some()).then_some(ProxyMode::Explicit)
        })
    }

    /// The `(name, value)` pairs the table promotes, for the keys that are
    /// set. The conventional uppercase spellings: `reqwest`, `git` and
    /// `curl` read both cases, and the uppercase one is what a reader
    /// expects to see in a process listing. A managed px has no address
    /// before an operation starts it, so `mode = "px"` promotes nothing.
    pub fn child_env(&self) -> Vec<(&'static str, String)> {
        if matches!(self.effective_mode(), Some(ProxyMode::Px | ProxyMode::None)) {
            return Vec::new();
        }
        [
            ("HTTPS_PROXY", &self.https),
            ("HTTP_PROXY", &self.http),
            ("NO_PROXY", &self.no_proxy),
        ]
        .into_iter()
        .filter_map(|(name, value)| value.clone().map(|value| (name, value)))
        .collect()
    }

    /// This table with every key it leaves unset taken from `lower` — the
    /// user's file over a site profile, key by key.
    ///
    /// ```
    /// use vibe_core::user_config::ProxyConfig;
    /// let site: ProxyConfig = toml::from_str("mode='px'\npac='https://pac.example.corp/p.pac'\nca_file='/etc/corp-ca.pem'").unwrap();
    /// let mine: ProxyConfig = toml::from_str("ca_file='/home/me/ca.pem'").unwrap();
    /// let merged = mine.over(&site);
    /// assert_eq!(merged.pac.as_deref(), Some("https://pac.example.corp/p.pac"));
    /// assert_eq!(merged.ca_file.unwrap().to_str(), Some("/home/me/ca.pem"));
    /// ```
    pub fn over(mut self, lower: &ProxyConfig) -> ProxyConfig {
        self.mode = self.mode.or(lower.mode);
        self.https = self.https.take().or_else(|| lower.https.clone());
        self.http = self.http.take().or_else(|| lower.http.clone());
        self.no_proxy = self.no_proxy.take().or_else(|| lower.no_proxy.clone());
        self.pac = self.pac.take().or_else(|| lower.pac.clone());
        self.upstream = self.upstream.take().or_else(|| lower.upstream.clone());
        if self.px_args.is_empty() {
            self.px_args = lower.px_args.clone();
        }
        self.px_path = self.px_path.take().or_else(|| lower.px_path.clone());
        self.px_port = self.px_port.or(lower.px_port);
        self.ca_file = self.ca_file.take().or_else(|| lower.ca_file.clone());
        self.revocation = self.revocation.or(lower.revocation);
        self
    }

    /// The first inconsistency in the table, as `(key, reason, fix)` — the
    /// shared check behind the site profile's validation and the network
    /// route's construction.
    ///
    /// ```
    /// use vibe_core::user_config::ProxyConfig;
    /// let px: ProxyConfig = toml::from_str("mode='px'").unwrap();
    /// assert_eq!(px.problem().unwrap().0, "proxy.pac");
    /// let owned: ProxyConfig =
    ///     toml::from_str("mode='px'\nupstream='proxy.example.corp:8080'\npx_args=['--port=1']").unwrap();
    /// assert_eq!(owned.problem().unwrap().0, "proxy.px_args");
    /// assert!(ProxyConfig::default().problem().is_none());
    /// ```
    pub fn problem(&self) -> Option<(&'static str, String, &'static str)> {
        for (key, value) in [
            ("proxy.https", self.https.as_deref()),
            ("proxy.http", self.http.as_deref()),
            ("proxy.no_proxy", self.no_proxy.as_deref()),
            ("proxy.pac", self.pac.as_deref()),
            ("proxy.upstream", self.upstream.as_deref()),
        ] {
            if value.is_some_and(|value| value.trim().is_empty()) {
                return Some((key, "it is empty".into(), "give a value or drop the key"));
            }
        }
        for (key, value) in [("proxy.https", &self.https), ("proxy.http", &self.http)] {
            if let Some(value) = value
                && !(value.starts_with("http://") || value.starts_with("https://"))
            {
                return Some((
                    key,
                    "a proxy URL must start with `http://` or `https://`".into(),
                    "write the proxy as `http://host:port`",
                ));
            }
        }
        if self.effective_mode() == Some(ProxyMode::Px) {
            if let Some(pac) = &self.pac
                && let Err(fix) = PacSource::parse(pac)
            {
                return Some(("proxy.pac", format!("`{pac}` is not a PAC location"), fix));
            }
            if self.pac.is_none() && self.upstream.is_none() {
                return Some((
                    "proxy.pac",
                    "`mode = \"px\"` needs a route".into(),
                    "set `pac = \"<your PAC URL>\"` or `upstream = \"<host:port>\"`",
                ));
            }
            if let Some(arg) = self.px_args.iter().find(|arg| {
                let name = arg.split('=').next().unwrap_or(arg);
                PX_OWNED_ARGS.contains(&name)
            }) {
                return Some((
                    "proxy.px_args",
                    format!("`{arg}` is an argument vibe sets itself"),
                    "drop it; vibe chooses the listen address, port, route and lifetime of its px",
                ));
            }
            if self.px_port == Some(0) {
                return Some((
                    "proxy.px_port",
                    "port 0 is not a fixed port".into(),
                    "give the loopback port your security policy allows, or drop `px_port`",
                ));
            }
        }
        None
    }
}
