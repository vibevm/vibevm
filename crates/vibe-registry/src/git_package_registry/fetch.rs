//! Clone / update orchestration for the per-package registry —
//! tag-based version listing, archive-first manifest reads with the
//! clone fallback, mirror-aware fetch dispatch, and content-hash
//! materialisation into the per-project cache (PROP-002 §2.3 / §2.6).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

impl GitPackageRegistry {
    /// Fetch `vibe.toml` at an arbitrary git ref (tag, branch,
    /// or commit SHA) — used by the git-source resolver path
    /// (PROP-002 §2.4.1) where the operator declared `tag = "..."` /
    /// `branch = "..."` / `rev = "..."` and we cannot enumerate
    /// versions through `list_versions` (which is tag-shaped).
    ///
    /// Same auth / token-injection discipline as
    /// [`Self::fetch_dep_manifest`]: token from env at construction,
    /// credentialed URL only for the spawned `git archive`, plain
    /// URL recorded in error messages.
    pub fn fetch_manifest_at_ref(
        &self,
        group: &Group,
        name: &str,
        refname: &str,
    ) -> Result<Manifest, RegistryError> {
        self.ensure_token_loaded()?;
        let plain_url = self.package_repo_url(group, name)?;
        let fetch_url = self.credentialed_url(&plain_url);
        let bytes = match self
            .backend
            .fetch_file_at_ref(&fetch_url, refname, Manifest::FILENAME)
        {
            Ok(bytes) => bytes,
            Err(GitError::ArchiveUnsupported { .. }) => {
                // GitHub (and a handful of other hosts) refuse
                // `git archive --remote` for `upload-archive` access
                // policy reasons. The git-source / redirect path hits
                // this when the target is on GitHub. Same fall-back
                // shape as `fetch_dep_manifest`: shallow-clone at the
                // requested ref and read `vibe.toml` from the
                // working tree. Slower than archive but works on every
                // host that accepts `git clone`.
                self.refresh_package(group, name, refname)?;
                let clone_dir = self.package_clone_dir(group, name);
                let manifest_path = clone_dir.join(Manifest::FILENAME);
                fs::read(&manifest_path).map_err(|source| RegistryError::Io {
                    path: manifest_path.clone(),
                    source,
                })?
            }
            Err(other) => return Err(RegistryError::from(other)),
        };
        let text = String::from_utf8(bytes).map_err(|e| RegistryError::MalformedMeta {
            path: PathBuf::from(format!("{plain_url}@{refname}:{}", Manifest::FILENAME)),
            reason: format!("invalid UTF-8: {e}"),
        })?;
        Manifest::parse_str(&text).map_err(|e| RegistryError::MalformedMeta {
            path: PathBuf::from(format!("{plain_url}@{refname}:{}", Manifest::FILENAME)),
            reason: e.to_string(),
        })
    }

    /// Run a read-only lookup `f` against the primary URL first, then
    /// each mirror URL in priority order. Returns the first `Ok`
    /// produced by any URL. If every URL fails, the **primary's**
    /// error is returned (not the last mirror's) — the primary is the
    /// canonical source and its diagnostic is the most useful one for
    /// the operator. Mirror errors are recorded in `tracing::debug!`
    /// for ops to correlate.
    ///
    /// `f` MUST be a pure read against the host (no cache writes, no
    /// per-package clone state) — the fetch / refresh paths use
    /// dedicated logic with content-hash verification across mirrors.
    fn try_lookup<T, F>(&self, group: &Group, name: &str, f: F) -> Result<T, RegistryError>
    where
        F: Fn(&str) -> Result<T, RegistryError>,
    {
        let (primary, mirrors) = self.package_urls(group, name)?;
        // The primary attempt sits outside the mirror loop: its error
        // is held as a plain value — THE diagnostic if nothing serves.
        let primary_err = match f(&primary) {
            Ok(v) => return Ok(v),
            Err(e) => e,
        };
        for (i, url) in mirrors.iter().enumerate() {
            match f(url) {
                Ok(v) => {
                    tracing::info!(
                        target: "vibe_registry",
                        registry = %self.name,
                        primary = %primary,
                        served_by = %url,
                        mirror_index = i,
                        "lookup served by mirror"
                    );
                    return Ok(v);
                }
                Err(e) => {
                    tracing::debug!(
                        target: "vibe_registry",
                        registry = %self.name,
                        mirror = %url,
                        error = %e,
                        "mirror lookup failed; trying next"
                    );
                }
            }
        }
        Err(primary_err)
    }

    /// Bootstrap (or refresh) the per-package clone at `clone_dir`
    /// against `url`. Used by [`Self::ensure_clone_against_sources`]
    /// and the mirror-fallback variants of [`Self::fetch`] /
    /// [`Self::refresh_package`].
    ///
    /// On entry: `clone_dir` is either absent, empty, or a previously
    /// populated git working tree. If the working tree exists,
    /// [`GitBackend::update`] is tried first — that preserves the local
    /// clone and is the cheap path. If `update` fails (origin
    /// unreachable, ref missing, etc.), the clone is wiped and we
    /// retry via [`GitBackend::bootstrap`] against the same URL. The
    /// wipe-and-rebootstrap branch is what allows the next mirror in
    /// the chain to take over cleanly even if a previous clone left
    /// stale state behind.
    fn bootstrap_or_update_at(
        &self,
        url: &str,
        refname: &str,
        clone_dir: &Path,
    ) -> Result<(), RegistryError> {
        if clone_dir.join(".git").exists() {
            match self.backend.update(clone_dir, refname) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::debug!(
                        target: "vibe_registry",
                        registry = %self.name,
                        url = %url,
                        error = %e,
                        "update on existing clone failed; wiping and re-bootstrapping"
                    );
                    fs::remove_dir_all(clone_dir).map_err(|source| RegistryError::Io {
                        path: clone_dir.to_path_buf(),
                        source,
                    })?;
                }
            }
        }
        if clone_dir.exists() {
            // Half-populated dir from a prior failed bootstrap — clean.
            fs::remove_dir_all(clone_dir).map_err(|source| RegistryError::Io {
                path: clone_dir.to_path_buf(),
                source,
            })?;
        }
        if let Some(parent) = clone_dir.parent() {
            fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        // PROP-002 §2.2.1 — under `auth = "token-env"` the bootstrap
        // is performed with a credentialised URL, then the recorded
        // origin URL is rewritten to the plain (token-free) form so
        // the freshly-cloned `.git/config` does NOT carry the token
        // on disk. Subsequent `update` calls hit the plain origin
        // (and 401 on a still-private host); the
        // `ensure_clone_against_sources` retry path handles that by
        // wiping and re-bootstrapping. The token only ever lives in
        // memory and inside the spawned `git clone` process.
        let plain_url = strip_git_plus_prefix(url);
        let fetch_url = inject_token(plain_url, self.effective_token.as_deref());
        self.backend.bootstrap(&fetch_url, refname, clone_dir)?;
        if self.effective_token.is_some() {
            self.backend
                .set_remote_url(clone_dir, "origin", plain_url)?;
        }
        Ok(())
    }

    /// Bring the per-package clone at `package_clone_dir(kind, name)`
    /// to `refname` by trying the primary URL first, then each mirror
    /// URL in priority order. Returns the URL that ultimately served
    /// the clone (canonical or a mirror) so the caller can record /
    /// log it.
    ///
    /// Mirror dispatch on this path is the cache-mutating sibling of
    /// [`Self::try_lookup`]: same primary-first ordering, same
    /// "primary's error is the most informative" semantics on full
    /// failure. The crucial difference is per-source state — each
    /// retry that goes through bootstrap wipes the local clone first,
    /// so a flapping primary that left a half-populated dir cannot
    /// poison the mirror attempt.
    ///
    /// **Mirror integrity** is **not** checked here: the content from
    /// whichever URL succeeds is taken verbatim. The caller (typically
    /// [`Self::fetch_with_expected_hash`]) layers a content_hash
    /// gate on top when a lockfile pin is available.
    fn ensure_clone_against_sources(
        &self,
        group: &Group,
        name: &str,
        refname: &str,
    ) -> Result<String, RegistryError> {
        let (primary, mirrors) = self.package_urls(group, name)?;
        let clone_dir = self.package_clone_dir(group, name);
        // Primary outside the mirror loop — its error is a plain value.
        let primary_err = match self.bootstrap_or_update_at(&primary, refname, &clone_dir) {
            Ok(()) => return Ok(primary),
            Err(e) => e,
        };
        for (i, url) in mirrors.iter().enumerate() {
            match self.bootstrap_or_update_at(url, refname, &clone_dir) {
                Ok(()) => {
                    tracing::info!(
                        target: "vibe_registry",
                        registry = %self.name,
                        primary = %primary,
                        served_by = %url,
                        mirror_index = i,
                        "fetch served by mirror"
                    );
                    return Ok(url.clone());
                }
                Err(e) => {
                    tracing::debug!(
                        target: "vibe_registry",
                        registry = %self.name,
                        mirror = %url,
                        error = %e,
                        "mirror fetch failed; trying next"
                    );
                }
            }
        }
        Err(primary_err)
    }

    /// Enumerate available versions for `<group>/<name>` *without cloning*.
    /// Tags that don't match `v<semver>` are silently dropped.
    ///
    /// Mirror-aware: tries the primary URL first, then each mirror in
    /// priority order. The first URL that yields a tag list wins. If
    /// every URL says `RepoNotFound`, the result is `UnknownPackage`
    /// (treated identically to the primary-only path).
    pub fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        // Index fast path (PROP-005 §2.10 slice 10). When the
        // registry has an upstream index attached, query it first.
        // 200 → return versions; 404 → fall through to git path
        // (UnknownPackage from the index does not authoritatively
        // mean "absent" — the index may be stale); other errors →
        // also fall through with a debug-level log.
        if let Some(client) = &self.index_client {
            match client.list_versions(group, name) {
                Ok(Some(versions)) => {
                    tracing::debug!(
                        target: "vibe_registry::index",
                        registry = %self.name,
                        group = %group,
                        name = %name,
                        count = versions.len(),
                        "list_versions served from index"
                    );
                    return Ok(versions);
                }
                Ok(None) => {
                    tracing::debug!(
                        target: "vibe_registry::index",
                        registry = %self.name,
                        group = %group,
                        name = %name,
                        "index returned 404; falling through to git ls-remote"
                    );
                }
                Err(e) => {
                    tracing::debug!(
                        target: "vibe_registry::index",
                        registry = %self.name,
                        error = %e,
                        "index lookup failed; falling through to git ls-remote"
                    );
                }
            }
        }
        // PROP-002 §2.2.1 — fail fast when this registry declared
        // `auth = "token-env"` but the env-var resolved empty, before
        // we burn a network round-trip on a guaranteed-401.
        self.ensure_token_loaded()?;
        let backend = Arc::clone(&self.backend);
        let owned_group = group.clone();
        let owned_name = name.to_owned();
        let token = self.effective_token.clone();
        self.try_lookup(group, name, move |url| {
            let plain = strip_git_plus_prefix(url);
            let fetch_url = inject_token(plain, token.as_deref());
            let tags = backend.list_tags(&fetch_url).map_err(|e| match e {
                GitError::RepoNotFound { .. } => RegistryError::UnknownPackage {
                    group: owned_group.clone(),
                    name: owned_name.clone(),
                },
                other => RegistryError::Git(other),
            })?;
            let mut versions: Vec<semver::Version> = tags
                .iter()
                .filter_map(|t| {
                    let stripped = t.strip_prefix('v')?;
                    semver::Version::parse(stripped).ok()
                })
                .collect();
            versions.sort();
            Ok(versions)
        })
    }

    /// Pick the best tag matching `pkgref.version` from the upstream tag list.
    /// Returns a [`ResolvedPackage`] whose `source_dir` points at the
    /// (not-yet-populated) clone directory under the cache bucket.
    ///
    /// The registry resolves by `(group, name)` identity (PROP-008); a
    /// pkgref reaching this point without a `group` is an
    /// [`RegistryError::UnqualifiedPkgref`] — short names must be
    /// qualified at the CLI boundary first.
    pub fn resolve(&self, pkgref: &PackageRef) -> Result<ResolvedPackage, RegistryError> {
        let group = pkgref
            .group
            .as_ref()
            .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;
        let versions = self.list_versions(group, &pkgref.name)?;
        let picked = match &pkgref.version {
            VersionSpec::Latest => versions.iter().rev().find(|v| v.pre.is_empty()).cloned(),
            VersionSpec::Req(req) => versions
                .iter()
                .rev()
                .find(|v| req.matches(v) && v.pre.is_empty())
                .or_else(|| versions.iter().rev().find(|v| req.matches(v)))
                .cloned(),
        };
        let Some(version) = picked else {
            return Err(RegistryError::NoMatchingVersion {
                group: group.clone(),
                name: pkgref.name.clone(),
                req: match &pkgref.version {
                    VersionSpec::Latest => "latest".to_string(),
                    VersionSpec::Req(r) => r.to_string(),
                },
            });
        };
        Ok(ResolvedPackage {
            group: group.clone(),
            name: pkgref.name.clone(),
            version,
            source_dir: self.package_clone_dir(group, &pkgref.name),
        })
    }

    /// Read a candidate version's `vibe.toml` *without cloning*. The
    /// depsolver calls this during the resolve walk to read declared
    /// `[requires]` of a candidate before committing to install. A walk
    /// over N candidates of one package costs N `git archive` round-trips,
    /// not N clones.
    ///
    /// Mirror-aware on the archive path: the primary URL is tried
    /// first, then each mirror in priority order. The clone-fallback
    /// path (used when *every* URL says `ArchiveUnsupported`) clones
    /// only against the primary URL — the clone state is shared and
    /// cross-source verification has not yet landed (Phase B v0).
    pub fn fetch_dep_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, RegistryError> {
        self.ensure_token_loaded()?;
        let tag = format!("v{version}");
        let backend = Arc::clone(&self.backend);
        let tag_for_lookup = tag.clone();
        let token = self.effective_token.clone();
        let archive_result = self.try_lookup(group, name, move |url| {
            let plain = strip_git_plus_prefix(url);
            let fetch_url = inject_token(plain, token.as_deref());
            backend
                .fetch_file_at_ref(&fetch_url, &tag_for_lookup, Manifest::FILENAME)
                .map_err(RegistryError::from)
        });
        let url = self.package_repo_url(group, name)?;
        let bytes = match archive_result {
            Ok(bytes) => bytes,
            Err(RegistryError::Git(GitError::ArchiveUnsupported { .. })) => {
                // GitHub (and a few other hosts) disable
                // `upload-archive` server-side, so `git archive --remote`
                // can't pull a single file without cloning. Fall back to
                // a per-package shallow clone at the requested tag and
                // read the manifest from the working tree. Slower than
                // the archive path but works on every git host that
                // accepts `git clone`. The clone lands in the same
                // per-package cache directory the install path would
                // use anyway, so this is also pre-warming the cache for
                // the imminent install.
                //
                // Phase B v0: the clone fallback talks only to the
                // primary URL. Mirror dispatch for the clone path
                // requires the cross-source `content_hash` check to
                // come along with it, so it lands together with that.
                self.refresh_package(group, name, &tag)?;
                let clone_dir = self.package_clone_dir(group, name);
                let manifest_path = clone_dir.join(Manifest::FILENAME);
                fs::read(&manifest_path).map_err(|source| RegistryError::Io {
                    path: manifest_path.clone(),
                    source,
                })?
            }
            Err(other) => return Err(other),
        };
        let text = String::from_utf8(bytes).map_err(|e| RegistryError::MalformedMeta {
            path: PathBuf::from(format!("{url}@{tag}:{}", Manifest::FILENAME)),
            reason: format!("invalid UTF-8: {e}"),
        })?;
        Manifest::parse_str(&text).map_err(|e| RegistryError::MalformedMeta {
            path: PathBuf::from(format!("{url}@{tag}:{}", Manifest::FILENAME)),
            reason: e.to_string(),
        })
    }

    /// Refresh the per-package clone for `(group, name)` against `refname`
    /// without touching the per-project cache. If the clone exists, runs
    /// `update`; otherwise bootstraps a fresh clone. Mirror-aware:
    /// the primary URL is tried first, then each mirror in priority
    /// order — the first source that lands a working clone wins.
    ///
    /// Used by `vibe registry sync` to walk lockfile entries and pull
    /// upstream changes for everything currently installed, without
    /// re-applying writes (that's `vibe update`'s job, not sync's).
    pub fn refresh_package(
        &self,
        group: &Group,
        name: &str,
        refname: &str,
    ) -> Result<(), RegistryError> {
        self.ensure_clone_against_sources(group, name, refname)?;
        Ok(())
    }

    /// Materialise the resolved package into the per-project cache. Clones
    /// (or updates) the per-package repo at the requested tag, then copies
    /// the worktree into `<cache_root>/<kind>/<name>/v<version>/`,
    /// stripping `.git/`.
    ///
    /// Mirror-aware: the primary URL is tried first, then each mirror
    /// in priority order. Whichever source lands the clone first wins
    /// and the cache is materialised from that clone. The
    /// [`CachedPackage::source_uri`] is **always** the canonical
    /// primary URL — mirror URLs are an availability detail, not a
    /// lockfile-recorded identity (PROP-002 §2.3 step 3).
    ///
    /// No content_hash gate at this layer — see
    /// [`Self::fetch_with_expected_hash`] for the cross-source
    /// integrity check.
    pub fn fetch(
        &self,
        resolved: &ResolvedPackage,
        cache_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        self.fetch_with_expected_hash(resolved, cache_root, None)
    }

    /// Mirror-aware fetch with an optional cross-source content_hash
    /// gate.
    ///
    /// Walks the URL chain primary-first; for each URL that yields a
    /// working clone, materialises the cache and computes the
    /// content_hash:
    ///
    /// - If `expected_hash` is `None` (no lockfile pin), accept the
    ///   first source that lands content. Equivalent to [`Self::fetch`].
    /// - If `expected_hash` is `Some(h)`, accept the first source
    ///   whose computed hash equals `h`. Sources serving a disagreeing
    ///   hash trigger a `tracing::warn!` (mirror-integrity event) and
    ///   the walk continues to the next URL — the cache is wiped
    ///   between attempts so a poisoned source cannot leave bytes
    ///   behind. This is the supply-chain check from
    ///   [PROP-002 §2.3](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#mirror).
    ///
    /// If every URL is reached but none matches, the **last
    /// successful fetch's** [`CachedPackage`] is returned (with the
    /// disagreeing hash); it is the caller's responsibility — today
    /// `vibe-install`'s `plan_install` — to convert the stored hash
    /// vs. lockfile-pin mismatch into the user-actionable
    /// `ContentDrift` error. This split keeps registry-layer concerns
    /// (sources, fallback, integrity attempts) separate from
    /// install-layer concerns (lockfile-aware error rendering).
    ///
    /// If every URL fails at the network layer (no source produced
    /// any content), the **primary's** error is surfaced — same
    /// "primary is canonical and its diagnostic is most useful"
    /// semantics as [`Self::try_lookup`].
    #[specmark::spec(
        deviates = "spec://vibevm/discipline/ENGINE-CONFORM-v0.1#rules",
        reason = "no-unwrap-in-domain: primary_err is Some whenever the source loop \
                  exhausts — the primary URL exists by package_urls' type and its \
                  failure is recorded before any continue; lifting the primary out \
                  of the loop (the try_lookup shape) would duplicate the three-step \
                  per-source body this fn shares between primary and mirrors"
    )]
    pub fn fetch_with_expected_hash(
        &self,
        resolved: &ResolvedPackage,
        cache_root: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        // PROP-002 §2.2.1 — bail before any clone work when this
        // registry is `auth = "token-env"` but the env-var resolved
        // empty.
        self.ensure_token_loaded()?;
        let canonical_url = self.package_repo_url(&resolved.group, &resolved.name)?;
        let tag = format!("v{}", resolved.version);
        let (primary, mirrors) = self.package_urls(&resolved.group, &resolved.name)?;
        let clone_dir = self.package_clone_dir(&resolved.group, &resolved.name);
        let dest_cache = cache_root
            .join(resolved.group.as_str())
            .join(&resolved.name)
            .join(format!("v{}", resolved.version));

        let mut primary_err: Option<RegistryError> = None;
        let mut last_cached: Option<CachedPackage> = None;

        for (i, url) in std::iter::once(&primary).chain(mirrors.iter()).enumerate() {
            // 1. Bring the local clone to `tag` from this URL.
            if let Err(e) = self.bootstrap_or_update_at(url, &tag, &clone_dir) {
                if i == 0 {
                    primary_err = Some(e);
                } else {
                    tracing::debug!(
                        target: "vibe_registry",
                        registry = %self.name,
                        mirror = %url,
                        error = %e,
                        "mirror fetch failed; trying next"
                    );
                }
                continue;
            }

            // 2. Materialise the per-project cache, stripping `.git/`.
            if dest_cache.exists() {
                fs::remove_dir_all(&dest_cache).map_err(|source| RegistryError::Io {
                    path: dest_cache.clone(),
                    source,
                })?;
            }
            copy_dir_excluding_git(&clone_dir, &dest_cache)?;

            let manifest_path = dest_cache.join(Manifest::FILENAME);
            let manifest = Manifest::read(&manifest_path)?;
            if manifest.package.is_none() {
                return Err(RegistryError::MalformedMeta {
                    path: manifest_path.clone(),
                    reason: "registry package manifest must carry a [package] table".to_string(),
                });
            }
            let content_hash = compute_content_hash(&dest_cache)?;

            // 3. Cross-source content_hash gate.
            let cached = CachedPackage {
                resolved: resolved.clone(),
                cache_dir: dest_cache.clone(),
                manifest,
                content_hash: content_hash.clone(),
                source_uri: canonical_url.clone(),
                registry_name: Some(self.name.clone()),
                source_ref: Some(tag.clone()),
                resolved_commit: None,
                overridden: false,
                is_git_source: false,
                is_path_source: false,
                via_redirect: None,
            };
            match expected_hash {
                None => {
                    if i > 0 {
                        tracing::info!(
                            target: "vibe_registry",
                            registry = %self.name,
                            primary = %primary,
                            served_by = %url,
                            mirror_index = i - 1,
                            "fetch served by mirror"
                        );
                    }
                    return Ok(cached);
                }
                Some(expected) if expected == content_hash => {
                    if i > 0 {
                        tracing::info!(
                            target: "vibe_registry",
                            registry = %self.name,
                            primary = %primary,
                            served_by = %url,
                            mirror_index = i - 1,
                            "fetch served by mirror; content_hash matches lockfile pin"
                        );
                    }
                    return Ok(cached);
                }
                Some(expected) => {
                    tracing::warn!(
                        target: "vibe_registry",
                        registry = %self.name,
                        url = %url,
                        expected = %expected,
                        actual = %content_hash,
                        "source served content with unexpected content_hash; \
                         falling through to next source"
                    );
                    last_cached = Some(cached);
                    // Wipe the local clone state so the next URL bootstraps
                    // fresh — a poisoned mirror's working tree must not
                    // survive into the next attempt.
                    if clone_dir.exists() {
                        fs::remove_dir_all(&clone_dir).map_err(|source| RegistryError::Io {
                            path: clone_dir.clone(),
                            source,
                        })?;
                    }
                }
            }
        }

        // Every URL was exhausted.
        if let Some(cached) = last_cached {
            // At least one source served content; none matched the
            // expected hash. Return the last one — `vibe-install`'s
            // `plan_install` will lift this into a `ContentDrift`
            // error against the lockfile pin and surface the actionable
            // message. Doing the rendering here would duplicate that
            // logic and lose the lockfile context the install layer
            // already carries.
            return Ok(cached);
        }
        Err(primary_err.expect("primary URL must exist"))
    }
}

/// Recursively copy `src` into `dst`, excluding any `.git` directory at any
/// depth. Used to materialise a clone into the package cache without
/// dragging the git index along — the cache holds payload only, identity
/// rides on `content_hash`. `pub(crate)` because the multi-registry
/// resolver shares the same materialisation path for `[[override]]` clones.
pub(crate) fn copy_dir_excluding_git(src: &Path, dst: &Path) -> Result<(), RegistryError> {
    fs::create_dir_all(dst).map_err(|source| RegistryError::Io {
        path: dst.to_path_buf(),
        source,
    })?;
    for entry in walkdir::WalkDir::new(src)
        .into_iter()
        .filter_entry(|e| e.file_name() != std::ffi::OsStr::new(".git"))
        .filter_map(|e| e.ok())
    {
        let rel = entry.path().strip_prefix(src).unwrap_or(entry.path());
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|source| RegistryError::Io {
                path: target.clone(),
                source,
            })?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            fs::copy(entry.path(), &target).map_err(|source| RegistryError::Io {
                path: target.clone(),
                source,
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use crate::git_package_registry::test_support::*;

    #[test]
    fn list_versions_filters_non_semver_and_sorts() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let url = "git@host:org/org.vibevm.wal.git";
        fake.seed_tags(
            url,
            vec![
                "v0.2.0".into(),
                "v0.1.0".into(),
                "v0.10.0".into(),
                "release-foo".into(),
                "v1.0.0-rc.1".into(),
                "draft".into(),
                "1.2.3".into(), // missing `v` prefix — dropped
            ],
        );
        let r = registry_with(cache.path(), "git@host:org", NamingConvention::Fqdn, fake);
        let versions = r.list_versions(&org(), "wal").unwrap();
        let strs: Vec<String> = versions.iter().map(|v| v.to_string()).collect();
        assert_eq!(strs, vec!["0.1.0", "0.2.0", "0.10.0", "1.0.0-rc.1"]);
    }

    #[test]
    fn list_versions_empty_when_repo_has_no_tags() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags("git@host:org/org.vibevm.wal.git", vec![]);
        let r = registry_with(cache.path(), "git@host:org", NamingConvention::Fqdn, fake);
        let v = r.list_versions(&org(), "wal").unwrap();
        assert!(v.is_empty());
    }

    #[test]
    fn list_versions_repo_not_found_translates_to_unknown_package() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // No seed for the URL → FakeBackend returns RepoNotFound.
        let r = registry_with(cache.path(), "git@host:org", NamingConvention::Fqdn, fake);
        let err = r.list_versions(&org(), "ghost").unwrap_err();
        assert!(matches!(err, RegistryError::UnknownPackage { .. }));
    }

    #[test]
    fn list_versions_falls_through_to_mirror_when_primary_unreachable() {
        // Primary's per-package URL is NOT seeded — primary returns
        // RepoNotFound. Mirror's URL IS seeded with tags. Mirror
        // dispatch should pick up the tag list from the mirror.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(
            "https://mirror.example/vibespecs/org.vibevm.wal.git",
            vec!["v0.1.0".into(), "v0.2.0".into()],
        );
        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::Fqdn,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake,
        );
        let versions = r.list_versions(&org(), "wal").unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].to_string(), "0.1.0");
        assert_eq!(versions[1].to_string(), "0.2.0");
    }

    #[test]
    fn list_versions_prefers_primary_when_both_seeded() {
        // Primary has [v0.1.0] only; mirror has [v0.1.0, v0.2.0].
        // Primary wins because it answers first; the mirror is never
        // consulted. The user's lockfile thus reflects what the
        // canonical source publishes — mirrors don't get to introduce
        // versions the primary doesn't carry.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(
            "https://primary.example/vibespecs/org.vibevm.wal.git",
            vec!["v0.1.0".into()],
        );
        fake.seed_tags(
            "https://mirror.example/vibespecs/org.vibevm.wal.git",
            vec!["v0.1.0".into(), "v0.2.0".into()],
        );
        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::Fqdn,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake,
        );
        let versions = r.list_versions(&org(), "wal").unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].to_string(), "0.1.0");
    }

    #[test]
    fn list_versions_returns_primary_error_when_all_urls_fail() {
        // Neither primary nor mirror is seeded. The result is
        // UnknownPackage from the *primary's* `RepoNotFound` — that's
        // the canonical "the package doesn't exist" diagnostic.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::Fqdn,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake,
        );
        let err = r.list_versions(&org(), "ghost").unwrap_err();
        assert!(matches!(err, RegistryError::UnknownPackage { .. }));
    }

    #[test]
    fn list_versions_walks_mirrors_in_priority_order() {
        // Three mirrors, only the third is seeded. The dispatcher
        // should iterate through mirrors[0], mirrors[1], mirrors[2]
        // and find the answer on mirrors[2]. Mirror order is the
        // caller's responsibility (MultiRegistryResolver does the
        // priority sort) — at this layer we only verify left-to-right
        // dispatch.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(
            "https://m3.example/vibespecs/org.vibevm.wal.git",
            vec!["v0.3.0".into()],
        );
        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::Fqdn,
            vec![
                "https://m1.example/vibespecs".to_string(),
                "https://m2.example/vibespecs".to_string(),
                "https://m3.example/vibespecs".to_string(),
            ],
            fake,
        );
        let versions = r.list_versions(&org(), "wal").unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].to_string(), "0.3.0");
    }

    #[test]
    fn resolve_picks_latest_stable() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(
            "git@host:org/org.vibevm.wal.git",
            vec!["v0.1.0".into(), "v0.2.0".into(), "v1.0.0-rc.1".into()],
        );
        let r = registry_with(cache.path(), "git@host:org", NamingConvention::Fqdn, fake);
        let p = PackageRef::parse("org.vibevm/wal").unwrap();
        let resolved = r.resolve(&p).unwrap();
        // 1.0.0-rc.1 is pre-release; latest stable wins.
        assert_eq!(resolved.version.to_string(), "0.2.0");
    }

    #[test]
    fn resolve_picks_exact_version() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(
            "git@host:org/org.vibevm.wal.git",
            vec!["v0.1.0".into(), "v0.2.0".into(), "v0.3.0".into()],
        );
        let r = registry_with(cache.path(), "git@host:org", NamingConvention::Fqdn, fake);
        let p = PackageRef::parse("org.vibevm/wal@0.2.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        assert_eq!(resolved.version.to_string(), "0.2.0");
    }

    #[test]
    fn resolve_picks_range() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(
            "git@host:org/org.vibevm.wal.git",
            vec!["v0.1.0".into(), "v0.1.5".into(), "v0.2.0".into()],
        );
        let r = registry_with(cache.path(), "git@host:org", NamingConvention::Fqdn, fake);
        let p = PackageRef::parse("org.vibevm/wal@^0.1").unwrap();
        let resolved = r.resolve(&p).unwrap();
        assert_eq!(resolved.version.to_string(), "0.1.5");
    }

    #[test]
    fn resolve_no_match_errors() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags("git@host:org/org.vibevm.wal.git", vec!["v0.1.0".into()]);
        let r = registry_with(cache.path(), "git@host:org", NamingConvention::Fqdn, fake);
        let p = PackageRef::parse("org.vibevm/wal@^9.0").unwrap();
        let err = r.resolve(&p).unwrap_err();
        assert!(matches!(err, RegistryError::NoMatchingVersion { .. }));
    }

    #[test]
    fn fetch_dep_manifest_falls_through_to_mirror_on_archive_path() {
        // Primary's archive endpoint is empty (FakeBackend returns
        // FileNotFoundInRef). Mirror's archive endpoint has the
        // manifest. Dispatch should hit the mirror and return the
        // manifest without a clone.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let primary_url = "https://primary.example/vibespecs/org.vibevm.wal.git";
        let mirror_url = "https://mirror.example/vibespecs/org.vibevm.wal.git";
        // Tag list seeded only on the mirror — list_versions will land
        // on the mirror first too.
        fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
        fake.seed_file(
            mirror_url,
            "v0.1.0",
            "vibe.toml",
            manifest_text("wal", "flow", "0.1.0").into_bytes(),
        );
        let _ = primary_url; // documented for reading the test
        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::Fqdn,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake.clone(),
        );
        let v = semver::Version::parse("0.1.0").unwrap();
        let manifest = r.fetch_dep_manifest(&org(), "wal", &v).unwrap();
        assert_eq!(manifest.require_package().unwrap().name, "wal");
        // No clone — the mirror served the manifest via the archive
        // path, same as the primary-only test asserts.
        assert_eq!(fake.bootstrap_count(), 0);
        assert_eq!(fake.update_count(), 0);
    }

    #[test]
    fn fetch_dep_manifest_reads_via_archive_without_clone() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let url = "git@host:org/org.vibevm.wal.git";
        fake.seed_tags(url, vec!["v0.1.0".into()]);
        fake.seed_file(
            url,
            "v0.1.0",
            "vibe.toml",
            manifest_text("wal", "flow", "0.1.0").into_bytes(),
        );
        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::Fqdn,
            fake.clone(),
        );
        let v = semver::Version::parse("0.1.0").unwrap();
        let manifest = r.fetch_dep_manifest(&org(), "wal", &v).unwrap();
        assert_eq!(manifest.require_package().unwrap().name, "wal");
        assert_eq!(
            manifest.require_package().unwrap().version.to_string(),
            "0.1.0"
        );
        // Critically: no clone was triggered for this manifest read.
        assert_eq!(fake.bootstrap_count(), 0);
        assert_eq!(fake.update_count(), 0);
    }

    #[test]
    fn fetch_clones_and_populates_per_project_cache() {
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        // Build a fake upstream tree at the seeded URL: vibe.toml
        // plus a spec file and a stray `.git/` to make sure the copy
        // strips it on the way to the cache.
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(pkg_root.join("spec")).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();
        fs::write(pkg_root.join("spec/foo.md"), "content\n").unwrap();
        // Upstream tree has no .git/ — the FakeBackend creates one in
        // dest after copying; we want to verify our extractor strips it.

        let fake = Arc::new(FakeBackend::default());
        let url = "git@host:org/org.vibevm.wal.git";
        fake.seed_tags(url, vec!["v0.1.0".into()]);
        fake.seed_bootstrap(url, pkg_root.clone());

        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::Fqdn,
            fake.clone(),
        );

        let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        let cached = r.fetch(&resolved, pkg_cache.path()).unwrap();

        // Cache populated, no .git/ dragged through.
        assert!(cached.cache_dir.join("vibe.toml").exists());
        assert!(cached.cache_dir.join("spec/foo.md").exists());
        assert!(!cached.cache_dir.join(".git").exists());

        // Manifest parsed and content_hash populated.
        assert_eq!(cached.package_meta().name, "wal");
        assert!(cached.content_hash.starts_with("sha256:"));

        // source_uri is the canonical per-package repo URL.
        assert_eq!(cached.source_uri, url);

        // Bootstrap was called exactly once.
        assert_eq!(fake.bootstrap_count(), 1);
    }

    #[test]
    fn fetch_falls_through_to_mirror_when_primary_unreachable() {
        // Primary URL has tags seeded (so list_versions finds the
        // version) but no bootstrap seed → primary's clone fails.
        // Mirror has BOTH tags and bootstrap seeded. The fetch
        // walk should land on the mirror, materialise from there,
        // and still record the canonical primary URL as
        // `cached.source_uri` per PROP-002 §2.3 step 3.
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let primary_url = "https://primary.example/vibespecs/org.vibevm.wal.git";
        let mirror_url = "https://mirror.example/vibespecs/org.vibevm.wal.git";

        let fake = Arc::new(FakeBackend::default());
        // Tags on both — list_versions hits primary first and finds it.
        fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
        fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
        // Bootstrap only on mirror — primary's clone path fails.
        fake.seed_bootstrap(mirror_url, pkg_root.clone());

        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::Fqdn,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake.clone(),
        );

        let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        let cached = r.fetch(&resolved, pkg_cache.path()).unwrap();

        // Materialised from the mirror.
        assert_eq!(cached.package_meta().name, "wal");
        assert_eq!(cached.package_meta().version.to_string(), "0.1.0");
        // PROP-002 §2.3 step 3: source_uri is canonical primary URL,
        // regardless of which source actually served the bytes.
        assert_eq!(cached.source_uri, primary_url);
        assert_eq!(cached.source_ref.as_deref(), Some("v0.1.0"));
        assert_eq!(cached.registry_name.as_deref(), Some("vibespecs"));

        // Bootstrap was attempted twice: primary (fail) + mirror (ok).
        assert_eq!(fake.bootstrap_count(), 2);
        assert_eq!(
            fake.bootstrap_urls(),
            vec![primary_url.to_string(), mirror_url.to_string()]
        );
    }

    #[test]
    fn fetch_prefers_primary_when_both_reachable() {
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let primary_url = "https://primary.example/vibespecs/org.vibevm.wal.git";
        let mirror_url = "https://mirror.example/vibespecs/org.vibevm.wal.git";

        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
        fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
        // Both URLs serve the same content from the same source dir.
        fake.seed_bootstrap(primary_url, pkg_root.clone());
        fake.seed_bootstrap(mirror_url, pkg_root.clone());

        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::Fqdn,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake.clone(),
        );

        let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();

        // Bootstrap exactly once — primary won, mirror untouched.
        assert_eq!(fake.bootstrap_count(), 1);
        assert_eq!(fake.bootstrap_urls(), vec![primary_url.to_string()]);
    }

    #[test]
    fn fetch_falls_through_when_primary_update_fails() {
        // First fetch lands a working clone via primary. Then the
        // primary's tag goes missing (we wire `fail_update_for_url`).
        // Second fetch tries `update` against primary, fails,
        // wipes-and-rebootstraps from primary (still fails — no seed
        // after wipe? actually bootstrap is still seeded), …
        //
        // Actually — once `update` fails on the primary's existing
        // clone, `bootstrap_or_update_at` wipes the clone and retries
        // bootstrap on the SAME URL. The bootstrap then re-seeds
        // (primary IS seeded), so the SAME URL succeeds. To force
        // fall-through, primary must fail BOTH update AND bootstrap.
        // Drop primary's bootstrap seed before the second fetch.
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let primary_url = "https://primary.example/vibespecs/org.vibevm.wal.git";
        let mirror_url = "https://mirror.example/vibespecs/org.vibevm.wal.git";

        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
        fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
        fake.seed_bootstrap(primary_url, pkg_root.clone());
        fake.seed_bootstrap(mirror_url, pkg_root.clone());

        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::Fqdn,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake.clone(),
        );

        // First fetch lands the clone via primary.
        let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();
        assert_eq!(fake.bootstrap_count(), 1);
        assert_eq!(fake.update_count(), 0);

        // Now make primary's update + bootstrap both fail. Mirror
        // remains seeded.
        fake.fail_update_for_url(primary_url);
        fake.bootstrap_seeds.lock().unwrap().remove(primary_url);

        // Second fetch: update primary fails → wipe+re-bootstrap from
        // primary fails → fall through to mirror, which seeds a fresh
        // clone via bootstrap.
        let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();
        // Update was tried once (against primary, failed). Bootstrap
        // counts: 1 (initial primary) + 1 (re-bootstrap primary, fails
        // RepoNotFound after seed removed) + 1 (mirror, succeeds).
        assert_eq!(fake.update_count(), 1);
        assert_eq!(fake.bootstrap_count(), 3);
        assert_eq!(
            fake.bootstrap_urls(),
            vec![
                primary_url.to_string(), // initial fetch
                primary_url.to_string(), // retry after update fail
                mirror_url.to_string(),  // mirror takes over
            ]
        );
    }

    #[test]
    fn fetch_with_expected_hash_passes_through_when_no_pin() {
        // expected_hash = None — equivalent to `fetch`. Just verifies
        // the trait/wrapper plumbing is wired and identical to the
        // existing single-source fetch behaviour.
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let fake = Arc::new(FakeBackend::default());
        let url = "git@host:org/org.vibevm.wal.git";
        fake.seed_tags(url, vec!["v0.1.0".into()]);
        fake.seed_bootstrap(url, pkg_root.clone());

        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::Fqdn,
            fake.clone(),
        );

        let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        let cached = r
            .fetch_with_expected_hash(&resolved, pkg_cache.path(), None)
            .unwrap();
        assert!(cached.content_hash.starts_with("sha256:"));
        assert_eq!(cached.package_meta().name, "wal");
    }

    #[test]
    fn fetch_with_expected_hash_skips_mirror_with_disagreeing_content() {
        // Two seeded mirrors. Primary serves content A; mirror[0]
        // serves content B (disagreeing); mirror[1] serves content A
        // (matches the lockfile pin which is the hash of A).
        // Expected: primary wins on the first iteration because A
        // matches the pin — mirror walk never runs.
        //
        // To make the cross-source check actually fire, we make
        // primary unreachable so the walk reaches the mirrors. Then
        // mirror[0] serves B, hash check fails, fall through to
        // mirror[1] which serves A and matches.
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();

        // Two distinct fixture trees → distinct content_hashes.
        let pkg_a = upstream.path().join("pkg-a");
        fs::create_dir_all(&pkg_a).unwrap();
        fs::write(
            pkg_a.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();
        fs::write(pkg_a.join("README.md"), "# canonical content\n").unwrap();

        let pkg_b = upstream.path().join("pkg-b");
        fs::create_dir_all(&pkg_b).unwrap();
        fs::write(
            pkg_b.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();
        fs::write(pkg_b.join("README.md"), "# DIVERGENT content\n").unwrap();

        // Compute the expected hash of pkg_a for the lockfile pin.
        let temp_for_hash = tempdir().unwrap();
        copy_dir_excluding_git(&pkg_a, temp_for_hash.path()).unwrap();
        let expected_hash = compute_content_hash(temp_for_hash.path()).unwrap();

        let primary_url = "https://primary.example/vibespecs/org.vibevm.wal.git";
        let mirror_a_url = "https://mirror-bad.example/vibespecs/org.vibevm.wal.git";
        let mirror_b_url = "https://mirror-ok.example/vibespecs/org.vibevm.wal.git";

        let fake = Arc::new(FakeBackend::default());
        // All sources seed tags so the resolver reaches them in order.
        fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
        fake.seed_tags(mirror_a_url, vec!["v0.1.0".into()]);
        fake.seed_tags(mirror_b_url, vec!["v0.1.0".into()]);
        // Primary unreachable for clone (no bootstrap seed).
        // mirror_a serves divergent content.
        fake.seed_bootstrap(mirror_a_url, pkg_b.clone());
        // mirror_b serves canonical content.
        fake.seed_bootstrap(mirror_b_url, pkg_a.clone());

        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::Fqdn,
            vec![
                "https://mirror-bad.example/vibespecs".to_string(),
                "https://mirror-ok.example/vibespecs".to_string(),
            ],
            fake.clone(),
        );

        let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        let cached = r
            .fetch_with_expected_hash(&resolved, pkg_cache.path(), Some(&expected_hash))
            .unwrap();

        // Final material is canonical content.
        assert_eq!(cached.content_hash, expected_hash);
        // source_uri remains the canonical primary URL.
        assert_eq!(cached.source_uri, primary_url);

        // Walk: primary (bootstrap fail) → mirror_a (succeed, hash mismatch,
        // fall through) → mirror_b (succeed, hash match).
        assert_eq!(
            fake.bootstrap_urls(),
            vec![
                primary_url.to_string(),
                mirror_a_url.to_string(),
                mirror_b_url.to_string(),
            ]
        );
    }

    #[test]
    fn fetch_with_expected_hash_returns_last_attempt_when_no_match() {
        // Every source serves disagreeing content; lockfile pins
        // something else. Per the contract, the registry returns the
        // last successful CachedPackage with its (non-matching) hash;
        // vibe-install's `plan_install` then renders ContentDrift.
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let primary_url = "https://primary.example/vibespecs/org.vibevm.wal.git";
        let mirror_url = "https://mirror.example/vibespecs/org.vibevm.wal.git";

        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
        fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
        fake.seed_bootstrap(primary_url, pkg_root.clone());
        fake.seed_bootstrap(mirror_url, pkg_root.clone());

        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::Fqdn,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake.clone(),
        );

        let bogus_pin = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
        let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        let cached = r
            .fetch_with_expected_hash(&resolved, pkg_cache.path(), Some(bogus_pin))
            .unwrap();

        // Returned cached carries the actual (non-matching) hash —
        // not the pin. vibe-install's plan_install lifts this into
        // ContentDrift.
        assert_ne!(cached.content_hash, bogus_pin);
        assert!(cached.content_hash.starts_with("sha256:"));
        // Both URLs were tried.
        assert_eq!(fake.bootstrap_count(), 2);
    }

    #[test]
    fn refresh_package_falls_through_to_mirror_when_primary_unreachable() {
        // refresh_package walks primary then mirror, same as fetch.
        // Used by `vibe registry sync`. Test that a fresh sync against
        // an unreachable primary lands the clone via mirror.
        let cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let primary_url = "https://primary.example/vibespecs/org.vibevm.wal.git";
        let mirror_url = "https://mirror.example/vibespecs/org.vibevm.wal.git";

        let fake = Arc::new(FakeBackend::default());
        fake.seed_bootstrap(mirror_url, pkg_root.clone());

        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::Fqdn,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake.clone(),
        );

        r.refresh_package(&org(), "wal", "v0.1.0").unwrap();

        // Primary (fail) + mirror (succeed).
        assert_eq!(
            fake.bootstrap_urls(),
            vec![primary_url.to_string(), mirror_url.to_string()]
        );
    }

    #[test]
    fn fetch_dep_manifest_clone_fallback_uses_mirror_dispatch() {
        // GitHub-shape host: archive endpoint is unsupported. The
        // dep-manifest fetch falls back to the per-package clone via
        // refresh_package. With mirror dispatch wired, the clone walk
        // tries primary then mirror — mirror seeded, primary not.
        let cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let primary_url = "https://primary.example/vibespecs/org.vibevm.wal.git";
        let mirror_url = "https://mirror.example/vibespecs/org.vibevm.wal.git";

        // FakeBackend's `fetch_file_at_ref` returns FileNotFoundInRef
        // when not seeded. To trigger the clone fallback we need an
        // ArchiveUnsupported. Build a dedicated backend variant.
        struct NoArchiveBackend(Arc<FakeBackend>);
        impl GitBackend for NoArchiveBackend {
            fn bootstrap(&self, url: &str, refname: &str, dest: &Path) -> Result<(), GitError> {
                self.0.bootstrap(url, refname, dest)
            }
            fn update(&self, dest: &Path, refname: &str) -> Result<(), GitError> {
                self.0.update(dest, refname)
            }
            fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
                self.0.list_tags(url)
            }
            fn fetch_file_at_ref(
                &self,
                url: &str,
                _refname: &str,
                _path: &str,
            ) -> Result<Vec<u8>, GitError> {
                Err(GitError::ArchiveUnsupported {
                    url: url.to_string(),
                })
            }
        }

        let inner = Arc::new(FakeBackend::default());
        inner.seed_tags(primary_url, vec!["v0.1.0".into()]);
        inner.seed_tags(mirror_url, vec!["v0.1.0".into()]);
        inner.seed_bootstrap(mirror_url, pkg_root.clone());
        // Primary has no bootstrap seed → primary's clone fails →
        // mirror takes over.

        let backend: Arc<dyn GitBackend> = Arc::new(NoArchiveBackend(inner.clone()));
        let r = GitPackageRegistry::open_with_mirrors(
            "vibespecs",
            "https://primary.example/vibespecs",
            "main",
            NamingConvention::Fqdn,
            vec!["https://mirror.example/vibespecs".to_string()],
            cache.path(),
            backend,
            DEFAULT_FRESHNESS_SECS,
        )
        .unwrap();

        let v = semver::Version::parse("0.1.0").unwrap();
        let manifest = r.fetch_dep_manifest(&org(), "wal", &v).unwrap();
        assert_eq!(manifest.require_package().unwrap().name, "wal");

        // Clone-fallback walked primary (fail) + mirror (ok).
        assert_eq!(
            inner.bootstrap_urls(),
            vec![primary_url.to_string(), mirror_url.to_string()]
        );
    }

    #[test]
    fn fetch_reuses_existing_clone_via_update() {
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let fake = Arc::new(FakeBackend::default());
        let url = "git@host:org/org.vibevm.wal.git";
        fake.seed_tags(url, vec!["v0.1.0".into()]);
        fake.seed_bootstrap(url, pkg_root.clone());

        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::Fqdn,
            fake.clone(),
        );
        let p = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();

        let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();
        let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();

        // First fetch: bootstrap; second: update (clone exists from first).
        assert_eq!(fake.bootstrap_count(), 1);
        assert_eq!(fake.update_count(), 1);
    }
}
