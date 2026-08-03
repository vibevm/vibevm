//! Clone-free lookups for the per-package registry — tag-based version
//! listing, archive-first manifest reads with the clone fallback, and
//! version resolution against the upstream tag set (PROP-002 §2.3 /
//! §2.5). The clone / cache-materialisation half lives in
//! [`super::fetch`].

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

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
        let versions = self.list_versions(group, pkgref.name.as_str())?;
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
                name: pkgref.name.to_string(),
                req: match &pkgref.version {
                    VersionSpec::Latest => "latest".to_string(),
                    VersionSpec::Req(r) => r.to_string(),
                },
            });
        };
        Ok(ResolvedPackage {
            group: group.clone(),
            name: pkgref.name.to_string(),
            version,
            source_dir: self.package_clone_dir(group, pkgref.name.as_str()),
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
}

#[cfg(test)]
#[path = "lookup/tests.rs"]
mod tests;
