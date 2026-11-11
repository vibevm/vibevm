//! The registry-source model + the local-`[[registry]]` url parser.
//!
//! [`RegistrySource`] is the heterogeneous entry the multi-registry walk
//! iterates: git registries ([`GitPerPackageRegistry`]) and local-directory
//! registries ([`LocalRegistry`]) coexist in declared order, dispatched per
//! variant on the four core operations. [`local_path_from_url`] turns a local
//! `[[registry]]` url into the filesystem path `LocalRegistry` opens.
//!
//! Split out of the resolver root to hold the file-length budget (PROP-002
//! §2.2.2 — a local url is served from the filesystem, never git-cloned).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

/// One source in the priority-ordered registry walk. Git registries
/// ([`GitPerPackageRegistry`] — per-package repos under an org URL, versions by
/// git tag) and local-directory registries ([`LocalRegistry`] — a monorepo
/// laid out `<root>/<group>/<name>/v<version>/`) coexist in declared order.
///
/// A `[[registry]]` whose `url` is local (`file://` / bare path, per
/// [`url_is_local`]) is served by [`LocalRegistry`] straight off the
/// filesystem — never git-cloned — so a plain on-disk directory works as a
/// registry (PROP-002 §2.2.2). The two backends share the `<group>/<name>
/// /v<version>/` layout but resolve it differently (git per-package repos vs
/// a filesystem tree), so they cannot be wrapped one as the other; the
/// resolver dispatches per variant on the four core operations
/// (list / resolve / fetch-dep-manifest / fetch).
#[derive(Clone)]
pub enum RegistrySource {
    /// Remote (or local-git) org-root registry: `<org>/<group>.<name>.git`
    /// per-package repos, versions by `git ls-remote` tags.
    Git(Arc<GitPerPackageRegistry>),
    /// Local-directory registry: a filesystem root laid out
    /// `<root>/<group>/<name>/v<version>/`, read with no git.
    Local(LocalRegistrySource),
    /// Single-repository registry (`type = "monorepo"`, PROP-002
    /// `#registry-type`): one Git repository whose `ref` carries every
    /// package as `<group>/<name>/v<version>/`, cloned on first use.
    Monorepo(Arc<MonorepoRegistrySource>),
}

/// The monorepo flavour of a [`RegistrySource`]. Construction makes no
/// contact; the clone (through the remap Locator's candidates and the
/// operation's Git backend, hence its source policy and admission) happens
/// on the first read and is reused for the rest of the operation.
pub struct MonorepoRegistrySource {
    pub name: String,
    pub url: String,
    pub refname: String,
    pub auth: vibe_core::manifest::AuthKind,
    candidates: Vec<String>,
    cache_root: PathBuf,
    backend: Arc<dyn GitBackend>,
    freshness_secs: u64,
    opened: std::sync::OnceLock<GitMonorepoRegistry>,
    gate: std::sync::Mutex<()>,
    /// The registries the effective profile allows — see
    /// [`GitPerPackageRegistry::with_registry_roster`]. Carried into the
    /// clone failure, which is the one git operation a monorepo
    /// registry performs on its own behalf.
    roster: Arc<crate::git_failure::RegistryRoster>,
}

impl MonorepoRegistrySource {
    pub(crate) fn new(
        reg: &RegistrySection,
        auth: vibe_core::manifest::AuthKind,
        candidates: Vec<String>,
        cache_root: &Path,
        backend: Arc<dyn GitBackend>,
        freshness_secs: u64,
    ) -> Self {
        MonorepoRegistrySource {
            name: reg.name.clone(),
            url: reg.url.clone(),
            refname: reg.r#ref.clone(),
            auth,
            candidates,
            cache_root: cache_root.to_path_buf(),
            backend,
            freshness_secs,
            opened: std::sync::OnceLock::new(),
            gate: std::sync::Mutex::new(()),
            roster: Arc::new(crate::git_failure::RegistryRoster::default()),
        }
    }

    /// Record the registries this profile allows, for the clone failure
    /// this source reports. Called by the catalog, which is the one
    /// place that knows the declared set.
    pub(crate) fn with_registry_roster(
        mut self,
        roster: Arc<crate::git_failure::RegistryRoster>,
    ) -> Self {
        self.roster = roster;
        self
    }

    /// Whether the repository is read through the filesystem (a `file:`
    /// url, an absolute path, a Windows share): it needs no network, so
    /// `--offline` reads it like a `localdir` registry (PROP-002
    /// `#offline-local`).
    pub fn is_local(&self) -> bool {
        is_local_url(&self.url)
    }

    /// The local-directory view under `--offline`: only a local repository,
    /// and only its local remap candidates, are read. `None` when the
    /// repository is not local.
    pub fn offline_local(&self) -> Result<Option<&LocalRegistry>, RegistryError> {
        if !self.is_local() {
            return Ok(None);
        }
        self.open(true).map(|opened| Some(opened.local()))
    }

    /// The cloned registry, cloning or refreshing it on first use.
    pub fn registry(&self) -> Result<&GitMonorepoRegistry, RegistryError> {
        self.open(false)
    }

    fn open(&self, local_only: bool) -> Result<&GitMonorepoRegistry, RegistryError> {
        if let Some(opened) = self.opened.get() {
            return Ok(opened);
        }
        let _guard = self.gate.lock().map_err(|_| RegistryError::MalformedMeta {
            path: self.cache_root.clone(),
            reason: "monorepo registry clone lock poisoned".into(),
        })?;
        if self.opened.get().is_none() {
            let cache = self.cache_root.join("monorepo");
            let candidates: Vec<String> = self
                .candidates
                .iter()
                .filter(|candidate| !local_only || is_local_url(candidate))
                .cloned()
                .collect();
            // A controlled transport admits only native preparation.
            // The whole clone chain is one errand — "capturing the
            // mirror of registry `corp`" — so its failure is explained
            // here, where the registry's name, its declared url and the
            // allowed set are all in hand, rather than inside a clone
            // helper that knows only a candidate address.
            let explain = |error| {
                crate::git_failure::upgrade(
                    crate::git_failure::GitOperation::capture_mirror(&self.name),
                    &self.name,
                    &self.url,
                    &self.roster,
                    error,
                )
            };
            let opened = if self.backend.native_materialisation_for(&self.url)? {
                GitMonorepoRegistry::open_native(
                    &self.url,
                    &candidates,
                    &self.refname,
                    &cache,
                    Arc::clone(&self.backend),
                )
                .map_err(explain)?
            } else {
                GitMonorepoRegistry::open_routes(
                    &self.url,
                    &candidates,
                    &self.refname,
                    &cache,
                    Arc::clone(&self.backend),
                    self.freshness_secs,
                )
                .map_err(explain)?
            }
            .with_name(&self.name);
            let _ = self.opened.set(opened);
        }
        self.opened
            .get()
            .ok_or_else(|| RegistryError::MalformedMeta {
                path: self.cache_root.clone(),
                reason: "monorepo registry clone produced no registry".into(),
            })
    }

    /// The clone's local-directory view.
    pub fn local(&self) -> Result<&LocalRegistry, RegistryError> {
        self.registry().map(GitMonorepoRegistry::local)
    }
}

/// Whether a repository `url` is read through the filesystem
/// ([`crate::git_backend::shell::native_admission::local_repository`]).
pub(crate) fn is_local_url(url: &str) -> bool {
    crate::git_backend::shell::native_admission::local_repository(url)
        .is_some_and(|found| found.is_ok())
}

/// The local-directory flavour of a [`RegistrySource`] — a [`LocalRegistry`]
/// plus the `[[registry]]` metadata the walk and the diagnostics still need
/// (the registry `name`, used to dispatch a fetch back to the source that
/// resolved it, and the original `url`, used for the failure-discriminator
/// attempts and the lockfile `source_url`).
#[derive(Clone)]
pub struct LocalRegistrySource {
    pub name: String,
    pub url: String,
    pub registry: LocalRegistry,
}

impl RegistrySource {
    /// The `[[registry]].name` — identifies the source for the fetch
    /// dispatch (a resolution records `registry_name`; fetch finds the source
    /// by name) and for the failure-discriminator attempts.
    pub fn name(&self) -> &str {
        match self {
            RegistrySource::Git(g) => g.name(),
            RegistrySource::Local(ls) => &ls.name,
            RegistrySource::Monorepo(mono) => &mono.name,
        }
    }

    /// The `[[registry]].url` — for the failure-discriminator attempts and
    /// the lockfile `source_url`.
    pub fn url(&self) -> &str {
        match self {
            RegistrySource::Git(g) => g.org_url(),
            RegistrySource::Local(ls) => &ls.url,
            RegistrySource::Monorepo(mono) => &mono.url,
        }
    }

    /// The **directory root** this source serves packages from, laid out
    /// `<root>/<group>/<name>/v<version>/` — the place a template is read
    /// off the manifests on disk (PROP-075 `#TEMPLATE-DISCOVERY`).
    ///
    /// A `localdir` registry is that directory. A `monorepo` registry's is
    /// its **working copy**, opened on first use and reused afterwards, so
    /// one repository answers as a directory registry like any other. A
    /// per-package **git** registry has no such root and answers `None`,
    /// and so does a working copy that cannot be opened — a root is an
    /// enumeration's optional contribution, never its failure.
    ///
    /// `offline` reads only a local repository and only its local
    /// candidates, exactly as a resolution under `--offline` does
    /// (PROP-002 `#offline-local`).
    ///
    /// Deciding this per variant beside the variants is the point: a
    /// caller that matched the variants itself forgot the monorepo, and
    /// every listing over the corporate shape — one mirrored monorepo and
    /// nothing else — then had no root at all (PROP-075
    /// `#VERSIONS-CHANNEL-DISCOVERY`).
    pub fn directory_root(&self, offline: bool) -> Option<PathBuf> {
        match self {
            RegistrySource::Git(_) => None,
            RegistrySource::Local(ls) => Some(ls.registry.root().to_path_buf()),
            RegistrySource::Monorepo(mono) => {
                let opened = if offline {
                    mono.offline_local().ok().flatten()
                } else {
                    mono.local().ok()
                };
                opened.map(|local| local.root().to_path_buf())
            }
        }
    }
}

/// Whether a url names a **local directory** by an explicit `file:` scheme
/// (`file:///C:/x`, `file:///home/x`) — the shape a held directory-registry
/// lock row records, and the url shape a `[[registry]]` without `type`
/// infers as `localdir` (`RegistryType::infer`). A bare path and a `git+`
/// transport are not directories here. (The wider [`url_is_local`] is the
/// `--offline` filter's predicate.)
pub(crate) fn is_local_directory_url(url: &str) -> bool {
    url.trim().to_ascii_lowercase().starts_with("file:")
}

/// Turn a local `[[registry]]` url (`file://…` / bare path, already classified
/// local by [`url_is_local`] and not a `git+` transport — `git+file://` is a
/// local *git* repo, handled by the git-clone backend, not here) into the
/// filesystem path [`LocalRegistry`] opens. Handles the common empty-authority
/// forms — `file:///C:/x` (Windows drive), `file:///home/x` (POSIX),
/// `file:/x` — and bare paths verbatim. UNC (`file://host/share`) and anything
/// else with a non-empty authority are rejected: they are not the
/// plain-directory registry shape `LocalRegistry` serves.
pub(crate) fn local_path_from_url(url: &str) -> Result<PathBuf, RegistryError> {
    let s = url.trim();
    let raw = if let Some(after) = s
        .strip_prefix("file://")
        .or_else(|| s.strip_prefix("file:"))
    {
        // `file://<authority>/<path>` or `file:<path>`. Only the empty-authority
        // form (`file:///…`, `after` starts with `/`) maps to a plain path; a
        // non-empty authority would be a UNC share, which LocalRegistry does
        // not serve.
        let path_slash =
            after
                .strip_prefix('/')
                .ok_or_else(|| RegistryError::BadLocalRegistryUrl {
                    url: url.to_string(),
                })?;
        format!("/{path_slash}")
    } else {
        // No `file:` scheme — a bare filesystem path, used as-is.
        s.to_string()
    };
    let path = PathBuf::from(strip_leading_drive_slash(&raw));
    if path.as_os_str().is_empty() {
        return Err(RegistryError::BadLocalRegistryUrl {
            url: url.to_string(),
        });
    }
    Ok(path)
}

/// The directory a `localdir` registry url names (`file:///…` or a plain
/// path) — the public face of the path rule the resolver uses, so a
/// publisher writes exactly where resolution reads.
///
/// ```
/// let path = vibe_registry::localdir_path("file:///srv/vibe").unwrap();
/// assert_eq!(path, std::path::PathBuf::from("/srv/vibe"));
/// ```
pub fn localdir_path(url: &str) -> Result<PathBuf, RegistryError> {
    local_path_from_url(url)
}

/// Drop the POSIX-style leading slash before a Windows drive letter:
/// `/C:/x` or `/C:\x` → `C:/x` / `C:\x` (the shape `file:///C:/x` produces
/// after stripping the scheme + authority marker). A no-op for POSIX roots
/// (`/home/x`) and for paths that do not start with `/<letter>:`.
fn strip_leading_drive_slash(s: &str) -> &str {
    let b = s.as_bytes();
    if b.len() >= 3 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':' {
        &s[1..]
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `local_path_from_url` handles the `file://` Windows-drive and POSIX
    /// forms and bare paths — the shapes a local-directory `[[registry]]` url
    /// takes (per `url_is_local`). A `git+` transport is NOT a local-directory
    /// url (it is a local *git* repo the git-clone backend handles) and never
    /// reaches this parser.
    #[test]
    fn local_path_from_url_parses_file_and_bare_forms() {
        // file:// Windows drive: the empty-authority form `file:///C:/x`.
        assert_eq!(
            local_path_from_url("file:///C:/x/y").unwrap(),
            PathBuf::from("C:/x/y")
        );
        // file:// POSIX absolute.
        assert_eq!(
            local_path_from_url("file:///home/x/y").unwrap(),
            PathBuf::from("/home/x/y")
        );
        // Bare path (no scheme) — used verbatim.
        assert_eq!(
            local_path_from_url("/abs/path").unwrap(),
            PathBuf::from("/abs/path")
        );
        // A non-empty authority (UNC share) is rejected — not a plain directory.
        assert!(local_path_from_url("file://host/share/x").is_err());
    }
}
