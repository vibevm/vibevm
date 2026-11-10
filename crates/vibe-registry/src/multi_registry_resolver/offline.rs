//! Store-backed resolution (PROP-010 §2.6) — the offline posture and
//! the availability fallback. Split into its own file because it is
//! one seam: every path here reads the machine store (plus the local
//! `file://` sources) and NEVER issues `git fetch` / `ls-remote` /
//! archive — under the offline posture resolution is computed against
//! the store **as of its last refresh**, and a store hit is
//! **authoritative for availability**: bytes we already fetched and
//! already verified against a lockfile pin outrank a registry that no
//! longer lists the version (`A-CACHE-HIT-IS-AUTHORITATIVE-FOR-
//! AVAILABILITY`).
//!
//! Provenance without minting: a store-backed resolution takes
//! `source_uri` from the existing lockfile entry handed in via
//! [`MultiRegistryResolver::with_locked_packages`] — the availability
//! case is a re-resolve of an earlier install. A package in no
//! registry AND in no lock entry is not rescued by the store; a
//! `store://` wire value would be a new identity mint, which is an
//! owner act, not a resolver's.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

impl MultiRegistryResolver {
    /// Resolve under the offline posture (PROP-010 §2.6,
    /// `RESOLVER-OFFLINE-MODE`): the local `file://` sources first —
    /// they never touch the network and keep today's semantics — then
    /// the machine store. No git registry is entered, so no
    /// `git fetch` / `ls-remote` / archive call can happen. A miss is
    /// the hard offline error naming the package and the recovery
    /// recipes (PROP-010 §2.5, `OFFLINE-HARD-ERROR`).
    pub(super) fn resolve_offline(
        &self,
        pkgref: &PackageRef,
        group: &Group,
    ) -> Result<MultiResolution, RegistryError> {
        for src in self.sources()? {
            if let RegistrySource::Monorepo(mono) = src {
                // A monorepo on this machine (or a share) needs no
                // network: read it like a `localdir` registry.
                let Some(local) = mono.offline_local()? else {
                    continue;
                };
                match local.resolve(pkgref) {
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
                    Err(RegistryError::UnknownPackage { .. })
                    | Err(RegistryError::NoMatchingVersion { .. }) => continue,
                    Err(other) => return Err(other),
                }
            }
            let RegistrySource::Local(ls) = src else {
                // A git registry is a network source — under the
                // offline posture it is not entered at all.
                continue;
            };
            match ls.registry.resolve(pkgref) {
                Ok(resolved) => {
                    // Same exact package-directory identity as the online
                    // local walk and the local acquisition owner, so an
                    // offline plan names the source the install records.
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
                // "Not here" from a local source walks on — the store
                // may still hold the package. Anything else (a
                // malformed tree, an I/O failure) halts, exactly as it
                // does on the online walk.
                Err(RegistryError::UnknownPackage { .. })
                | Err(RegistryError::NoMatchingVersion { .. }) => continue,
                Err(other) => return Err(other),
            }
        }
        if let Some(resolution) = self.store_hit(pkgref, group) {
            return Ok(resolution);
        }
        Err(RegistryError::OfflinePackageUnavailable {
            group: group.clone(),
            name: pkgref.name.to_string(),
            req: req_label(&pkgref.version),
        })
    }

    /// The availability fallback (PROP-010 §2.6,
    /// `A-CACHE-HIT-IS-AUTHORITATIVE-FOR-AVAILABILITY`), consulted by
    /// the online walk ONLY where a registry answered "no such
    /// package / no such version". Operational failures (network,
    /// auth, malformed metadata) never reach this function, so they
    /// can never be masked by the store.
    pub(super) fn store_availability_fallback(
        &self,
        pkgref: &PackageRef,
        group: &Group,
    ) -> Option<MultiResolution> {
        self.store_hit(pkgref, group)
    }

    /// A store hit for `(group, name)`: the lockfile entry's pinned
    /// version, WHEN it satisfies the requested constraint and WHEN
    /// the entry is present in the store. Provenance (`source_url`,
    /// `registry_name`, `source_ref`) is taken verbatim from the lock
    /// entry — never minted here.
    fn store_hit(&self, pkgref: &PackageRef, group: &Group) -> Option<MultiResolution> {
        let root = self.store_root.as_deref()?;
        let locked = self.locked.get(&pkgref.qualified_name())?;
        if !version_matches(&pkgref.version, &locked.version) {
            return None;
        }
        let entry = store::lookup_at(root, group, locked.name.as_str(), &locked.version)?;
        Some(MultiResolution {
            direct: None,
            locked: None,
            selected_route: None,
            resolved: ResolvedPackage {
                group: locked.group.clone(),
                name: locked.name.as_str().to_string(),
                version: locked.version.clone(),
                // The store entry is the content source from here on —
                // fetch short-circuits to serve its bytes (PROP-010
                // §2.7: the layout is the index).
                source_dir: entry,
            },
            registry_name: locked.registry.clone(),
            source_url: locked.source_url.as_str().to_string(),
            source_ref: locked.source_ref.clone(),
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            via_redirect: None,
            from_store: true,
            redirect_target_auth: vibe_core::manifest::AuthKind::None,
            redirect_target_token_env: None,
        })
    }

    /// Serve a store-backed resolution: the entry IS the content
    /// source — manifest off disk, content hash over the entry, no
    /// source walk, no network. A pin, when the caller has one, is
    /// verified with the SAME entry gate the fetch path applies (the
    /// `AlreadyPresent` branch of `fetch_with_expected_hash`), so an
    /// entry tampered with outside vibevm is named, never silently
    /// used (`A-MISMATCH-IS-NAMED-NEVER-SWALLOWED`).
    pub(super) fn fetch_from_store(
        &self,
        resolution: &MultiResolution,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        let entry = resolution.resolved.source_dir.clone();
        let manifest_path = entry.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path,
                reason: "store entry manifest must carry a [package] table".to_string(),
            });
        }
        let content_hash = compute_content_hash(&entry)?;
        if let Some(expected) = expected_hash {
            crate::git_package_registry::verify_store_entry_against_pin(
                &entry,
                expected,
                &resolution.resolved.group,
                &resolution.resolved.name,
                &resolution.resolved.version,
            )?;
        }
        Ok(CachedPackage {
            locked_selection: None,
            route_publication: vibe_core::RoutePublication::Local,
            recorded_source_kind: self
                .locked
                .get(&format!(
                    "{}/{}",
                    resolution.resolved.group, resolution.resolved.name
                ))
                .and_then(|p| p.source_kind),
            store_reused: Some(true),
            acquisition: Some(vibe_wire::generated::shared::SourceAcquisition::Store),
            resolved: resolution.resolved.clone(),
            // The entry is what `vibedeps/` materialisation copies
            // from — same contract as a registry fetch's store insert.
            cache_dir: entry,
            manifest,
            content_hash,
            source_uri: resolution.source_url.clone(),
            // Nothing redirected this fetch (`PROP-066#provenance`).
            fetched_from: None,
            registry_name: resolution.registry_name.clone(),
            source_ref: resolution.source_ref.clone(),
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_embedded: false,
            is_local: false,
            via_redirect: None,
            template: None,
            via_channel: None,
        })
    }
}

/// Does the pinned lockfile version satisfy the requested constraint?
/// `Latest` accepts the pin (offline resolution is computed against
/// the store as of its last refresh — PROP-010 §2.6); a requirement
/// must match it outright.
pub(super) fn version_matches(req: &VersionSpec, pinned: &semver::Version) -> bool {
    match req {
        VersionSpec::Latest => true,
        VersionSpec::Req(r) => r.matches(pinned),
        // Fail closed (PROP-075 `#CHANNEL-SYNTAX`).
        VersionSpec::Channel(_) => false,
    }
}

/// Human label for a requested constraint — `VersionSpec`'s `Display`
/// renders `Latest` as the empty string, which would leave the
/// offline-miss message with a dangling `@`.
fn req_label(spec: &VersionSpec) -> String {
    spec.requirement_label()
}
