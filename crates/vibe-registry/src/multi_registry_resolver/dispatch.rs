//! Fetch-side dispatch over the resolved source kind — override /
//! path-source / git-source / redirect short-circuits ahead of the
//! registry-served fetch, plus the override clone-and-materialise
//! path (PROP-002 §2.3 / §2.4).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

impl MultiRegistryResolver {
    /// Consume the already selected source once, preserving special/local routes.
    pub fn fetch_for_install(
        &self,
        resolution: &MultiResolution,
        store_root: &Path,
        expected_hash: Option<&str>,
        slot: &Path,
        progress: &vibe_core::progress::Progress,
    ) -> Result<crate::PackageAcquisition, RegistryError> {
        if let Some(locked) = &resolution.locked {
            return self.fetch_locked(resolution, locked, store_root, Some(slot));
        }
        if resolution.direct.is_some() {
            return self.acquire_direct_source(resolution, store_root, expected_hash, Some(slot));
        }
        if resolution.via_redirect.is_some() && !resolution.from_store {
            return self.acquire_redirect(resolution, store_root, expected_hash, Some(slot));
        }
        if !resolution.from_store
            && !resolution.overridden
            && !resolution.is_path_source
            && !resolution.is_git_source
            && resolution.via_redirect.is_none()
            && let Some(RegistrySource::Git(registry)) = self
                .sources()?
                .iter()
                .find(|source| Some(source.name()) == resolution.registry_name.as_deref())
        {
            let task = progress.task(format!(
                "Fetching {}/{}@{}",
                resolution.resolved.group, resolution.resolved.name, resolution.resolved.version
            ));
            let package = format!("{}/{}", resolution.resolved.group, resolution.resolved.name);
            let result = crate::git_backend::transfers::observed(
                Some(&task.progress()),
                Some(&package),
                Some(vibe_core::progress::transfer::Purpose::PackageClone),
                || {
                    registry.fetch_on_route(
                        &resolution.resolved,
                        store_root,
                        expected_hash,
                        Some(slot),
                        resolution.selected_route.as_ref(),
                    )
                },
            );
            if result.is_ok() {
                task.finish()
            } else {
                task.fail("package fetch failed")
            };
            return result;
        }
        self.fetch_with_expected_hash_progress(resolution, store_root, expected_hash, progress)
            .map(|cached| crate::PackageAcquisition::Cached(Box::new(cached)))
    }
    /// Fetch a previously-resolved package into the machine-global
    /// store. The returned [`CachedPackage`] carries lockfile-v2
    /// provenance (`registry_name` / `source_ref` / `overridden`)
    /// populated by the `GitPerPackageRegistry` impl or by the
    /// override path.
    pub fn fetch(
        &self,
        resolution: &MultiResolution,
        store_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        self.fetch_with_expected_hash(resolution, store_root, None)
    }

    /// Mirror-aware fetch with an optional cross-source content_hash
    /// gate, inserting the accepted payload into the machine-global
    /// store (PROP-010 §2.7).
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
        store_root: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        self.fetch_with_expected_hash_progress(
            resolution,
            store_root,
            expected_hash,
            &self.progress,
        )
    }

    /// Observed sibling used by install's per-package fetch loop.
    pub fn fetch_with_expected_hash_progress(
        &self,
        resolution: &MultiResolution,
        store_root: &Path,
        expected_hash: Option<&str>,
        progress: &vibe_core::progress::Progress,
    ) -> Result<CachedPackage, RegistryError> {
        let label = format!(
            "Fetching {}/{}@{}",
            resolution.resolved.group, resolution.resolved.name, resolution.resolved.version
        );
        let task = progress.task(label);
        if resolution.from_store {
            task.detail("reusing machine package cache");
        } else if resolution.is_path_source {
            task.detail("reading local path source");
        } else if resolution.is_git_source {
            task.detail("Git fetch of declared source");
        } else if let Some(registry) = resolution.registry_name.as_deref() {
            task.detail(format!("Git fetch from registry {registry}"));
        }
        // The clone this fetch makes is a transfer of this task, for this
        // package (`PROP-068#git`).
        let package = format!("{}/{}", resolution.resolved.group, resolution.resolved.name);
        let result = crate::git_backend::transfers::observed(
            Some(&task.progress()),
            Some(&package),
            Some(vibe_core::progress::transfer::Purpose::PackageClone),
            || self.fetch_with_expected_hash_inner(resolution, store_root, expected_hash),
        );
        match &result {
            Ok(_) if resolution.from_store => task.skip("already cached"),
            Ok(_) => task.finish(),
            Err(_) => task.fail("package fetch failed"),
        }
        result
    }

    fn fetch_with_expected_hash_inner(
        &self,
        resolution: &MultiResolution,
        store_root: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        if let Some(locked) = &resolution.locked {
            return match self.fetch_locked(resolution, locked, store_root, None)? {
                crate::PackageAcquisition::Cached(cached) => Ok(*cached),
                crate::PackageAcquisition::InPlace(_) => Err(RegistryError::MalformedMeta {
                    path: PathBuf::from(Lockfile::FILENAME),
                    reason: "cache acquisition returned a workspace-bound preparation".into(),
                }),
            };
        }
        if resolution.direct.is_some() {
            return match self.acquire_direct_source(resolution, store_root, expected_hash, None)? {
                crate::PackageAcquisition::Cached(cached) => Ok(*cached),
                crate::PackageAcquisition::InPlace(_) => Err(RegistryError::MalformedMeta {
                    path: PathBuf::from("direct-source"),
                    reason: "cache acquisition returned workspace preparation".into(),
                }),
            };
        }
        // Store-backed resolution (PROP-010 §2.6): the entry IS the
        // content source — serve its bytes off disk, with no source
        // walk and no network; a pin, when present, is verified with
        // the same entry gate the fetch path applies.
        if resolution.from_store {
            return self.fetch_from_store(resolution, expected_hash);
        }
        if resolution.overridden {
            return self.fetch_override(resolution, store_root);
        }
        if resolution.is_path_source {
            return self.fetch_path_source(resolution, store_root);
        }
        if resolution.is_git_source {
            return self.fetch_git_source(resolution, store_root, expected_hash);
        }
        if resolution.via_redirect.is_some() {
            return self.fetch_via_redirect(resolution, store_root, expected_hash);
        }
        let registry_name =
            resolution
                .registry_name
                .as_deref()
                .ok_or_else(|| RegistryError::UnknownPackage {
                    group: resolution.resolved.group.clone(),
                    name: resolution.resolved.name.clone(),
                })?;
        let src = self
            .sources()?
            .iter()
            .find(|s| s.name() == registry_name)
            .ok_or_else(|| RegistryError::UnknownPackage {
                group: resolution.resolved.group.clone(),
                name: resolution.resolved.name.clone(),
            })?;
        // Dispatch on the source kind: a git registry fetches by clone +
        // mirror-aware hash gate; a local-directory registry copies straight
        // off the filesystem via `LocalRegistry::fetch` (no git, no ref).
        // `LocalRegistry::fetch` already populates the `CachedPackage`
        // provenance correctly for a registry-served local source.
        match src {
            RegistrySource::Git(reg) => {
                match reg.fetch_on_route(
                    &resolution.resolved,
                    store_root,
                    expected_hash,
                    None,
                    resolution.selected_route.as_ref(),
                )? {
                    crate::PackageAcquisition::Cached(cached) => Ok(*cached),
                    crate::PackageAcquisition::InPlace(prepared) => {
                        Err(RegistryError::PreparedInPlace {
                            path: prepared.slot().into(),
                            reason: "cache fetch cannot return a workspace-bound candidate".into(),
                        })
                    }
                }
            }
            RegistrySource::Local(ls) => {
                let mut cached = ls.registry.fetch(&resolution.resolved, store_root)?;
                // The directory fetch owner has no catalog alias; this resolver does.
                cached.registry_name = resolution.registry_name.clone();
                Ok(cached)
            }
            // A monorepo package is a subdirectory of the operation's clone;
            // the fetch records registry, branch and commit (PROP-002
            // `#registry-type`).
            RegistrySource::Monorepo(mono) => {
                let mut cached =
                    crate::Registry::fetch(mono.registry()?, &resolution.resolved, store_root)?;
                cached.registry_name = resolution.registry_name.clone();
                Ok(cached)
            }
        }
    }

    /// Prepare a registry-served in-place candidate without changing its slot.
    /// Special sources keep their existing non-in-place materialization routes.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION"
    )]
    pub fn prepare_in_place(
        &self,
        resolution: &MultiResolution,
        slot: &Path,
    ) -> Result<crate::PreparedInPlace, RegistryError> {
        if let Some(locked) = &resolution.locked {
            return self.prepare_locked(resolution, locked, slot);
        }
        if resolution.direct.is_some() {
            return self.prepare_direct_source(resolution, slot);
        }
        if resolution.via_redirect.is_some()
            && !resolution.from_store
            && self
                .backend
                .native_materialisation_for(&resolution.source_url)?
        {
            return self.prepare_redirect(resolution, slot);
        }
        if resolution.overridden
            || resolution.is_path_source
            || resolution.is_git_source
            || resolution.via_redirect.is_some()
            || resolution.from_store
        {
            return Err(RegistryError::PreparedInPlace { path: slot.to_path_buf(), reason: "this special source route has no registry-bound prepared in-place identity; its existing non-in-place route remains available".into() });
        }
        let registry = resolution.registry_name.as_deref();
        let source = self
            .sources()?
            .iter()
            .find(|source| Some(source.name()) == registry)
            .ok_or_else(|| RegistryError::UnknownPackage {
                group: resolution.resolved.group.clone(),
                name: resolution.resolved.name.clone(),
            })?;
        match source {
            RegistrySource::Git(registry) => registry.prepare_in_place_on_route(
                &resolution.resolved,
                slot,
                resolution.selected_route.as_ref(),
            ),
            RegistrySource::Local(_) | RegistrySource::Monorepo(_) => {
                Err(RegistryError::InPlaceUnsupported {
                    group: resolution.resolved.group.clone(),
                    name: resolution.resolved.name.clone(),
                })
            }
        }
    }

    /// Compatibility placement API. Pre-effect admission uses `prepare_in_place`
    /// and applies its retained candidate without resolving the source again.
    pub fn materialise_in_place(
        &self,
        resolution: &MultiResolution,
        slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        if let Some(locked) = &resolution.locked {
            return self.prepare_locked(resolution, locked, slot)?.materialise();
        }
        if resolution.direct.is_some() {
            return self.prepare_direct_source(resolution, slot)?.materialise();
        }
        let registry_name =
            resolution
                .registry_name
                .as_deref()
                .ok_or_else(|| RegistryError::UnknownPackage {
                    group: resolution.resolved.group.clone(),
                    name: resolution.resolved.name.clone(),
                })?;
        let src = self
            .sources()?
            .iter()
            .find(|s| s.name() == registry_name)
            .ok_or_else(|| RegistryError::UnknownPackage {
                group: resolution.resolved.group.clone(),
                name: resolution.resolved.name.clone(),
            })?;
        // In-place needs a git source to clone and incrementally update; a
        // local-directory registry has no git backend (PROP-022 §2.4), so it
        // is `InPlaceUnsupported` — same posture as `--registry <path>`.
        match src {
            RegistrySource::Git(reg) => reg.materialise_in_place(&resolution.resolved, slot),
            RegistrySource::Local(_) | RegistrySource::Monorepo(_) => {
                Err(RegistryError::InPlaceUnsupported {
                    group: resolution.resolved.group.clone(),
                    name: resolution.resolved.name.clone(),
                })
            }
        }
    }

    pub(super) fn fetch_override(
        &self,
        resolution: &MultiResolution,
        store_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        let url = &resolution.source_url;
        let refname = resolution
            .source_ref
            .clone()
            .unwrap_or_else(|| DEFAULT_OVERRIDE_REF.to_string());
        let group = &resolution.resolved.group;
        let name = resolution.resolved.name.as_str();

        let clone_dir = self.override_clone_dir(group, name);
        if let Some(lease) = &resolution.direct {
            lease
                .registry()
                .ensure_selected_clone(&refname, &clone_dir)?;
        } else {
            ensure_clone_at(self.backend.as_ref(), url, &refname, &clone_dir)?;
        }
        if let Some(lease) = &resolution.direct
            && Manifest::read(clone_dir.join(Manifest::FILENAME))? != *lease.manifest()
        {
            return Err(RegistryError::MalformedMeta {
                path: clone_dir.join(Manifest::FILENAME),
                reason: "override manifest changed after selection".into(),
            });
        }

        // Hash the clone directly (the recipe skips `.git`), then insert
        // write-once into the machine store — an already-present entry is
        // returned untouched and its manifest read back, so the
        // `CachedPackage` describes the bytes that will materialise.
        let content_hash = compute_content_hash(&clone_dir)?;
        let inserted = store::insert_current_at(
            store_root,
            &clone_dir,
            group,
            name,
            &resolution.resolved.version,
            &content_hash,
        )?;
        let reused = matches!(&inserted, store::InsertOutcome::AlreadyPresent(_));
        let dest = inserted.into_entry();

        let manifest_path = dest.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: "registry package manifest must carry a [package] table".to_string(),
            });
        }

        let resolved_commit = self.backend.head_commit(&clone_dir)?;
        Ok(CachedPackage {
            locked_selection: None,
            route_publication: vibe_core::RoutePublication::Local,
            recorded_source_kind: None,
            store_reused: Some(reused),
            acquisition: Some(vibe_wire::generated::shared::SourceAcquisition::GitRepository),
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
            // Nothing redirected this fetch (`PROP-066#provenance`).
            fetched_from: None,
            registry_name: None,
            source_ref: Some(refname),
            resolved_commit,
            overridden: true,
            is_git_source: false,
            is_path_source: false,
            is_embedded: false,
            is_local: false,
            via_redirect: None,
            template: None,
            via_channel: None,
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
