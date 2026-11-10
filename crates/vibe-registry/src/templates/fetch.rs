//! Fetching a template's two external documents — and nothing else.
//!
//! Every fetch here is an ordinary transfer through the operation's
//! controlled transport and the remap Locator (PROP-075
//! `#TEMPLATE-CACHE`, PROP-066 `#locator`), so a closed network
//! redirects a template's inputs with the rows it already uses and
//! `vibe remap explain` answers for them. The two artifact kinds are
//! [`ArtifactKind::TemplateIndex`] and [`ArtifactKind::TemplateDigest`]
//! (AV-FMT-004) — two vocabulary members, no new routing mechanism and
//! no new config table.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CACHE");

use std::time::Duration;

use vibe_core::remap::{Artifact, ArtifactKind};

use super::error::TemplateError;
use crate::http_admission::ManagedHttpClient;

mod local;
pub use local::local_file;

/// How long a template-document read waits. The same ten seconds the
/// index client gives a file read: these are small documents on a
/// release host, and a template that cannot answer in ten seconds is a
/// template a user should hear about rather than wait for.
const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// Which of the two documents is being read — the word every refusal and
/// every cache message uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Document {
    /// The version list.
    VersionList,
    /// One version's digest document.
    Digests,
}

impl Document {
    /// The artifact kind this document is fetched as.
    ///
    /// ```
    /// use vibe_core::remap::ArtifactKind;
    /// use vibe_registry::templates::Document;
    ///
    /// assert_eq!(Document::VersionList.kind(), ArtifactKind::TemplateIndex);
    /// assert_eq!(Document::Digests.kind(), ArtifactKind::TemplateDigest);
    /// ```
    pub fn kind(self) -> ArtifactKind {
        match self {
            Document::VersionList => ArtifactKind::TemplateIndex,
            Document::Digests => ArtifactKind::TemplateDigest,
        }
    }

    /// The noun a message names it by.
    ///
    /// ```
    /// use vibe_registry::templates::Document;
    ///
    /// assert_eq!(Document::VersionList.label(), "version list");
    /// assert_eq!(Document::Digests.label(), "digest document");
    /// ```
    pub fn label(self) -> &'static str {
        match self {
            Document::VersionList => "version list",
            Document::Digests => "digest document",
        }
    }
}

/// Where a template document is actually read from, and how.
///
/// Implemented once for production ([`HttpTransport`]) and once per test
/// over a local directory, which is what lets every CLI test of the
/// template layer run with no socket at all — the rule the campaign's
/// own constraint states and `tests/` holds.
/// ```
/// use vibe_registry::templates::{Document, Fetched, TemplateError, TemplateTransport};
///
/// /// A transport over one document held in memory — the shape every
/// /// offline test of the template layer uses.
/// struct Served(&'static str, &'static str);
/// impl TemplateTransport for Served {
///     fn read(
///         &self,
///         document: Document,
///         package: &str,
///         url: &str,
///     ) -> Result<Fetched, TemplateError> {
///         if url == self.0 {
///             return Ok(Fetched { chosen_url: url.to_string(), body: self.1.to_string() });
///         }
///         Err(TemplateError::Fetch {
///             package: package.to_string(),
///             what: document.label().to_string(),
///             url: url.to_string(),
///             reason: "no such document".to_string(),
///         })
///     }
/// }
///
/// let transport: &dyn TemplateTransport = &Served("https://e.invalid/i.json", "[]");
/// let fetched = transport.read(Document::VersionList, "org.e/tool", "https://e.invalid/i.json")?;
/// assert_eq!(fetched.body, "[]");
/// let refusal = transport
///     .read(Document::Digests, "org.e/tool", "https://e.invalid/other")
///     .unwrap_err();
/// assert_eq!(refusal.code(), "template_fetch_failed");
/// # Ok::<(), vibe_registry::templates::TemplateError>(())
/// ```
pub trait TemplateTransport {
    /// Read one document. `url` is the **declared** URL; the
    /// implementation passes it through the Locator itself, because the
    /// chosen URL is what the cache envelope and the lock record must
    /// carry.
    fn read(&self, document: Document, package: &str, url: &str) -> Result<Fetched, TemplateError>;
}

/// One fetched document and the URL it actually came from.
///
/// ```
/// use vibe_registry::templates::Fetched;
///
/// let fetched = Fetched {
///     chosen_url: "https://mirror.corp/index.json".to_string(),
///     body: "[]".to_string(),
/// };
/// assert_eq!(fetched.body, "[]");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    /// The URL after remap — what the envelope records and the lock
    /// record's `source_url` / `digest_url` carry.
    pub chosen_url: String,
    /// The document's bytes as text.
    pub body: String,
}

/// The production transport: the operation's controlled HTTP client,
/// with the declared URL passed through the active Locator first.
#[derive(Debug, Default, Clone)]
pub struct HttpTransport;

impl TemplateTransport for HttpTransport {
    fn read(&self, document: Document, package: &str, url: &str) -> Result<Fetched, TemplateError> {
        let chosen = choose_url(document, package, url)?;
        let failure = |reason: String| TemplateError::Fetch {
            package: package.to_string(),
            what: document.label().to_string(),
            url: chosen.clone(),
            reason,
        };
        // A `[[remap]]` row of a closed network puts these documents on
        // this filesystem (PROP-066 `#locator`), and a mirror snapshot is
        // exactly that: read them off disk rather than hand a `file://`
        // or a bare path to an HTTP client, which cannot take one. The
        // declaration itself stays the credential-free HTTPS URL
        // `#TEMPLATE-DIGEST-SOURCE` requires — a remap changes where the
        // bytes are found, never what the package declared.
        if let Some(path) = local_file(&chosen, url) {
            let body = local::read(&path).map_err(failure)?;
            return Ok(Fetched {
                chosen_url: chosen,
                body,
            });
        }
        let client =
            ManagedHttpClient::legacy(FETCH_TIMEOUT).map_err(|error| TemplateError::Fetch {
                package: package.to_string(),
                what: document.label().to_string(),
                url: chosen.clone(),
                reason: error.to_string(),
            })?;
        let request = client.get(&chosen);
        let response = crate::http_admission::get(&client, request, &chosen, None)
            .map_err(|error| failure(error.to_string()))?;
        if !response.status().is_success() {
            return Err(failure(format!("HTTP {}", response.status().as_u16())));
        }
        let body = response
            .text()
            .map_err(|error| failure(error.to_string()))?;
        Ok(Fetched {
            chosen_url: chosen,
            body,
        })
    }
}

/// The declared URL after the Locator has had its say. A refusal is
/// `remap_denied`, never a silent fall-through to the origin: a closed
/// network that denies a host means it.
pub fn choose_url(document: Document, package: &str, url: &str) -> Result<String, TemplateError> {
    let locator = vibe_core::remap::active();
    if locator.is_empty() {
        return Ok(url.to_string());
    }
    let artifact = Artifact::new(document.kind())
        .with_url(url)
        .with_package(package);
    locator
        .choose_url(&artifact)
        .map_err(|refusal| TemplateError::RemapDenied {
            package: package.to_string(),
            what: document.label().to_string(),
            url: url.to_string(),
            reason: refusal.to_string(),
        })
}
