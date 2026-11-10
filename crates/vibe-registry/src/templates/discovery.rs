//! Finding the template of a package — on disk, and in the cache alone
//! (PROP-075 `#TEMPLATE-DISCOVERY`, `#ACTIVATION-HOOK`).
//!
//! A client asked for the versions of `group/name` splits the published
//! rows in one read: rows carrying `[package] template = true` are
//! template **revisions**, every other row is a concrete version. The
//! newest revision is the **active** template; older revisions stay
//! resolvable by identity so a lock that pins one can be replayed.
//!
//! For a local-directory registry — `--registry DIR`, a `localdir`
//! source, a `monorepo` working copy, a mirrored layout registry — that
//! answer is read from the manifests on disk, which are already local,
//! and `vibe_index::layout::project` derives the index from exactly those
//! manifests, so the indexed and the layout path agree by construction.
//!
//! The second entry point reads **only the cache**: the declaration kept
//! beside the inputs plus the cached version list, with no registry root,
//! no index read and no request. That is the one question a prompt hook
//! asks, and the budget `#ACTIVATION-HOOK` gives it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DISCOVERY");

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use vibe_core::Group;
use vibe_core::manifest::Manifest;

use super::error::TemplateError;
use super::{ActiveTemplate, HttpTransport, Posture, TemplateLayer};

impl TemplateLayer {
    /// **Discovery over a local-directory registry**: read the manifests
    /// on disk and split this package's versions into template revisions
    /// and concrete versions.
    ///
    /// A layout registry's manifests are already local, and
    /// `vibe_index::layout::project` derives its index from exactly these
    /// manifests, so the indexed and the layout path agree by
    /// construction (PROP-075 `#TEMPLATE-DISCOVERY`).
    ///
    /// `Ok(None)` means this package publishes no template revision —
    /// the ordinary case for every package in the tree.
    pub fn discover_in_directory(
        &self,
        registry_root: &Path,
        group: &Group,
        name: &str,
    ) -> Result<Option<ActiveTemplate>, TemplateError> {
        let package_dir = registry_root.join(group.as_str()).join(name);
        if !package_dir.is_dir() {
            return Ok(None);
        }
        let mut revisions: BTreeMap<semver::Version, PathBuf> = BTreeMap::new();
        let mut published: BTreeSet<semver::Version> = BTreeSet::new();
        let entries = std::fs::read_dir(&package_dir).map_err(|error| TemplateError::Io {
            path: package_dir.clone(),
            reason: error.to_string(),
        })?;
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let Some(version) = file_name
                .to_str()
                .and_then(|spelling| spelling.strip_prefix('v'))
                .and_then(|spelling| semver::Version::parse(spelling).ok())
            else {
                continue;
            };
            let manifest_path = entry.path().join(Manifest::FILENAME);
            let Ok(text) = std::fs::read_to_string(&manifest_path) else {
                continue;
            };
            if declares_template(&text) {
                revisions.insert(version, entry.path());
            } else {
                published.insert(version);
            }
        }
        let Some((revision, tree)) = revisions.iter().next_back() else {
            return Ok(None);
        };
        let coordinate = format!("{group}/{name}");
        let manifest_path = tree.join(Manifest::FILENAME);
        let template_toml =
            std::fs::read_to_string(&manifest_path).map_err(|error| TemplateError::Io {
                path: manifest_path,
                reason: error.to_string(),
            })?;
        let section = vibe_template::template_section(&template_toml).map_err(|error| {
            TemplateError::Declaration {
                package: coordinate,
                reason: error.to_string(),
            }
        })?;
        let cache = self.cache(group, name);
        // Keep the declaration beside its inputs, so a channel can be
        // lowered later from the cache alone — the budget a prompt hook
        // has (PROP-075 `#ACTIVATION-HOOK`).
        cache.write_declaration(&template_toml)?;
        // A registry that carries a mirror's template snapshot primes
        // this cache from it, and nothing below here learns that the
        // bytes came from a registry rather than from upstream: the two
        // document readers find an ordinary cache hit and never reach the
        // transport. That is the whole mechanism by which a client inside
        // a closed network renders a snapshotted version "with the same
        // code as outside" (PROP-075 `#MIRROR-TEMPLATE-SNAPSHOT`).
        self.prime_from_snapshot(registry_root, group, name, &cache);
        Ok(Some(ActiveTemplate {
            group: group.clone(),
            name: name.to_string(),
            revision: revision.clone(),
            tree: tree.clone(),
            template_toml,
            section,
            published: published.into_iter().collect(),
            revisions: revisions.keys().cloned().collect(),
        }))
    }

    /// Prime `cache` from the registry's own template snapshot, if it
    /// carries one.
    ///
    /// **Never fails the discovery.** A snapshot is a cache of someone
    /// else's bytes: one that cannot be read leaves the client with its
    /// own cache and whatever network it has, which is a degraded answer,
    /// while a refusal would make the registry unusable for the package.
    /// What a broken snapshot must not do is go unmentioned, so the
    /// reasons are warned about in [`super::snapshot`] where each is detected.
    fn prime_from_snapshot(
        &self,
        registry_root: &Path,
        group: &Group,
        name: &str,
        cache: &super::TemplateCache,
    ) {
        let Ok(Some(snapshot)) = super::Snapshot::read(registry_root, group, name) else {
            return;
        };
        match snapshot.prime(cache) {
            Ok(0) => {}
            Ok(placed) => tracing::debug!(
                target: "vibe_registry::templates",
                "{group}/{name}: primed {placed} file(s) from the registry's template snapshot \
                 of {}",
                snapshot.synced()
            ),
            Err(error) => tracing::warn!(
                target: "vibe_registry::templates",
                "{group}/{name}: the registry's template snapshot could not be primed into the \
                 cache ({error}); falling back to this machine's own cache"
            ),
        }
    }

    /// The snapshot `<group>/<name>`'s registry carries, if any — what
    /// `vibe app versions` reports the `[sync] last` of, so a user in a
    /// closed network can see how old the version list is.
    pub fn registry_snapshot(
        &self,
        registry_root: &Path,
        group: &Group,
        name: &str,
    ) -> Option<super::Snapshot> {
        super::Snapshot::read(registry_root, group, name)
            .ok()
            .flatten()
    }

    /// The template of `<group>/<name>` **from the cache alone**: the
    /// declaration cached beside the inputs, with no registry root, no
    /// index read and no request.
    ///
    /// The returned template carries no concrete versions and no tree —
    /// nothing may be installed from it. It exists for the one question a
    /// prompt hook asks: which installed versions does this channel
    /// hold (PROP-075 `#ACTIVATION-HOOK`, `#VERSION-FILE-FORMATS`).
    pub fn cached_template(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Option<ActiveTemplate>, TemplateError> {
        let cache = self.cache(group, name);
        let Some(template_toml) = cache.read_declaration()? else {
            return Ok(None);
        };
        let coordinate = format!("{group}/{name}");
        let section = vibe_template::template_section(&template_toml).map_err(|error| {
            TemplateError::Declaration {
                package: coordinate,
                reason: error.to_string(),
            }
        })?;
        let revision =
            declared_version(&template_toml).unwrap_or_else(|| semver::Version::new(0, 0, 0));
        Ok(Some(ActiveTemplate {
            group: group.clone(),
            name: name.to_string(),
            revision,
            tree: cache.dir().to_path_buf(),
            template_toml,
            section,
            published: Vec::new(),
            revisions: Vec::new(),
        }))
    }

    /// The channel table of `<group>/<name>` read from the cache alone,
    /// or `None` when nothing about that package is cached.
    ///
    /// Always offline by construction: the cached version list answers
    /// whatever its age, and a miss is `None` rather than a refusal —
    /// a prompt hook must never fail a `cd`.
    pub fn cached_channels(
        &self,
        group: &Group,
        name: &str,
    ) -> Option<BTreeMap<String, BTreeSet<semver::Version>>> {
        let template = self.cached_template(group, name).ok()??;
        let offline = TemplateLayer {
            templates_root: self.templates_root.clone(),
            ttl: self.ttl,
            posture: Posture::Offline,
            transport: Box::new(HttpTransport),
            now: self.now,
            recorded: std::sync::Mutex::new(BTreeMap::new()),
        };
        let domain = offline.domain(&template).ok()?;
        Some(domain.channels().clone())
    }
}

/// Whether this manifest text declares `[package] template = true`.
///
/// Read off the text rather than a typed `Manifest`, because a template
/// that uses the numeric coercion (`size = "{size:…}"`) does not
/// deserialise into one (PROP-075 `#TEMPLATE-DOCUMENT-READING`).
///
/// ```
/// use vibe_registry::templates::declares_template;
///
/// assert!(declares_template("[package]\nname = \"t\"\ntemplate = true\n"));
/// assert!(!declares_template("[package]\nname = \"t\"\n"));
/// assert!(!declares_template("[package]\nname = \"t\"\ntemplate = false\n"));
/// // A `template` key in some other table is not the flag.
/// assert!(!declares_template("[package]\nname = \"t\"\n\n[other]\ntemplate = true\n"));
/// ```
pub fn declares_template(manifest_toml: &str) -> bool {
    #[derive(serde::Deserialize)]
    struct OnlyPackage {
        package: Option<FlagOnly>,
    }
    #[derive(serde::Deserialize)]
    struct FlagOnly {
        #[serde(default)]
        template: bool,
    }
    toml::from_str::<OnlyPackage>(manifest_toml)
        .ok()
        .and_then(|document| document.package)
        .is_some_and(|package| package.template)
}

/// The `[package].version` a manifest text declares.
///
/// Read off the text for the same reason [`declares_template`] is: a
/// template that uses the numeric coercion does not deserialise into a
/// typed `Manifest` (PROP-075 `#TEMPLATE-DOCUMENT-READING`).
///
/// ```
/// use vibe_registry::templates::declared_version;
///
/// assert_eq!(
///     declared_version("[package]\nname = \"t\"\nversion = \"0.3.0\"\n")
///         .map(|v| v.to_string())
///         .as_deref(),
///     Some("0.3.0")
/// );
/// assert!(declared_version("[package]\nname = \"t\"\n").is_none());
/// ```
pub fn declared_version(manifest_toml: &str) -> Option<semver::Version> {
    #[derive(serde::Deserialize)]
    struct OnlyPackage {
        package: Option<VersionOnly>,
    }
    #[derive(serde::Deserialize)]
    struct VersionOnly {
        version: Option<semver::Version>,
    }
    toml::from_str::<OnlyPackage>(manifest_toml)
        .ok()?
        .package?
        .version
}
