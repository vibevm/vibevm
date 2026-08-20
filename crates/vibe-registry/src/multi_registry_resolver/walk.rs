//! The priority-ordered registry walk — override / path-source /
//! git-source short-circuits, `UnknownPackage` fall-through, the
//! auth-aware 401 classification (PROP-002 §2.3.1), and the
//! dep-manifest read that follows the resolved source kind. The
//! fetch-side dispatch lives in [`super::dispatch`].

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::attempt::format_walk_attempts;
use super::redirect_follow::try_fetch_redirect;
use super::*;

/// Read a dep manifest straight off a local-directory registry's filesystem
/// (`<root>/<group>/<name>/v<version>/vibe.toml`) — the `fetch_dep_manifest`
/// leg for a [`super::RegistrySource::Local`]. Returns `UnknownPackage` when
/// the coordinate is absent (so the walk falls through to the next source,
/// matching the git arm's `FileNotFoundInRef` / `UnknownPackage` fall-through)
/// and `MalformedMeta` when the file is unparseable.
fn read_local_dep_manifest(
    ls: &super::LocalRegistrySource,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<Manifest, RegistryError> {
    let path = ls
        .registry
        .root()
        .join(group.as_str())
        .join(name)
        .join(format!("v{version}"))
        .join(Manifest::FILENAME);
    if !path.exists() {
        return Err(RegistryError::UnknownPackage {
            group: group.clone(),
            name: name.to_string(),
        });
    }
    Manifest::read(&path).map_err(|e| RegistryError::MalformedMeta {
        path: path.clone(),
        reason: e.to_string(),
    })
}

impl MultiRegistryResolver {
    /// All versions of `(group, name)` available to this resolver — the
    /// candidate set a choosing solver (resolvo) enumerates. Override /
    /// path-source / git-source pin a single version and win over any
    /// registry copy; otherwise the priority-ordered registry walk
    /// returns the first registry's version list (same §2.3.1
    /// failure-mode discriminator as [`Self::resolve`]). A package found
    /// only behind a redirect falls back to the single resolvable version
    /// via the full `resolve` path — enumerating *every* version behind a
    /// redirect is a follow-up; `resolve` / `fetch_manifest` already
    /// follow redirects, so install never breaks on one.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver"
    )]
    pub fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        let qualified = format!("{group}/{name}");
        let pinned = self.overrides.contains_key(&qualified)
            || self.path_packages.contains_key(&qualified)
            || self.git_packages.contains_key(&qualified);

        if !pinned {
            // PROP-010 §2.6: under the offline posture the candidate
            // set is the local `file://` sources plus the machine
            // store — a git registry is a network source and is not
            // entered at all (no `ls-remote`, ever). The store's
            // contribution surfaces through the probe-resolve below,
            // which under offline resolves to the lockfile pin held in
            // the store.
            let offline_sources: Vec<&RegistrySource> = if self.offline {
                self.sources
                    .iter()
                    .filter(|s| matches!(s, RegistrySource::Local(_)))
                    .collect()
            } else {
                self.sources.iter().collect()
            };
            for src in offline_sources {
                match src {
                    super::RegistrySource::Git(reg) => match reg.list_versions(group, name) {
                        Ok(versions) if !versions.is_empty() => return Ok(versions),
                        Ok(_) => continue,
                        Err(RegistryError::UnknownPackage { .. }) => continue,
                        Err(RegistryError::Git(crate::git_backend::GitError::AuthFailed {
                            ..
                        })) if matches!(reg.auth_kind(), vibe_core::manifest::AuthKind::None)
                            && !self.strict_auth =>
                        {
                            continue;
                        }
                        Err(other) => return Err(other),
                    },
                    super::RegistrySource::Local(ls) => {
                        match ls.registry.list_versions(group, name) {
                            Ok(versions) if !versions.is_empty() => return Ok(versions),
                            Ok(_) => continue,
                            Err(RegistryError::UnknownPackage { .. }) => continue,
                            Err(other) => return Err(other),
                        }
                    }
                }
            }
        }

        // Pinned (override / path / git), redirect-only, or nothing from
        // the raw walk: defer to the full resolve path for the single
        // resolvable version, so every install mode yields a candidate.
        let probe = PackageRef::new(
            None,
            Some(group.clone()),
            name.to_string(),
            vibe_core::VersionSpec::Latest,
        )
        .map_err(|_| RegistryError::UnknownPackage {
            group: group.clone(),
            name: name.to_string(),
        })?;
        Ok(vec![self.resolve(&probe)?.resolved.version])
    }

    /// Resolve a pkgref through the override-then-registries decision tree.
    pub fn resolve(&self, pkgref: &PackageRef) -> Result<MultiResolution, RegistryError> {
        // Step 1: override short-circuit.
        if let Some(ovr) = self.overrides.get(&pkgref.qualified_name()) {
            return self.resolve_override(pkgref, ovr);
        }

        // Step 1.25: path-source short-circuit (PROP-007 §2.5).
        // `[requires.packages]` table-form may declare a dep as
        // `{ path = "..." }`; the package lives in a local directory
        // (typically a sibling workspace member). Path-source sits one
        // notch above git-source — a pkgref present in both sets
        // resolves via path-source. No registry walk, no git clone.
        if let Some(dep) = self.path_packages.get(&pkgref.qualified_name()) {
            return self.resolve_path_source(pkgref, dep);
        }

        // Step 1.5: git-source short-circuit (PROP-002 §2.4.1).
        // `[requires.packages]` table-form may declare a dep as
        // `{ git = "...", tag/branch/rev = "..." }`; the resolver
        // bypasses the `[[registry]]` walk for that pkgref entirely
        // and fetches directly from the declared URL.
        if let Some(dep) = self.git_packages.get(&pkgref.qualified_name()) {
            return self.resolve_git_source(pkgref, dep);
        }

        // Step 2: priority-ordered registry walk. PROP-002 §2.3.1
        // failure-mode discriminator:
        //
        // - `UnknownPackage` → fall through to next registry.
        // - `Git(AuthFailed)` on an `auth = "none"` registry →
        //   reclassify as `UnknownPackage` and fall through. (For
        //   public registries 401 / 403 means "no public answer
        //   here", e.g. GitVerse's policy on missing repos.)
        // - `Git(AuthFailed)` on an authenticated registry
        //   (`token-env`, `credential-helper`) → halt with the
        //   error — the operator declared this registry expects
        //   credentials, the credentials presented were rejected,
        //   the operator must see that.
        // - `MissingToken` on any registry → halt — the manifest
        //   declared `auth = "token-env"` but the env-var is unset;
        //   the operator must fix the env. (Walking past this would
        //   silently downgrade a private registry to "not present"
        //   which would mask configuration errors.)
        // - any other error → halt as before (network, malformed
        //   manifest, server error, ...).
        // A registry resolves by `(group, name)` identity (PROP-008 §2.2);
        // a pkgref reaching the registry walk without a `group` is an
        // `UnqualifiedPkgref` — short names are qualified at the CLI
        // boundary, never here.
        let group = pkgref
            .group
            .as_ref()
            .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;

        // Step 1.7: the offline posture (PROP-010 §2.6,
        // `RESOLVER-OFFLINE-MODE`). The override / path-source /
        // git-source short-circuits above are their own mechanisms
        // (declared-URL or local-dir); the registry walk from here on
        // would reach the network, so under `offline` it is replaced
        // wholesale: local `file://` sources plus the machine store,
        // nothing else — no `git fetch` / `ls-remote` / archive at all.
        if self.offline {
            return self.resolve_offline(pkgref, group);
        }

        let mut attempts: Vec<RegistryWalkAttempt> = Vec::new();
        for src in &self.sources {
            match src {
                RegistrySource::Git(reg) => match reg.resolve(pkgref) {
                    Ok(resolved) => {
                        let stub_tag = format!("v{}", resolved.version);
                        // Step 2a: redirect probe (PROP-002 §2.4.2). The
                        // registry served a tag; check whether the repo
                        // at that tag is a stub pointing elsewhere. The
                        // probe is one extra `git archive` call, only
                        // when the registry-walk leg succeeded; cheap.
                        if let Some(redirect) =
                            try_fetch_redirect(&self.backend, reg, &resolved, &stub_tag)?
                        {
                            return self
                                .follow_redirect(pkgref, &resolved, reg, &redirect, &stub_tag);
                        }
                        let url = reg.package_repo_url(&resolved.group, &resolved.name)?;
                        return Ok(MultiResolution {
                            resolved,
                            registry_name: Some(reg.name().to_string()),
                            source_url: url,
                            source_ref: Some(stub_tag),
                            overridden: false,
                            is_git_source: false,
                            is_path_source: false,
                            via_redirect: None,
                            from_store: false,
                            redirect_target_auth: vibe_core::manifest::AuthKind::None,
                            redirect_target_token_env: None,
                        });
                    }
                    Err(RegistryError::UnknownPackage { .. }) => {
                        attempts.push(RegistryWalkAttempt {
                            name: reg.name().to_string(),
                            url: reg.org_url().to_string(),
                            auth: reg.auth_kind(),
                            status: WalkAttemptStatus::NotFound,
                        });
                        continue;
                    }
                    Err(err @ RegistryError::NoMatchingVersion { .. }) => {
                        // PROP-010 §2.6 — "no such version" from a
                        // registry is not the last word: a store hit is
                        // authoritative for availability. Only this
                        // absence shape consults the store; operational
                        // failures below never do, so nothing is masked.
                        if let Some(resolution) = self.store_availability_fallback(pkgref, group) {
                            return Ok(resolution);
                        }
                        return Err(err);
                    }
                    Err(RegistryError::Git(crate::git_backend::GitError::AuthFailed {
                        ..
                    })) if matches!(reg.auth_kind(), vibe_core::manifest::AuthKind::None)
                        && !self.strict_auth =>
                    {
                        tracing::debug!(
                            target: "vibe_registry::resolve",
                            registry = %reg.name(),
                            "auth_failed on auth=none registry treated as unknown-package; walking"
                        );
                        attempts.push(RegistryWalkAttempt {
                            name: reg.name().to_string(),
                            url: reg.org_url().to_string(),
                            auth: reg.auth_kind(),
                            status: WalkAttemptStatus::Public401,
                        });
                        continue;
                    }
                    Err(other) => return Err(other),
                },
                // A local-directory registry: resolve straight off the
                // filesystem — no redirect probe, no per-package repo URL,
                // no auth. The source is the directory itself, recorded as
                // the lockfile `source_url` (a `file://` / path string) with
                // no `source_ref` (there is no git ref).
                RegistrySource::Local(ls) => match ls.registry.resolve(pkgref) {
                    Ok(resolved) => {
                        return Ok(MultiResolution {
                            resolved,
                            registry_name: Some(ls.name.clone()),
                            source_url: ls.url.clone(),
                            source_ref: None,
                            overridden: false,
                            is_git_source: false,
                            is_path_source: false,
                            via_redirect: None,
                            from_store: false,
                            redirect_target_auth: vibe_core::manifest::AuthKind::None,
                            redirect_target_token_env: None,
                        });
                    }
                    Err(RegistryError::UnknownPackage { .. }) => {
                        attempts.push(RegistryWalkAttempt {
                            name: ls.name.clone(),
                            url: ls.url.clone(),
                            auth: vibe_core::manifest::AuthKind::None,
                            status: WalkAttemptStatus::NotFound,
                        });
                        continue;
                    }
                    Err(err @ RegistryError::NoMatchingVersion { .. }) => {
                        // Same absence shape as the git branch above:
                        // the store outranks a registry that no longer
                        // lists the version (PROP-010 §2.6); the
                        // original error survives byte-for-byte when
                        // the store cannot serve.
                        if let Some(resolution) = self.store_availability_fallback(pkgref, group) {
                            return Ok(resolution);
                        }
                        return Err(err);
                    }
                    Err(other) => return Err(other),
                },
            }
        }

        // No registry had a satisfying answer — but a store hit is
        // authoritative for availability even here (PROP-010 §2.6,
        // `A-CACHE-HIT-IS-AUTHORITATIVE-FOR-AVAILABILITY`): the store
        // holds bytes already verified against a lockfile pin, and a
        // registry that no longer lists a version has only described
        // ITS inventory, not the bytes on this disk. Only the absence
        // shapes reach this point — operational errors halted above —
        // so nothing real is masked.
        if let Some(resolution) = self.store_availability_fallback(pkgref, group) {
            return Ok(resolution);
        }

        // No registry had a satisfying answer. Two shapes:
        //
        // - If we walked at least one registry, surface the
        //   aggregate per-registry status so the operator sees
        //   exactly what happened where (PackageNotFoundEverywhere).
        // - Otherwise (no `[[registry]]` configured) fall back to
        //   the simpler UnknownPackage for back-compat with
        //   downstream consumers that match on it specifically.
        if attempts.is_empty() {
            return Err(RegistryError::UnknownPackage {
                group: group.clone(),
                name: pkgref.name.to_string(),
            });
        }
        let summary = format_walk_attempts(&attempts);
        Err(RegistryError::PackageNotFoundEverywhere {
            group: group.clone(),
            name: pkgref.name.to_string(),
            summary,
            attempts,
        })
    }

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
        let resolution = match self.resolve(&pinned_pkgref) {
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
                let r = self.resolve(&fallback_pkgref)?;
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

        if resolution.from_store {
            // Store-backed (offline posture or availability fallback,
            // PROP-010 §2.6): the entry IS the manifest's home — the
            // layout is the index (§2.7) — so transitive-dependency
            // manifests read straight off disk. No network, no clone.
            let manifest_path = resolution.resolved.source_dir.join(Manifest::FILENAME);
            return Manifest::read(&manifest_path).map_err(RegistryError::from);
        }

        if resolution.is_path_source {
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
            // Redirect-resolved: target_url is in source_url, target_ref
            // is in source_ref. Open a synthetic single-package
            // registry on the target and read the manifest at the
            // recorded ref. Auth carries the redirect's declared
            // policy so private targets keep working.
            let target_url = resolution.source_url.clone();
            let target_ref = resolution.source_ref.clone().unwrap_or_default();
            let synthetic_name = format!("redirect-target-{group}-{name}");
            let target_reg = GitPerPackageRegistry::open_single_package(
                &synthetic_name,
                &target_url,
                &target_ref,
                &self.cache_root,
                Arc::clone(&self.backend),
                DEFAULT_FRESHNESS_SECS,
                resolution.redirect_target_auth,
                resolution.redirect_target_token_env.as_deref(),
            )?;
            return target_reg.fetch_manifest_at_ref(group, name, &target_ref);
        }

        if resolution.is_git_source {
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
                .sources
                .iter()
                .find(|s| s.name() == name_filter.as_str())
        {
            return match src {
                RegistrySource::Git(reg) => reg.fetch_dep_manifest(group, name, version),
                RegistrySource::Local(ls) => read_local_dep_manifest(ls, group, name, version),
            };
        }
        let mut last_err: Option<RegistryError> = None;
        for src in &self.sources {
            let tried = match src {
                RegistrySource::Git(reg) => reg.fetch_dep_manifest(group, name, version),
                RegistrySource::Local(ls) => read_local_dep_manifest(ls, group, name, version),
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

#[cfg(test)]
#[path = "walk/tests.rs"]
mod tests;
