//! The identity of a transfer (`PROP-068#transfer`): its record, the
//! closed vocabularies the record speaks, and the reduction every URL goes
//! through before anything writes it down (`#redaction`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#transfer");

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use specmark::spec;

use super::clock;

/// The running count behind every transfer id of this process: the one
/// process-wide state the wrapper keeps, since an id must be unique on the
/// machine and the pid alone is not.
static NEXT: AtomicU64 = AtomicU64::new(1);

/// How the bytes are known: read and counted by vibe, or reported by git.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-068#transfer")]
pub enum TransferKind {
    Http,
    Git,
}

/// What the bytes are for — the closed vocabulary of `PROP-068#purpose`.
///
/// ```
/// use vibe_core::progress::transfer::Purpose;
///
/// assert_eq!(Purpose::parse("native-input"), Some(Purpose::NativeInput));
/// assert_eq!(Purpose::ReleaseBundle.as_str(), "release-bundle");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Purpose {
    PackageSource,
    Archive,
    NativeInput,
    PackageClone,
    ReleaseBundle,
    IndexFile,
    Other,
}

impl Purpose {
    const ALL: [Self; 7] = [
        Self::PackageSource,
        Self::Archive,
        Self::NativeInput,
        Self::PackageClone,
        Self::ReleaseBundle,
        Self::IndexFile,
        Self::Other,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::PackageSource => "package-source",
            Self::Archive => "archive",
            Self::NativeInput => "native-input",
            Self::PackageClone => "package-clone",
            Self::ReleaseBundle => "release-bundle",
            Self::IndexFile => "index-file",
            Self::Other => "other",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|purpose| purpose.as_str() == text)
    }
}

/// Where the bytes land.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-068#transfer")]
pub enum Destination {
    Store,
    Cache,
    Work,
    Home,
}

/// `counted` when vibe counted the bytes it wrote, `reported` when it
/// relayed git's figure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-068#transfer")]
pub enum Precision {
    Counted,
    Reported,
}

/// The identity and context of one transfer (`PROP-068#transfer`), plus
/// the command's resume line, which its history entry names only when it
/// was interrupted (`#pause`).
///
/// ```
/// use vibe_core::progress::transfer::{Destination, Purpose, TransferRecord};
///
/// let record = TransferRecord::http(Purpose::Archive, "https://example.test/a.zip", Destination::Cache)
///     .package("org.example/a@1.0.0")
///     .step("unpack")
///     .batch("20261101T100000.000Z");
/// assert!(record.id.starts_with(&format!("{}:", std::process::id())));
/// assert_eq!(record.step.as_deref(), Some("unpack"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-068#transfer")]
pub struct TransferRecord {
    pub id: String,
    pub batch: String,
    pub kind: TransferKind,
    pub url: String,
    pub purpose: Purpose,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    pub destination: Destination,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    pub started: String,
    pub pid: u32,
    pub command: String,
    pub precision: Precision,
    pub limited: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_bps: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remapped_from: Option<String>,
    /// The proxy the connection goes through (`PROP-068#proxy`): its
    /// `scheme://host:port`, or `px` for the vibe-managed px; absent for a
    /// direct connection. Never a credential.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continues: Option<String>,
}

impl TransferRecord {
    /// A counted HTTP transfer from `url`, whose credentials, query and
    /// fragment are dropped here, before anything records it.
    pub fn http(purpose: Purpose, url: &str, destination: Destination) -> Self {
        Self::fresh(TransferKind::Http, safe_url(url), purpose, destination)
    }

    /// A clone or fetch the git binary performs from `remote` (`#git`):
    /// its bytes are git's own report, so the record says `reported`.
    ///
    /// ```
    /// use vibe_core::progress::transfer::{Destination, Precision, Purpose, TransferRecord};
    ///
    /// let record = TransferRecord::git(
    ///     Purpose::PackageClone,
    ///     "https://x-access-token:secret@git.example/org/repo.git",
    ///     Destination::Store,
    /// );
    /// assert_eq!(record.url, "https://git.example/org/repo.git");
    /// assert_eq!(record.precision, Precision::Reported);
    /// ```
    pub fn git(purpose: Purpose, remote: &str, destination: Destination) -> Self {
        Self {
            precision: Precision::Reported,
            ..Self::fresh(TransferKind::Git, safe_remote(remote), purpose, destination)
        }
    }

    fn fresh(kind: TransferKind, url: String, purpose: Purpose, destination: Destination) -> Self {
        let pid = std::process::id();
        let proxy = crate::net::route().label_for(&url);
        Self {
            id: format!("{pid}:{}", NEXT.fetch_add(1, Ordering::Relaxed)),
            batch: pid.to_string(),
            kind,
            url,
            purpose,
            package: None,
            step: None,
            destination,
            total: None,
            started: clock::format_millis(clock::unix_millis()),
            pid,
            command: String::new(),
            precision: Precision::Counted,
            limited: false,
            limit_bps: None,
            remapped_from: None,
            proxy,
            resume: None,
            continues: None,
        }
    }

    pub fn package(self, key: impl Into<String>) -> Self {
        Self {
            package: Some(key.into()),
            ..self
        }
    }

    pub fn step(self, id: impl Into<String>) -> Self {
        Self {
            step: Some(id.into()),
            ..self
        }
    }

    /// The run the command belongs to; the pid when there is none.
    pub fn batch(self, batch: impl Into<String>) -> Self {
        Self {
            batch: batch.into(),
            ..self
        }
    }

    pub fn total(self, total: Option<u64>) -> Self {
        Self { total, ..self }
    }

    /// The URL the manifest named, when a remap row fetched another.
    pub fn remapped_from(self, declared: &str) -> Self {
        Self {
            remapped_from: Some(safe_url(declared)),
            ..self
        }
    }

    /// The command line that resumes the command if it is interrupted.
    pub fn resume(self, line: impl Into<String>) -> Self {
        Self {
            resume: Some(line.into()),
            ..self
        }
    }

    /// `<pid>-<n>`: the id spelled for a file name.
    pub(crate) fn file_stem(&self) -> String {
        self.id.replace(':', "-")
    }

    /// The label of the transfer's task: the last part of the URL.
    pub(super) fn label(&self) -> String {
        let name = self
            .url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .filter(|name| !name.is_empty())
            .unwrap_or("data");
        format!("Downloading {name}")
    }
}

/// `user`, `password`, query and fragment removed — never a token in a
/// record, a journal or a progress line (`PROP-068#redaction`).
///
/// ```
/// use vibe_core::progress::transfer::safe_url;
///
/// assert_eq!(
///     safe_url("https://name:secret@example.test/file?token=secret#fragment"),
///     "https://example.test/file"
/// );
/// assert_eq!(safe_url("not a url"), "remote endpoint");
/// ```
pub fn safe_url(raw: &str) -> String {
    let Ok(mut url) = url::Url::parse(raw) else {
        return "remote endpoint".to_owned();
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

/// A git remote reduced like [`safe_url`]: a URL loses its user, password,
/// query and fragment, an scp-like `user@host:path` its user, and a local
/// path stays as it is, since it carries no credential and is the only name
/// the remote has.
///
/// ```
/// use vibe_core::progress::transfer::safe_remote;
///
/// assert_eq!(safe_remote("git+file:///srv/org.example.tool.git"), "file:///srv/org.example.tool.git");
/// assert_eq!(safe_remote("git@git.example:org/repo.git"), "git.example:org/repo.git");
/// assert_eq!(safe_remote("/srv/repo.git"), "/srv/repo.git");
/// ```
pub fn safe_remote(raw: &str) -> String {
    let raw = raw.strip_prefix("git+").unwrap_or(raw);
    // A one-letter scheme is a Windows drive (`C:/repo`), not a URL.
    if url::Url::parse(raw).is_ok_and(|url| url.scheme().len() > 1) {
        return safe_url(raw);
    }
    match raw.split_once('@') {
        Some((_, rest)) if rest.contains(':') => rest.to_owned(),
        _ => raw.to_owned(),
    }
}
