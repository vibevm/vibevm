//! Held source selection precedes the priority-ordered walk for new names.
//! Fetch dispatch lives in [`super::dispatch`].

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::attempt::format_walk_attempts;
use super::redirect_follow::try_fetch_redirect;
use super::*;

mod local;
mod manifest;
mod negative;
mod observation;
use local::read_local_dep_manifest;
pub(super) use negative::RemoteMisses;
pub use observation::ResolutionPurpose;

impl MultiRegistryResolver {
    /// Enumerate the held version first. New sources retain the declared-source
    /// and ordered-registry candidate algorithm; redirect-only sources expose
    /// their single resolvable version through the same resolution owner.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver"
    )]
    pub fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        observation::versions(&self.progress, group, name, |task| {
            self.list_versions_inner(group, name, task)
        })
    }

    fn list_versions_inner(
        &self,
        group: &Group,
        name: &str,
        task: &vibe_core::progress::ProgressTask,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        self.list_versions_in_registry(group, name, task, None)
    }

    pub(super) fn list_versions_in_registry(
        &self,
        group: &Group,
        name: &str,
        task: &vibe_core::progress::ProgressTask,
        registry_filter: Option<&str>,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        let qualified = format!("{group}/{name}");
        if let Some(locked) = self.locked.get(&qualified) {
            return self.locked_versions(locked);
        }
        let pinned = self.overrides()?.contains_key(&qualified)
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
            let selected = registry_filter
                .map(|name| self.selected_registry_source(name))
                .transpose()?;
            let sources: Vec<&RegistrySource> = match &selected {
                Some(source) => vec![source.as_ref()],
                None => self.sources()?.iter().collect(),
            };
            for src in sources
                .into_iter()
                .filter(|source| !self.offline || matches!(source, RegistrySource::Local(_)))
            {
                match src {
                    super::RegistrySource::Git(reg) => {
                        match self
                            .ask_remote(reg, group, name, task, || reg.list_versions(group, name))
                        {
                            Ok(versions) if !versions.is_empty() => return Ok(versions),
                            Ok(_) => continue,
                            Err(RegistryError::UnknownPackage { .. }) => continue,
                            Err(RegistryError::Git(crate::git_backend::GitError::AuthFailed {
                                ..
                            })) if matches!(
                                reg.auth_kind(),
                                vibe_core::manifest::AuthKind::None
                            ) && !self.strict_auth =>
                            {
                                continue;
                            }
                            Err(other) => return Err(other),
                        }
                    }
                    super::RegistrySource::Local(ls) => {
                        task.detail(format!("checking local registry {}", ls.name));
                        match ls.registry.list_versions(group, name) {
                            Ok(versions) if !versions.is_empty() => return Ok(versions),
                            Ok(_) => continue,
                            Err(RegistryError::UnknownPackage { .. }) => continue,
                            Err(other) => return Err(other),
                        }
                    }
                    super::RegistrySource::Monorepo(mono) => {
                        task.detail(format!("checking monorepo registry {}", mono.name));
                        match mono
                            .local()
                            .and_then(|local| local.list_versions(group, name))
                        {
                            Ok(versions) if !versions.is_empty() => return Ok(versions),
                            Ok(_) => continue,
                            Err(RegistryError::UnknownPackage { .. }) => continue,
                            Err(RegistryError::Git(crate::git_backend::GitError::AuthFailed {
                                ..
                            })) if mono.auth == vibe_core::manifest::AuthKind::None
                                && !self.strict_auth =>
                            {
                                continue;
                            }
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
        task.detail("resolving a single candidate after version-list discovery");
        Ok(vec![
            self.resolve_in_registry(&probe, task, registry_filter)?
                .resolved
                .version,
        ])
    }

    /// Resolve a pkgref through the override-then-registries decision tree.
    pub fn resolve(&self, pkgref: &PackageRef) -> Result<MultiResolution, RegistryError> {
        self.resolve_for(pkgref, ResolutionPurpose::VersionSelection)
    }

    fn resolve_inner(
        &self,
        pkgref: &PackageRef,
        task: &vibe_core::progress::ProgressTask,
    ) -> Result<MultiResolution, RegistryError> {
        self.resolve_in_registry(pkgref, task, None)
    }

    pub(super) fn resolve_in_registry(
        &self,
        pkgref: &PackageRef,
        task: &vibe_core::progress::ProgressTask,
        registry_filter: Option<&str>,
    ) -> Result<MultiResolution, RegistryError> {
        if let Some(locked) = self.locked.get(&pkgref.qualified_name()) {
            return self.resolve_locked(pkgref, locked);
        }
        if let Some(ovr) = self.overrides()?.get(&pkgref.qualified_name()) {
            task.detail("using declared override source");
            return self.resolve_override(pkgref, ovr);
        }

        // `[requires.packages]` table-form may declare a dep as
        // `{ path = "..." }`; the package lives in a local directory
        // (typically a sibling workspace member). Path-source sits one
        // notch above git-source — a pkgref present in both sets
        // resolves via path-source. No registry walk, no git clone.
        if let Some(dep) = self.path_packages.get(&pkgref.qualified_name()) {
            task.detail("using local path source");
            return self.resolve_path_source(pkgref, dep);
        }

        // `[requires.packages]` table-form may declare a dep as
        // `{ git = "...", tag/branch/rev = "..." }`; the resolver
        // bypasses the `[[registry]]` walk for that pkgref entirely
        // and fetches directly from the declared URL.
        if let Some(dep) = self.git_packages.get(&pkgref.qualified_name()) {
            task.detail("using declared Git source");
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
        if self.offline && registry_filter.is_none() {
            task.detail("offline: checking local registries and machine cache");
            return self.resolve_offline(pkgref, group);
        }

        let mut attempts: Vec<RegistryWalkAttempt> = Vec::new();
        let selected = registry_filter
            .map(|name| self.selected_registry_source(name))
            .transpose()?;
        let sources: Vec<&RegistrySource> = match &selected {
            Some(source) => vec![source.as_ref()],
            None => self.sources()?.iter().collect(),
        };
        for src in sources {
            let network = match src {
                RegistrySource::Git(_) => true,
                RegistrySource::Monorepo(mono) => !mono.is_local(),
                RegistrySource::Local(_) => false,
            };
            if self.offline && network {
                return Err(RegistryError::OfflinePackageUnavailable {
                    group: group.clone(),
                    name: pkgref.name.to_string(),
                    req: match &pkgref.version {
                        vibe_core::VersionSpec::Latest => "latest".into(),
                        vibe_core::VersionSpec::Req(req) => req.to_string(),
                    },
                });
            }
            match src {
                RegistrySource::Git(reg) => {
                    match self.ask_remote(reg, group, pkgref.name.as_str(), task, || {
                        reg.resolve_with_route(pkgref)
                    }) {
                        Ok((resolved, selected_route)) => {
                            let stub_tag = format!("v{}", resolved.version);
                            // Step 2a: redirect probe (PROP-002 §2.4.2). The
                            // registry served a tag; check whether the repo
                            // at that tag is a stub pointing elsewhere. The
                            // probe is one extra `git archive` call, only
                            // when the registry-walk leg succeeded; cheap.
                            if let Some(redirect) = try_fetch_redirect(
                                reg,
                                &resolved,
                                &stub_tag,
                                selected_route.as_ref(),
                            )? {
                                return self
                                    .follow_redirect(pkgref, &resolved, reg, &redirect, &stub_tag);
                            }
                            let url = reg.package_repo_url(&resolved.group, &resolved.name)?;
                            return Ok(MultiResolution {
                                direct: None,
                                locked: None,
                                selected_route,
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
                            if registry_filter.is_none()
                                && let Some(resolution) =
                                    self.store_availability_fallback(pkgref, group)
                            {
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
                    }
                }
                // A local-directory registry: resolve straight off the
                // filesystem — no redirect probe, no per-package repo URL,
                // no auth. The source is the directory itself, recorded as
                // the lockfile `source_url` (a `file://` / path string) with
                // no `source_ref` (there is no git ref).
                RegistrySource::Local(ls) => {
                    task.detail(format!("checking local registry {}", ls.name));
                    match ls.registry.resolve(pkgref) {
                        Ok(resolved) => {
                            let source_url =
                                crate::local_registry::source_uri_for_local(&resolved.source_dir);
                            return Ok(MultiResolution {
                                direct: None,
                                locked: None,
                                selected_route: None,
                                resolved,
                                registry_name: Some(ls.name.clone()),
                                source_url,
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
                            if registry_filter.is_none()
                                && let Some(resolution) =
                                    self.store_availability_fallback(pkgref, group)
                            {
                                return Ok(resolution);
                            }
                            return Err(err);
                        }
                        Err(other) => return Err(other),
                    }
                }
                RegistrySource::Monorepo(mono) => {
                    task.detail(format!("checking monorepo registry {}", mono.name));
                    match mono.local().and_then(|local| local.resolve(pkgref)) {
                        Ok(resolved) => {
                            let source_url = crate::git_registry::monorepo_source_url(
                                &mono.url,
                                &resolved.group,
                                &resolved.name,
                                &resolved.version,
                            );
                            return Ok(MultiResolution {
                                direct: None,
                                locked: None,
                                selected_route: None,
                                resolved,
                                registry_name: Some(mono.name.clone()),
                                source_url,
                                source_ref: Some(mono.refname.clone()),
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
                                name: mono.name.clone(),
                                url: mono.url.clone(),
                                auth: mono.auth,
                                status: WalkAttemptStatus::NotFound,
                            });
                            continue;
                        }
                        Err(err @ RegistryError::NoMatchingVersion { .. }) => {
                            if registry_filter.is_none()
                                && let Some(resolution) =
                                    self.store_availability_fallback(pkgref, group)
                            {
                                return Ok(resolution);
                            }
                            return Err(err);
                        }
                        Err(RegistryError::Git(crate::git_backend::GitError::AuthFailed {
                            ..
                        })) if mono.auth == vibe_core::manifest::AuthKind::None
                            && !self.strict_auth =>
                        {
                            attempts.push(RegistryWalkAttempt {
                                name: mono.name.clone(),
                                url: mono.url.clone(),
                                auth: mono.auth,
                                status: WalkAttemptStatus::Public401,
                            });
                            continue;
                        }
                        Err(other) => return Err(other),
                    }
                }
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
        if registry_filter.is_none()
            && let Some(resolution) = self.store_availability_fallback(pkgref, group)
        {
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
}

#[cfg(test)]
#[path = "walk/tests.rs"]
mod tests;
