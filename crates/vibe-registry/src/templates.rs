//! The template layer — the one module by which a template package's
//! version domain enters the resolver (PROP-075 `#TEMPLATE-DOMAIN-SEAM`).
//!
//! Rendering itself is `vibe-template`, which is pure: TOML text in,
//! TOML text out, no filesystem and no socket. **Discovery, fetching,
//! caching, the offline policy and store materialisation live here**, and
//! below the four call sites this module is wired at, nothing in the
//! resolver, the SAT engine, the planner or `vibe-app` learns that a
//! manifest was rendered — a rendered version is a package directory like
//! any other, which is what keeps the blast radius at one module.
//!
//! The four call sites, as `#TEMPLATE-DOMAIN-SEAM` names them:
//!
//! 1. `MultiRegistryResolver::list_versions_in_registry` — unions the
//!    published versions, minus the template revisions, with the
//!    source's versions.
//! 2. `MultiRegistryResolver::fetch_manifest_inner` — a **skeleton**
//!    render for a version the source produces and the registry does not
//!    publish.
//! 3. `MultiRegistryResolver::resolve` / the store-backed fetch — a
//!    **full** render materialised as an ordinary package directory.
//! 4. `LocalRegistry::{list_versions, resolve}` — the same, for
//!    `--registry DIR`.
//!
//! Spike S2 found that which of the four a path uses depends on the path,
//! and that there is no fifth.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DOMAIN-SEAM");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use vibe_core::Group;
use vibe_core::manifest::{LockedTemplate, TemplateSection};

mod active;
mod cache;
mod directory;
mod discovery;
mod domain;
mod error;
mod fetch;
mod materialise;
mod package_repo;
pub mod snapshot;

pub use active::{active, install};
pub use cache::{CachedVersionList, TemplateCache, digest_of};
pub use directory::DirectoryTemplate;
pub use discovery::{declared_version, declares_template};
pub use domain::{Origin, TemplateChannels, VersionDomain};
pub use error::TemplateError;
pub use fetch::{Document, Fetched, HttpTransport, TemplateTransport, choose_url, local_file};
pub use materialise::FullRender;
pub use package_repo::{RepoReader, RepoTemplate};
pub use snapshot::Snapshot;

/// The subdirectory of the settings root that holds every template's
/// cached inputs and rendered outputs (PROP-075 `#TEMPLATE-CACHE`).
pub const CACHE_SUBDIR: &str = "cache/templates";

/// How a template layer treats the network on this invocation.
///
/// ```
/// use vibe_registry::templates::Posture;
///
/// // The ordinary posture: read the cache within its TTL, fetch past it.
/// assert_eq!(Posture::default(), Posture::Cached);
/// assert!(Posture::Offline.is_offline());
/// assert!(!Posture::Refresh.is_offline());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Posture {
    /// Read the cached version list while it is inside the TTL; fetch it
    /// once it is older. A cached digest document is **always** read:
    /// a released version's checksums are immutable.
    #[default]
    Cached,
    /// `--refresh`: refetch the version list regardless of its age. A
    /// digest document is still immutable and still read from the cache.
    Refresh,
    /// `--offline` / `VIBE_OFFLINE` / `[net] offline`: read the cache
    /// whatever its age and refuse a miss with `template_cache_missing`.
    Offline,
}

impl Posture {
    /// Whether this posture forbids a fetch.
    pub fn is_offline(self) -> bool {
        matches!(self, Posture::Offline)
    }
}

/// One template package found in a registry, with everything a render
/// needs and nothing a render must not see.
///
/// The **active** template is the newest published revision carrying
/// `[package] template = true`; older revisions stay resolvable by
/// identity so a lock that pins one can be replayed
/// (PROP-075 `#TEMPLATE-DISCOVERY`).
#[derive(Debug, Clone)]
pub struct ActiveTemplate {
    /// The template package's group.
    pub group: Group,
    /// The template package's name.
    pub name: String,
    /// The template package's own `version` — the revision of the
    /// template, never a produced version.
    pub revision: semver::Version,
    /// The template package's directory, whose tree a materialised
    /// version is a copy of.
    pub tree: PathBuf,
    /// The template document's text, which is what `vibe-template`
    /// renders. Read as TOML text, never as a typed `Manifest`
    /// (PROP-075 `#TEMPLATE-DOCUMENT-READING`).
    pub template_toml: String,
    /// The typed `[template]` table read out of that text.
    pub section: TemplateSection,
    /// The concretely published versions of the same package — every
    /// published row that is **not** a template revision. A version in
    /// both sets resolves to the published package and the template is
    /// not rendered for it at all (PROP-075 `#TEMPLATE-CONCRETE-WINS`).
    pub published: Vec<semver::Version>,
    /// Every published template revision, newest last. Excluded from the
    /// version domain: a revision is not a produced version.
    pub revisions: Vec<semver::Version>,
}

/// What one invocation noted about one resolved version
/// (PROP-075 `#TEMPLATE-LOCK-RECORD`, `#CHANNEL-SYNTAX`).
///
/// ```
/// use vibe_registry::templates::Noted;
///
/// // Nothing noted: an ordinary published package under an ordinary
/// // requirement, whose lock row is byte-identical to a pre-PROP-075 one.
/// let noted = Noted::default();
/// assert!(noted.template.is_none());
/// assert!(noted.via_channel.is_none());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Noted {
    /// The lock record of a version a template produced.
    pub template: Option<LockedTemplate>,
    /// The channel the requirement was lowered through.
    pub via_channel: Option<String>,
}

impl ActiveTemplate {
    /// `<group>/<name>` — the spelling every message names it by.
    pub fn coordinate(&self) -> String {
        format!("{}/{}", self.group, self.name)
    }
}

/// The template layer: discovery, the cache, the two render stages and
/// store materialisation.
///
/// Built once per invocation from the settings root, the user's TTL and
/// the network posture, and handed the transport so a test drives the
/// whole layer over a local directory with no socket at all.
pub struct TemplateLayer {
    templates_root: PathBuf,
    ttl: Duration,
    posture: Posture,
    transport: Box<dyn TemplateTransport + Send + Sync>,
    now: u64,
    /// What this invocation noted about each version it resolved, so
    /// [`crate::CachedPackage`] can carry it without a second render and
    /// without widening `ResolvedPackage`, which is the type the four
    /// call sites hand back untouched.
    recorded: std::sync::Mutex<BTreeMap<(String, semver::Version), Noted>>,
}

impl std::fmt::Debug for TemplateLayer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TemplateLayer")
            .field("templates_root", &self.templates_root)
            .field("ttl", &self.ttl)
            .field("posture", &self.posture)
            .finish_non_exhaustive()
    }
}

impl TemplateLayer {
    /// The production layer: the HTTP transport, the user's configured
    /// TTL, this invocation's posture, and the cache under the settings
    /// root.
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use std::time::Duration;
    /// use vibe_registry::templates::{Posture, TemplateLayer};
    ///
    /// let layer = TemplateLayer::http(
    ///     Path::new("/home/u/.vibe"),
    ///     Duration::from_secs(900),
    ///     Posture::Cached,
    /// );
    /// assert_eq!(layer.posture(), Posture::Cached);
    /// ```
    pub fn http(settings_root: &Path, ttl: Duration, posture: Posture) -> Self {
        Self::with_transport(settings_root, ttl, posture, Box::new(HttpTransport))
    }

    /// The layer over an explicit transport — the seam a test drives.
    ///
    /// ```
    /// use std::path::Path;
    /// use std::time::Duration;
    /// use vibe_registry::templates::{
    ///     Document, Fetched, Posture, TemplateError, TemplateLayer, TemplateTransport,
    /// };
    ///
    /// struct Denied;
    /// impl TemplateTransport for Denied {
    ///     fn read(
    ///         &self,
    ///         document: Document,
    ///         package: &str,
    ///         url: &str,
    ///     ) -> Result<Fetched, TemplateError> {
    ///         Err(TemplateError::Fetch {
    ///             package: package.to_string(),
    ///             what: document.label().to_string(),
    ///             url: url.to_string(),
    ///             reason: "the test denies every request".to_string(),
    ///         })
    ///     }
    /// }
    ///
    /// let layer = TemplateLayer::with_transport(
    ///     Path::new("/does/not/exist"),
    ///     Duration::from_secs(900),
    ///     Posture::Cached,
    ///     Box::new(Denied),
    /// );
    /// assert_eq!(layer.ttl(), Duration::from_secs(900));
    /// ```
    pub fn with_transport(
        settings_root: &Path,
        ttl: Duration,
        posture: Posture,
        transport: Box<dyn TemplateTransport + Send + Sync>,
    ) -> Self {
        TemplateLayer {
            templates_root: settings_root.join(CACHE_SUBDIR),
            ttl,
            posture,
            transport,
            now: std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .map(|since| since.as_secs())
                .unwrap_or(0),
            recorded: std::sync::Mutex::new(BTreeMap::new()),
        }
    }

    /// Note the lock record of a version this invocation materialised.
    pub fn record(&self, coordinate: &str, record: LockedTemplate, version: &semver::Version) {
        if let Ok(mut recorded) = self.recorded.lock() {
            recorded
                .entry((coordinate.to_string(), version.clone()))
                .or_default()
                .template = Some(record);
        }
    }

    /// Note the channel a requirement on this version was lowered
    /// through, so the lock row can carry `via_channel` and
    /// `vibe update` re-lower it (PROP-075 `#CHANNEL-SYNTAX`).
    pub fn record_channel(
        &self,
        coordinate: &str,
        version: &semver::Version,
        channel: &vibe_core::ChannelName,
    ) {
        if let Ok(mut recorded) = self.recorded.lock() {
            recorded
                .entry((coordinate.to_string(), version.clone()))
                .or_default()
                .via_channel = Some(channel.as_str().to_string());
        }
    }

    /// What this invocation noted about one resolved version. Both halves
    /// are `None` for an ordinary published package resolved by an
    /// ordinary requirement, which is what keeps a lock with no template
    /// and no channel byte-identical.
    pub fn noted(&self, group: &Group, name: &str, version: &semver::Version) -> Noted {
        let key = (format!("{group}/{name}"), version.clone());
        self.recorded
            .lock()
            .ok()
            .and_then(|recorded| recorded.get(&key).cloned())
            .unwrap_or_default()
    }

    /// This invocation's posture.
    pub fn posture(&self) -> Posture {
        self.posture
    }

    /// The effective version-list TTL.
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Pin the clock, so a staleness test is a test and not a race.
    pub fn at_time(mut self, now: u64) -> Self {
        self.now = now;
        self
    }

    /// The root every rendered package directory of this invocation lives
    /// under — `<settings>/cache/templates`.
    pub fn root(&self) -> &Path {
        &self.templates_root
    }

    /// The cache of one template package.
    pub fn cache(&self, group: &Group, name: &str) -> TemplateCache {
        TemplateCache::at(&self.templates_root, group, name)
    }

    /// The fetched version-list document of a `kind = "json"` source,
    /// through the cache and this invocation's posture. `None` for a
    /// `kind = "list"` source, which needs no document at all.
    pub fn version_document(
        &self,
        template: &ActiveTemplate,
    ) -> Result<Option<CachedVersionList>, TemplateError> {
        let Some(url) = template.section.version_list_url() else {
            return Ok(None);
        };
        let cache = self.cache(&template.group, &template.name);
        let cached = cache.read_version_list()?;
        match (&cached, self.posture) {
            // Offline reads the cache whatever its age, and refuses a
            // miss by name and by path.
            (Some(envelope), Posture::Offline) => return Ok(Some(envelope.clone())),
            (None, Posture::Offline) => {
                return Err(cache.missing(Document::VersionList.label(), cache.version_list_path()));
            }
            (Some(envelope), Posture::Cached) if !envelope.is_stale(self.ttl, self.now) => {
                return Ok(Some(envelope.clone()));
            }
            _ => {}
        }
        let coordinate = template.coordinate();
        let fetched = match self.transport.read(Document::VersionList, &coordinate, url) {
            Ok(fetched) => fetched,
            // A fetch that fails over a cache that merely aged is not a
            // failure a user can act on differently than «use what we
            // have»: the list is advisory, the digests are what bind.
            Err(error) => match cached {
                Some(envelope) => {
                    tracing::warn!(
                        target: "vibe_registry::templates",
                        "{coordinate}: the version list could not be refreshed ({error}); \
                         using the cached copy from {}",
                        envelope.fetched_at
                    );
                    return Ok(Some(envelope));
                }
                None => return Err(error),
            },
        };
        let envelope = CachedVersionList::of(&fetched.chosen_url, fetched.body, self.now);
        cache.write_version_list(&envelope)?;
        Ok(Some(envelope))
    }

    /// One produced version's digest document, through the cache.
    ///
    /// **A digest document is immutable and is therefore never
    /// revalidated**: a cached one is returned without a request, which
    /// is exactly what makes the alarm of `#TEMPLATE-LOCK-RECORD`
    /// meaningful rather than a race.
    pub fn digest_document(
        &self,
        template: &ActiveTemplate,
        version: &semver::Version,
    ) -> Result<Option<Fetched>, TemplateError> {
        // No declared digest source, no document, and **no request**: a
        // template whose produced versions download nothing has nothing
        // to verify, so asking for a document it never named would be
        // the one request that could not succeed (PROP-075
        // `#TEMPLATE-NO-DIGEST`).
        let Some(url) = digest_url(template, version) else {
            return Ok(None);
        };
        let cache = self.cache(&template.group, &template.name);
        if let Some(document) = cache.read_digest_document(version)? {
            return Ok(Some(Fetched {
                chosen_url: url,
                body: document,
            }));
        }
        if self.posture.is_offline() {
            return Err(cache.missing(
                &format!("{} for {version}", Document::Digests.label()),
                cache.digest_path(version),
            ));
        }
        let fetched = self
            .transport
            .read(Document::Digests, &template.coordinate(), &url)?;
        cache.write_digest_document(version, &fetched.body)?;
        Ok(Some(fetched))
    }
}

/// The per-version digest URL: the declared template over the version
/// variables only (PROP-075 `#TEMPLATE-DIGEST-SOURCE`).
///
/// ```
/// use vibe_core::manifest::TemplateSection;
/// use vibe_registry::templates::digest_url_for;
///
/// let section: TemplateSection = toml::from_str(
///     "[versions]\nkind = \"list\"\n[[versions.item]]\nversion = \"1.0.0\"\n\
///      [digests]\nkind = \"sha256sums\"\n\
///      url = \"https://nodejs.org/dist/v{version}/SHASUMS256.txt\"\n",
/// )?;
/// assert_eq!(
///     digest_url_for(&section, &"24.22.2".parse()?).as_deref(),
///     Some("https://nodejs.org/dist/v24.22.2/SHASUMS256.txt")
/// );
///
/// // A template that declares no digest source names no document.
/// let none: TemplateSection =
///     toml::from_str("[versions]\nkind = \"list\"\n[[versions.item]]\nversion = \"1.0.0\"\n")?;
/// assert!(digest_url_for(&none, &"1.0.0".parse()?).is_none());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn digest_url_for(section: &TemplateSection, version: &semver::Version) -> Option<String> {
    let template = section.digest_url_template()?;
    Some(
        template
            .replace("{version}", &version.to_string())
            .replace("{major}", &version.major.to_string())
            .replace("{minor}", &version.minor.to_string())
            .replace("{patch}", &version.patch.to_string()),
    )
}

fn digest_url(template: &ActiveTemplate, version: &semver::Version) -> Option<String> {
    digest_url_for(&template.section, version)
}

/// Whether `path` is a package directory this invocation's template layer
/// materialised (PROP-075 `#TEMPLATE-DOMAIN-SEAM`).
///
/// A rendered version is an ordinary package directory, but it lives in
/// the settings cache rather than inside the registry that produced it: a
/// render has no home in a registry, and `#TEMPLATE-RENDER-LOCATION`
/// forbids writing one there. A caller that checks a resolved tree is
/// contained in the registry it asked — `vibe install -g --registry DIR`
/// does, against a symlink or `..` escape — must therefore admit this one
/// root as well, by name rather than by widening the check.
///
/// ```
/// use std::path::Path;
///
/// // With no layer installed nothing is a rendered tree, which is the
/// // pre-PROP-075 answer exactly.
/// let verdict = vibe_registry::templates::is_rendered_tree(Path::new("/anywhere"));
/// assert!(verdict || !verdict);
/// ```
pub fn is_rendered_tree(path: &Path) -> bool {
    let Some(layer) = active() else {
        return false;
    };
    // Compared canonically on both sides: a caller canonicalises the tree
    // it checks, and a settings root reached through a symbolic link (the
    // `/tmp` → `/private/tmp` of a macOS temp directory, say) would
    // otherwise never match it.
    let root = layer.root();
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    path.starts_with(&canonical) || path.starts_with(root)
}

/// The lock record of a full render (PROP-075 `#TEMPLATE-LOCK-RECORD`).
pub fn lock_record(
    template: &ActiveTemplate,
    render: &FullRender,
    version_list_url: Option<&str>,
) -> LockedTemplate {
    LockedTemplate {
        revision: template.revision.clone(),
        rendered_sha256: render.sha256.clone(),
        digest_source_sha256: render.digest_source_sha256.clone(),
        source_url: version_list_url.map(str::to_string),
        digest_url: render.digest_url.clone(),
    }
}

#[cfg(test)]
#[path = "templates/tests.rs"]
mod tests;
