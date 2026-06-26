//! Fetch-side dispatch over the resolved source kind — override /
//! path-source / git-source / redirect short-circuits ahead of the
//! registry-served fetch, plus the override clone-and-materialise
//! path (PROP-002 §2.3 / §2.4).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

impl MultiRegistryResolver {
    /// Materialise a previously-resolved package into the per-project cache.
    /// The returned [`CachedPackage`] carries lockfile-v2 provenance
    /// (`registry_name` / `source_ref` / `overridden`) populated by the
    /// `GitPackageRegistry` impl or by the override path.
    pub fn fetch(
        &self,
        resolution: &MultiResolution,
        project_cache: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        self.fetch_with_expected_hash(resolution, project_cache, None)
    }

    /// Mirror-aware fetch with an optional cross-source content_hash gate.
    ///
    /// `expected_hash`, when supplied (typically the lockfile pin for
    /// this `(kind, name, version)`), is enforced source-by-source:
    /// each URL in the registry's primary-then-mirror chain is tried,
    /// and the first whose served content matches the pin wins. A
    /// disagreeing source is logged at `tracing::warn!` and skipped.
    /// If every source disagrees, the last one's [`CachedPackage`] is
    /// returned — its `content_hash` differs from `expected_hash`, so the
    /// caller can compare the two to detect drift against the lockfile pin.
    ///
    /// Override-resolved entries skip mirror dispatch entirely —
    /// `[[override]]` is a surgical pin to one specific URL/ref by
    /// design, so the same URL is the only legitimate source.
    pub fn fetch_with_expected_hash(
        &self,
        resolution: &MultiResolution,
        project_cache: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        if resolution.overridden {
            return self.fetch_override(resolution, project_cache);
        }
        if resolution.is_path_source {
            return self.fetch_path_source(resolution, project_cache);
        }
        if resolution.is_git_source {
            return self.fetch_git_source(resolution, project_cache, expected_hash);
        }
        if resolution.via_redirect.is_some() {
            return self.fetch_via_redirect(resolution, project_cache, expected_hash);
        }
        let registry_name =
            resolution
                .registry_name
                .as_deref()
                .ok_or_else(|| RegistryError::UnknownPackage {
                    group: resolution.resolved.group.clone(),
                    name: resolution.resolved.name.clone(),
                })?;
        let reg = self
            .registries
            .iter()
            .find(|r| r.name() == registry_name)
            .ok_or_else(|| RegistryError::UnknownPackage {
                group: resolution.resolved.group.clone(),
                name: resolution.resolved.name.clone(),
            })?;
        // `GitPackageRegistry::fetch_with_expected_hash` already populates
        // `registry_name` / `source_ref` / `overridden = false` correctly;
        // nothing to wrap.
        reg.fetch_with_expected_hash(&resolution.resolved, project_cache, expected_hash)
    }

    /// Place a registry-served `in-place` package directly into its project
    /// `slot` (PROP-022 §2.4) — a fresh clone, or an incremental `git fetch`
    /// on an existing slot — bypassing the cache clone + snapshot copy. Routes
    /// to the [`GitPackageRegistry`] that resolved the package. The special
    /// source kinds (override / git-source / path-source / redirect) are not
    /// in-place candidates; they keep the move-based snapshot path.
    pub fn materialise_in_place(
        &self,
        resolution: &MultiResolution,
        slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        let registry_name =
            resolution
                .registry_name
                .as_deref()
                .ok_or_else(|| RegistryError::UnknownPackage {
                    group: resolution.resolved.group.clone(),
                    name: resolution.resolved.name.clone(),
                })?;
        let reg = self
            .registries
            .iter()
            .find(|r| r.name() == registry_name)
            .ok_or_else(|| RegistryError::UnknownPackage {
                group: resolution.resolved.group.clone(),
                name: resolution.resolved.name.clone(),
            })?;
        reg.materialise_in_place(&resolution.resolved, slot)
    }

    fn fetch_override(
        &self,
        resolution: &MultiResolution,
        project_cache: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        let url = &resolution.source_url;
        let refname = resolution
            .source_ref
            .clone()
            .unwrap_or_else(|| DEFAULT_OVERRIDE_REF.to_string());
        let group = &resolution.resolved.group;
        let name = resolution.resolved.name.as_str();

        let clone_dir = self.override_clone_dir(group, name);
        ensure_clone_at(self.backend.as_ref(), url, &refname, &clone_dir)?;

        let dest = project_cache
            .join(group.as_str())
            .join(name)
            .join(format!("v{}", resolution.resolved.version));
        if dest.exists() {
            std::fs::remove_dir_all(&dest).map_err(|source| RegistryError::Io {
                path: dest.clone(),
                source,
            })?;
        }
        copy_dir_excluding_git(&clone_dir, &dest)?;

        let manifest_path = dest.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: "registry package manifest must carry a [package] table".to_string(),
            });
        }
        let content_hash = compute_content_hash(&dest)?;

        Ok(CachedPackage {
            resolved: ResolvedPackage {
                group: group.clone(),
                name: name.to_string(),
                version: resolution.resolved.version.clone(),
                source_dir: clone_dir,
            },
            cache_dir: dest,
            manifest,
            content_hash,
            source_uri: url.clone(),
            registry_name: None,
            source_ref: Some(refname),
            resolved_commit: None,
            overridden: true,
            is_git_source: false,
            is_path_source: false,
            via_redirect: None,
        })
    }

    /// Where override clones live —
    /// `<cache_root>/__overrides__/<group>.<name>/clone/`. Distinct
    /// directory tree from registry-served clones so a package that flips
    /// between override and registry origins on different days does not
    /// share state across modes. Keyed by `(group, name)` identity
    /// (PROP-008).
    pub(super) fn override_clone_dir(&self, group: &Group, name: &str) -> PathBuf {
        self.cache_root
            .join("__overrides__")
            .join(format!("{group}.{name}"))
            .join("clone")
    }
}
