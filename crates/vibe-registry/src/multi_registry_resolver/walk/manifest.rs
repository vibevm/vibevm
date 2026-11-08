//! Existing metadata lookup semantics, separated from source-scoped selection.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");
use super::*;
impl MultiRegistryResolver {
    /// Read `vibe.toml` for a resolved `(group, name, version)`,
    /// transparently following any registry-redirect stub (PROP-002
    /// §2.4.2) or git-source declaration (§2.4.1). The depsolver's
    /// [`DepProvider::fetch_manifest`] adapter uses this so a
    /// stub-served pkgref returns the **target's** manifest (the stub
    /// itself carries only `vibe-redirect.toml`) and a git-source
    /// pkgref returns the manifest at the declared `tag`/`branch`/`rev`.
    ///
    /// The implementation re-runs [`Self::resolve`] with the version
    /// constraint pinned to `=<version>` so it converges on the same
    /// `MultiResolution` the install pipeline already saw, then reads
    /// the manifest from whichever URL the resolution recorded —
    /// stub's target for redirects, declared URL for git-source,
    /// the registry's own URL otherwise. Walking registries directly
    /// (the pre-M1.16 shape) cannot serve a stub-only repo.
    ///
    /// Keyed by `(group, name)` identity (PROP-008) — `kind` is metadata,
    /// read off the resolved manifest.
    pub fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, RegistryError> {
        observation::manifest(&self.progress, group, name, version, |task| {
            self.fetch_manifest_inner(group, name, version, task)
        })
    }

    fn fetch_manifest_inner(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
        task: &vibe_core::progress::ProgressTask,
    ) -> Result<Manifest, RegistryError> {
        // Build a pinned pkgref so `resolve` converges on the exact
        // slot the install pipeline committed to (the depsolver pinned
        // the version via `resolve_version` first). For pass-through
        // redirects (and direct registry installs) the stub's tag list
        // contains `v<version>` and the pinned resolve hits it
        // immediately. For pinned-policy redirects the stub may have
        // unrelated tags (the pinned semantic — every consumer goes
        // to the target's pinned ref, so the stub tag is irrelevant);
        // we fall back to a constraint-free resolve and verify the
        // resolved version still matches.
        let pinned_pkgref =
            PackageRef::parse(&format!("{group}/{name}@={version}")).map_err(|e| {
                RegistryError::MalformedMeta {
                    path: PathBuf::from("<synthetic-pkgref>"),
                    reason: format!("constructing pinned pkgref: {e}"),
                }
            })?;
        let resolution = match self.resolve_inner(&pinned_pkgref, task) {
            Ok(r) => r,
            Err(RegistryError::NoMatchingVersion { .. })
            | Err(RegistryError::PackageNotFoundEverywhere { .. })
            | Err(RegistryError::UnknownPackage { .. }) => {
                // The stub's tag list does not contain `=version` —
                // happens with pinned-policy redirects where the
                // stub-side tag and the target version are decoupled.
                // Re-resolve without a constraint and accept the
                // result as long as the version it produces matches
                // what the depsolver pinned.
                let fallback_pkgref =
                    PackageRef::parse(&format!("{group}/{name}")).map_err(|e| {
                        RegistryError::MalformedMeta {
                            path: PathBuf::from("<synthetic-pkgref>"),
                            reason: format!("constructing latest pkgref: {e}"),
                        }
                    })?;
                task.detail("exact metadata version absent; probing latest for a pinned redirect and verifying the requested version");
                let r = self.resolve_inner(&fallback_pkgref, task)?;
                if &r.resolved.version != version {
                    return Err(RegistryError::NoMatchingVersion {
                        group: group.clone(),
                        name: name.to_string(),
                        req: format!("={version}"),
                    });
                }
                r
            }
            Err(other) => return Err(other),
        };

        if let Some(locked) = &resolution.locked {
            return self.locked_manifest(&resolution, locked);
        }
        if let Some(direct) = &resolution.direct {
            return Ok(direct.manifest().clone());
        }
        if resolution.from_store {
            task.detail("reading metadata from machine cache");
            // Store-backed (offline posture or availability fallback,
            // PROP-010 §2.6): the entry IS the manifest's home — the
            // layout is the index (§2.7) — so transitive-dependency
            // manifests read straight off disk. No network, no clone.
            let manifest_path = resolution.resolved.source_dir.join(Manifest::FILENAME);
            return Manifest::read(&manifest_path).map_err(RegistryError::from);
        }

        if resolution.is_path_source {
            task.detail("reading metadata from local path source");
            // Path-source: the package lives in a local directory.
            // `path_packages` carries the resolver-side `package_dir`
            // (already canonicalised by the workspace layer); read
            // `vibe.toml` straight off disk so transitive dependencies
            // of a path-source package resolve.
            let dep = self
                .path_packages
                .get(&pinned_pkgref.qualified_name())
                .ok_or_else(|| RegistryError::UnknownPackage {
                    group: group.clone(),
                    name: name.to_string(),
                })?;
            let manifest_path = dep.package_dir.join(Manifest::FILENAME);
            return Manifest::read(&manifest_path).map_err(RegistryError::from);
        }

        if resolution.via_redirect.is_some() {
            task.detail("reading metadata from redirect target");
            // Redirect-resolved: target_url is in source_url, target_ref
            // is in source_ref. Open a synthetic single-package
            // registry on the target and read the manifest at the
            // recorded ref. Auth carries the redirect's declared
            // policy so private targets keep working.
            let target_url = resolution.source_url.clone();
            let target_ref = resolution.source_ref.clone().unwrap_or_default();
            let target_reg = self.redirect_target_registry(
                group,
                name,
                &target_url,
                &target_ref,
                resolution.redirect_target_auth,
                resolution.redirect_target_token_env.as_deref(),
            )?;
            return target_reg.fetch_manifest_at_ref(group, name, &target_ref);
        }

        if resolution.is_git_source {
            task.detail("reading metadata from declared Git source");
            // Git-source: source_url + source_ref carry the operator-
            // declared `tag`/`branch`/`rev`. Construct the same
            // synthetic registry the resolver used and re-read the
            // manifest at that ref.
            //
            // Note: `git_packages` lookup gives us the original
            // `auth` / `token_env`; the resolver did the lookup at
            // `resolve` time and stored the values there too.
            let dep = self
                .git_packages
                .get(&pinned_pkgref.qualified_name())
                .ok_or_else(|| RegistryError::UnknownPackage {
                    group: group.clone(),
                    name: name.to_string(),
                })?;
            let source_ref = resolution
                .source_ref
                .clone()
                .unwrap_or_else(|| dep.ref_kind.as_str().to_string());
            let synthetic_name = format!("git-source-{group}-{name}");
            let reg = GitPerPackageRegistry::open_single_package(
                &synthetic_name,
                &dep.url,
                &source_ref,
                &self.cache_root,
                Arc::clone(&self.backend),
                DEFAULT_FRESHNESS_SECS,
                dep.auth,
                dep.token_env.as_deref(),
            )?;
            return reg.fetch_manifest_at_ref(group, name, &source_ref);
        }

        // Override or registry: walk in priority order, preferring the
        // registry the resolver picked. Override-served packages have
        // `registry_name = None`, so we just walk and the first match
        // wins (overrides are not consulted by `fetch_dep_manifest` —
        // those are handled by the install pipeline directly).
        if let Some(name_filter) = &resolution.registry_name
            && let Some(src) = self
                .sources()?
                .iter()
                .find(|s| s.name() == name_filter.as_str())
        {
            task.detail(format!("reading metadata from registry {name_filter}"));
            return match src {
                RegistrySource::Git(reg) => reg.fetch_dep_manifest_selected(
                    &resolution.resolved,
                    resolution.selected_route.as_ref(),
                ),
                RegistrySource::Local(ls) => {
                    read_local_dep_manifest(&ls.registry, group, name, version)
                }
                RegistrySource::Monorepo(mono) => {
                    read_local_dep_manifest(mono.local()?, group, name, version)
                }
            };
        }
        let mut last_err: Option<RegistryError> = None;
        for src in self.sources()? {
            let tried = match src {
                RegistrySource::Git(reg) => reg.fetch_dep_manifest_selected(
                    &resolution.resolved,
                    resolution.selected_route.as_ref(),
                ),
                RegistrySource::Local(ls) => {
                    read_local_dep_manifest(&ls.registry, group, name, version)
                }
                RegistrySource::Monorepo(mono) => mono
                    .local()
                    .and_then(|local| read_local_dep_manifest(local, group, name, version)),
            };
            match tried {
                Ok(m) => return Ok(m),
                Err(err)
                    if matches!(
                        err,
                        RegistryError::Git(GitError::FileNotFoundInRef { .. })
                            | RegistryError::Git(GitError::ArchiveUnsupported { .. })
                            | RegistryError::Io { .. }
                            | RegistryError::MalformedMeta { .. }
                            | RegistryError::UnknownPackage { .. }
                            | RegistryError::NoMatchingVersion { .. }
                    ) =>
                {
                    last_err = Some(err);
                    continue;
                }
                Err(other) => return Err(other),
            }
        }
        Err(last_err.unwrap_or(RegistryError::UnknownPackage {
            group: group.clone(),
            name: name.to_string(),
        }))
    }
}
