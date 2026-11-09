//! The `DefaultConnectionSettings` blob and the Windows proxy strings
//! (`PROP-068#proxy-system`), as pure functions: the layout Windows
//! writes under the user's `Connections` key, and the two reductions that
//! turn its proxy and bypass strings into what px and vibe read.

/// `DefaultConnectionSettings` flag: connect directly.
pub const CONNECTION_DIRECT: u32 = 0x1;
/// `DefaultConnectionSettings` flag: use the proxy server named below.
pub const CONNECTION_PROXY: u32 = 0x2;
/// `DefaultConnectionSettings` flag: use the auto-config URL named below.
pub const CONNECTION_AUTO_CONFIG_URL: u32 = 0x4;
/// `DefaultConnectionSettings` flag: auto-detect (WPAD).
pub const CONNECTION_AUTO_DETECT: u32 = 0x8;

/// The settings one `DefaultConnectionSettings` blob carries.
///
/// ```
/// use vibe_core::net::{CONNECTION_AUTO_DETECT, CONNECTION_PROXY, ConnectionSettings};
/// let settings = ConnectionSettings {
///     version: 0x46,
///     counter: 9,
///     flags: CONNECTION_PROXY | CONNECTION_AUTO_DETECT,
///     proxy: Some("proxy.example.corp:8080".into()),
///     ..Default::default()
/// };
/// // The flag word is read through the three questions it answers.
/// assert!(settings.proxy_enabled() && settings.auto_detect());
/// assert!(!settings.auto_config_url_enabled());
/// assert_eq!(ConnectionSettings::default().flags, 0);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectionSettings {
    /// The format version the blob declares (`0x46` on Windows 10/11).
    pub version: u32,
    /// The change counter Windows bumps on every edit.
    pub counter: u32,
    /// The flag word: [`CONNECTION_DIRECT`], [`CONNECTION_PROXY`],
    /// [`CONNECTION_AUTO_CONFIG_URL`], [`CONNECTION_AUTO_DETECT`].
    pub flags: u32,
    /// The proxy server string, as Windows spells it.
    pub proxy: Option<String>,
    /// The bypass list, as Windows spells it.
    pub bypass: Option<String>,
    /// The auto-config (PAC) URL.
    pub auto_config_url: Option<String>,
}

impl ConnectionSettings {
    /// Whether [`CONNECTION_PROXY`] is set.
    pub fn proxy_enabled(&self) -> bool {
        self.flags & CONNECTION_PROXY != 0
    }
    /// Whether [`CONNECTION_AUTO_CONFIG_URL`] is set.
    pub fn auto_config_url_enabled(&self) -> bool {
        self.flags & CONNECTION_AUTO_CONFIG_URL != 0
    }
    /// Whether [`CONNECTION_AUTO_DETECT`] is set.
    pub fn auto_detect(&self) -> bool {
        self.flags & CONNECTION_AUTO_DETECT != 0
    }
}

/// Parse `HKCU\…\Internet Settings\Connections` value
/// `DefaultConnectionSettings` (`REG_BINARY`): little-endian `u32`
/// version, `u32` change counter, `u32` flags, then three
/// length-prefixed ASCII strings — the proxy server, the bypass list and
/// the auto-config URL. Anything after them (the per-connection GUID and
/// reserved words) is not read; a blob that stops early yields the fields
/// it carried.
///
/// ```
/// use vibe_core::net::parse_connection_settings;
/// // flags 0x0D = direct | auto-config URL | auto-detect, URL in the blob
/// // and no `AutoConfigURL` value — the managed-desktop shape px misses.
/// let mut blob = Vec::new();
/// blob.extend_from_slice(&0x46u32.to_le_bytes());
/// blob.extend_from_slice(&9u32.to_le_bytes());
/// blob.extend_from_slice(&0x0du32.to_le_bytes());
/// blob.extend_from_slice(&0u32.to_le_bytes());
/// blob.extend_from_slice(&0u32.to_le_bytes());
/// let url = b"https://pac.example.corp/proxy.pac";
/// blob.extend_from_slice(&(url.len() as u32).to_le_bytes());
/// blob.extend_from_slice(url);
/// blob.extend_from_slice(&[0; 32]);
///
/// let settings = parse_connection_settings(&blob).unwrap();
/// assert_eq!(settings.flags, 0x0d);
/// assert!(settings.auto_config_url_enabled() && settings.auto_detect());
/// assert!(!settings.proxy_enabled());
/// assert_eq!(
///     settings.auto_config_url.as_deref(),
///     Some("https://pac.example.corp/proxy.pac")
/// );
/// assert_eq!(settings.proxy, None);
///
/// assert!(parse_connection_settings(&[0, 1, 2]).is_err());
/// ```
pub fn parse_connection_settings(bytes: &[u8]) -> Result<ConnectionSettings, String> {
    let mut at = 0usize;
    let word = |at: &mut usize| -> Option<u32> {
        let slice = bytes.get(*at..*at + 4)?;
        *at += 4;
        Some(u32::from_le_bytes(slice.try_into().ok()?))
    };
    let version = word(&mut at).ok_or_else(|| {
        format!(
            "{} bytes is too short to be a connection-settings blob (12 bytes of header)",
            bytes.len()
        )
    })?;
    let counter = word(&mut at).ok_or_else(|| "it stops before the change counter".to_owned())?;
    let flags = word(&mut at).ok_or_else(|| "it stops before the flag word".to_owned())?;
    let mut settings = ConnectionSettings {
        version,
        counter,
        flags,
        ..ConnectionSettings::default()
    };
    // The three strings, in the blob's order. A blob may stop after any
    // of them; a length that runs past the end is a malformed blob.
    for field in 0..3u8 {
        let Some(length) = word(&mut at) else { break };
        let length = length as usize;
        let Some(text) = bytes.get(at..at + length) else {
            return Err(format!(
                "its {} field declares {length} bytes but only {} remain",
                ["proxy server", "bypass list", "auto-config URL"][field as usize],
                bytes.len().saturating_sub(at)
            ));
        };
        at += length;
        let text = String::from_utf8_lossy(text).trim().to_owned();
        let value = (!text.is_empty()).then_some(text);
        match field {
            0 => settings.proxy = value,
            1 => settings.bypass = value,
            _ => settings.auto_config_url = value,
        }
    }
    Ok(settings)
}

/// The `host:port` px is given as `--proxy`, out of a Windows proxy
/// server string: a bare `host:port`, or per-scheme entries
/// `http=host:port;https=host:port`. The `https` entry is preferred (px
/// tunnels HTTPS by `CONNECT`), then `http`, then a bare entry; a
/// `socks` or `ftp` entry is not one px can use and is left out. A
/// scheme prefix on the value (`http://host:port`) is dropped — px wants
/// the address alone.
///
/// ```
/// use vibe_core::net::pick_upstream;
/// assert_eq!(pick_upstream("proxy.example.corp:8080").as_deref(), Some("proxy.example.corp:8080"));
/// assert_eq!(
///     pick_upstream("http=a.example.corp:3128;https=b.example.corp:3129").as_deref(),
///     Some("b.example.corp:3129")
/// );
/// assert_eq!(
///     pick_upstream("ftp=f.example.corp:21;http=a.example.corp:3128").as_deref(),
///     Some("a.example.corp:3128")
/// );
/// assert_eq!(pick_upstream("http://proxy.example.corp:8080").as_deref(), Some("proxy.example.corp:8080"));
/// assert_eq!(pick_upstream("socks=s.example.corp:1080"), None);
/// assert_eq!(pick_upstream("   "), None);
/// ```
pub fn pick_upstream(raw: &str) -> Option<String> {
    let strip = |value: &str| {
        let value = value.trim();
        let value = value
            .strip_prefix("http://")
            .or_else(|| value.strip_prefix("https://"))
            .unwrap_or(value);
        let value = value.trim_end_matches('/').trim();
        (!value.is_empty()).then(|| value.to_owned())
    };
    let mut https = None;
    let mut http = None;
    let mut bare = None;
    for entry in raw.split(';').map(str::trim).filter(|e| !e.is_empty()) {
        match entry.split_once('=') {
            Some((scheme, value)) => {
                let slot = match scheme.trim().to_ascii_lowercase().as_str() {
                    "https" => &mut https,
                    "http" => &mut http,
                    // `socks` and `ftp` are not routes px can take.
                    _ => continue,
                };
                if slot.is_none() {
                    *slot = strip(value);
                }
            }
            None if bare.is_none() => bare = strip(entry),
            None => {}
        }
    }
    https.or(http).or(bare)
}

/// A Windows bypass list in vibe's `no_proxy` spelling: `;`-separated
/// becomes comma-separated, `<local>` is dropped (loopback and
/// dotless-name handling is vibe's own — loopback always bypasses), and
/// every other entry travels as written. `vibe_core::net::Bypass` reads
/// `*.corp.example` and a CIDR range; an entry it cannot read (Windows'
/// `172.16.*`) it ignores rather than widening, and px reads the same
/// list in its own `--noproxy` spelling.
///
/// ```
/// use vibe_core::net::bypass_list;
/// assert_eq!(
///     bypass_list("<local>;*.corp.example;10.0.0.0/8").as_deref(),
///     Some("*.corp.example,10.0.0.0/8")
/// );
/// assert_eq!(bypass_list("<local>"), None);
/// assert_eq!(bypass_list(""), None);
/// ```
pub fn bypass_list(raw: &str) -> Option<String> {
    let mut out: Vec<&str> = Vec::new();
    for entry in raw
        .split(|c: char| c == ';' || c == ',' || c.is_whitespace())
        .map(str::trim)
        .filter(|entry| !entry.is_empty() && !entry.eq_ignore_ascii_case("<local>"))
    {
        if !out.iter().any(|kept| kept.eq_ignore_ascii_case(entry)) {
            out.push(entry);
        }
    }
    (!out.is_empty()).then(|| out.join(","))
}
