//! `[tls]` — the certificates this installation trusts in addition to the
//! operating system's own verifier (`PROP-068#proxy`).
//!
//! The operating system decides first: on Windows the CryptoAPI chain
//! engine (every store, including the intermediate `CA` ones, enterprise
//! policy and AIA), on macOS the Security framework, elsewhere the
//! platform's root bundle. `[tls] extra_ca` is **added** to that decision
//! and never replaces it, so naming a corporate CA cannot take the public
//! web away.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy");

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// `[tls]` — extra certificates, always additive.
///
/// ```
/// use vibe_core::user_config::TlsConfig;
/// // One file.
/// let one: TlsConfig = toml::from_str("extra_ca = '/etc/corp-ca.pem'").unwrap();
/// assert_eq!(one.extra_ca_paths(), [std::path::PathBuf::from("/etc/corp-ca.pem")]);
/// // A list of files and directories.
/// let many: TlsConfig = toml::from_str("extra_ca = ['/etc/corp-ca.pem', '/etc/corp-certs']").unwrap();
/// assert_eq!(many.extra_ca_paths().len(), 2);
/// // The empty table is the default and is skipped on write.
/// assert!(TlsConfig::default().is_default());
/// assert!(TlsConfig::default().extra_ca_paths().is_empty());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TlsConfig {
    /// Extra trust anchors and pinned server certificates: one path, or a
    /// list of them. Each entry is either a certificate file (PEM, one or
    /// many certificates, or a single DER) or a **directory**, whose
    /// `*.pem`, `*.crt` and `*.cer` files are all read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_ca: Option<ExtraCa>,
}

/// One path or a list of them — the backward-compatible shape, so a file
/// that already held a single string keeps parsing.
///
/// ```
/// use vibe_core::user_config::ExtraCa;
/// let one: ExtraCa = toml::from_str("x = '/a.pem'").map(|t: toml::Table| t["x"].clone())
///     .and_then(|v| v.try_into()).unwrap();
/// assert_eq!(one.paths().len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ExtraCa {
    /// `extra_ca = "/etc/corp-ca.pem"`.
    One(PathBuf),
    /// `extra_ca = ["/etc/corp-ca.pem", "/etc/corp-certs"]`.
    Many(Vec<PathBuf>),
}

impl ExtraCa {
    /// The paths this value names, in the order written.
    pub fn paths(&self) -> Vec<PathBuf> {
        match self {
            Self::One(path) => vec![path.clone()],
            Self::Many(paths) => paths.clone(),
        }
    }
}

impl TlsConfig {
    /// `true` for the all-empty table — lets the serializer skip `[tls]`
    /// entirely on a config that never set it.
    pub fn is_default(&self) -> bool {
        *self == TlsConfig::default()
    }

    /// The paths `extra_ca` names, in the order written.
    pub fn extra_ca_paths(&self) -> Vec<PathBuf> {
        self.extra_ca
            .as_ref()
            .map(ExtraCa::paths)
            .unwrap_or_default()
    }

    /// This table with every key it leaves unset taken from `lower` — the
    /// user's file over a site profile, key by key, exactly as `[proxy]`
    /// layers.
    ///
    /// ```
    /// use vibe_core::user_config::TlsConfig;
    /// let site: TlsConfig = toml::from_str("extra_ca = '/etc/corp-ca.pem'").unwrap();
    /// let mine: TlsConfig = toml::from_str("extra_ca = ['/home/me/ca.pem']").unwrap();
    /// // The user's key replaces the site's, like every other layered key.
    /// assert_eq!(mine.clone().over(&site).extra_ca_paths(), [std::path::PathBuf::from("/home/me/ca.pem")]);
    /// // An unset key falls through.
    /// assert_eq!(TlsConfig::default().over(&site).extra_ca_paths().len(), 1);
    /// ```
    pub fn over(mut self, lower: &TlsConfig) -> TlsConfig {
        self.extra_ca = self.extra_ca.take().or_else(|| lower.extra_ca.clone());
        self
    }

    /// The first inconsistency in the table, as `(key, reason, fix)`.
    ///
    /// ```
    /// use vibe_core::user_config::TlsConfig;
    /// let empty: TlsConfig = toml::from_str("extra_ca = ''").unwrap();
    /// assert_eq!(empty.problem().unwrap().0, "tls.extra_ca");
    /// assert!(TlsConfig::default().problem().is_none());
    /// ```
    pub fn problem(&self) -> Option<(&'static str, String, &'static str)> {
        for path in self.extra_ca_paths() {
            if path.as_os_str().is_empty() {
                return Some((
                    "tls.extra_ca",
                    "it names an empty path".into(),
                    "give a certificate file or a directory of them, or drop the key",
                ));
            }
        }
        None
    }

    /// The `extra_ca` entries as `(key, path)` pairs, for the site
    /// profile's path validation and its Windows-path notes.
    pub fn declared_paths(&self) -> Vec<(&'static str, PathBuf)> {
        self.extra_ca_paths()
            .into_iter()
            .map(|path| ("tls.extra_ca", path))
            .collect()
    }
}

/// The effective extra-certificate list of one installation: `[tls]
/// extra_ca` together with the `[proxy] ca_file` alias, in that order and
/// without repeats.
///
/// Both keys mean the same thing — *trust these too* — so neither takes
/// the other away: `ca_file` is the spelling that shipped first and keeps
/// working, `extra_ca` is the one that says what it does and takes more
/// than one path.
///
/// ```
/// use std::path::PathBuf;
/// use vibe_core::user_config::{effective_extra_ca, ProxyConfig, TlsConfig};
/// let proxy: ProxyConfig = toml::from_str("ca_file = '/etc/corp-ca.pem'").unwrap();
/// let tls: TlsConfig = toml::from_str("extra_ca = ['/etc/corp-certs', '/etc/corp-ca.pem']").unwrap();
/// // The alias comes first, and the path named twice is listed once.
/// assert_eq!(
///     effective_extra_ca(&tls, &proxy),
///     [PathBuf::from("/etc/corp-ca.pem"), PathBuf::from("/etc/corp-certs")]
/// );
/// ```
pub fn effective_extra_ca(tls: &TlsConfig, proxy: &super::ProxyConfig) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut push = |path: &Path| {
        if !out.iter().any(|seen| seen == path) {
            out.push(path.to_path_buf());
        }
    };
    if let Some(path) = proxy.ca_file.as_deref() {
        push(path);
    }
    for path in tls.extra_ca_paths() {
        push(&path);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_string_still_parses_as_it_always_did() {
        let cfg: TlsConfig = toml::from_str("extra_ca = '/etc/corp-ca.pem'").unwrap();
        assert!(matches!(cfg.extra_ca, Some(ExtraCa::One(_))));
        assert_eq!(cfg.extra_ca_paths(), [PathBuf::from("/etc/corp-ca.pem")]);
    }

    #[test]
    fn a_list_keeps_the_order_it_was_written_in() {
        let cfg: TlsConfig = toml::from_str("extra_ca = ['/b.pem', '/a.pem']").unwrap();
        assert_eq!(
            cfg.extra_ca_paths(),
            [PathBuf::from("/b.pem"), PathBuf::from("/a.pem")]
        );
    }

    #[test]
    fn the_table_round_trips_through_toml() {
        let cfg: TlsConfig = toml::from_str("extra_ca = ['/a.pem', '/certs']").unwrap();
        let text = toml::to_string(&cfg).unwrap();
        assert_eq!(toml::from_str::<TlsConfig>(&text).unwrap(), cfg);
    }

    #[test]
    fn an_unknown_key_is_refused_so_a_typo_is_not_silently_ignored() {
        assert!(toml::from_str::<TlsConfig>("extra_cas = '/a.pem'").is_err());
    }
}
