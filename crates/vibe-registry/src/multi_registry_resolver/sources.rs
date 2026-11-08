//! Path-source and git-source dispatch — the `[requires.packages]`
//! table-form declarations that bypass the registry walk: git-source
//! resolution / fetch (PROP-002 §2.4.1) and path-source resolution /
//! fetch (PROP-007 §2.5).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#git-source");

use super::*;

/// The clone URL of a git-source dependency after the Locator has had its
/// say (`PROP-066#locator`). The declared `url` stays in the manifest and
/// in the lock — nothing is rewritten there — so the canonical origin is
/// still visible for review and a lock stays portable between networks;
/// only the place git is pointed at moves.
///
/// The artifact carries the dependency's `group/name`, so a `package` row
/// redirects one dependency without touching the rest.
fn located_git_source(
    dep: &GitPackageDep,
) -> Result<crate::git_package_registry::PublicationRoute, RegistryError> {
    let artifact = vibe_core::remap::Artifact::new(vibe_core::remap::ArtifactKind::GitSourceDep)
        .with_url(&dep.url)
        .with_package(format!("{}/{}", dep.group, dep.name));
    vibe_core::remap::active()
        .choose_candidate(&artifact)
        .map(|chosen| crate::git_package_registry::PublicationRoute {
            url: chosen.target.display(),
            publication: chosen.publication_scope(),
        })
        .map_err(|refusal| RegistryError::MalformedMeta {
            path: PathBuf::from(dep.url.clone()),
            reason: refusal.to_string(),
        })
}

/// A declared git source with the default `auth = "none"` keeps its incumbent
/// meaning on an SSH address: the transport's own key agent / SSH config is the
/// credential, exactly as before credential ceilings were bound
/// (spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth).
/// The default is indistinguishable from omission, so it lowers the same way
/// an omitted declaration does; an active source policy still decides through
/// the backend's `preflight_registry_auth`. Explicit mechanisms are unchanged.
fn declared_git_source_auth(
    dep: &GitPackageDep,
    physical: &str,
) -> Result<vibe_core::manifest::AuthKind, RegistryError> {
    if dep.auth != vibe_core::manifest::AuthKind::None {
        return Ok(dep.auth);
    }
    let ssh = crate::registry_auth::ssh_source(physical).map_err(|reason| {
        crate::registry_auth::declaration_error(
            &format!("git source {}/{}", dep.group, dep.name),
            physical,
            reason,
        )
    })?;
    Ok(if ssh {
        vibe_core::manifest::AuthKind::Ssh
    } else {
        vibe_core::manifest::AuthKind::None
    })
}

impl MultiRegistryResolver {
    /// Resolve a `[requires.packages]` git-source declaration
    /// (PROP-002 §2.4.1). Synthesises a single-package
    /// `GitPerPackageRegistry` pointing at `dep.url`, fetches
    /// `vibe.toml` at the declared `tag`/`branch`/`rev`,
    /// verifies the `(group, name)` identity matches and the optional
    /// `version` constraint is satisfied, returns a `MultiResolution`
    /// with `is_git_source = true`.
    pub(super) fn resolve_git_source(
        &self,
        pkgref: &PackageRef,
        dep: &GitPackageDep,
    ) -> Result<MultiResolution, RegistryError> {
        let refname = dep.ref_kind.as_str().to_string();
        let clone_url = located_git_source(dep)?;
        let execution_reference = match &dep.ref_kind {
            vibe_core::manifest::GitRefKind::Branch(branch) => format!("refs/heads/{branch}"),
            vibe_core::manifest::GitRefKind::Tag(tag) => format!("refs/tags/{tag}"),
            vibe_core::manifest::GitRefKind::Rev(revision) => revision.clone(),
        };
        let direct = self.retain_direct_source(direct::DirectSourceInput {
            group: &dep.group,
            name: pkgref.name.as_str(),
            canonical: &dep.url,
            physical: &clone_url.url,
            publication: clone_url.publication,
            reference: &refname,
            execution_reference: &execution_reference,
            kind: direct::DirectKind::Git,
            auth: declared_git_source_auth(dep, &clone_url.url)?,
            token_env: dep.token_env.as_deref(),
        })?;
        let manifest = direct.manifest();
        let meta = manifest
            .require_package()
            .map_err(|e| RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{}@{}:{}", dep.url, refname, Manifest::FILENAME)),
                reason: e.to_string(),
            })?;
        // Sanity: the declaration says `(group, name)` but the repo's
        // manifest declares some other identity. Refuse to install —
        // pulling code under a misnamed slot would silently misroute
        // on disk and confuse downstream commands. `kind` is metadata
        // (PROP-008 §2.3) — not compared here.
        if meta.group != dep.group || meta.name != pkgref.name {
            return Err(RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{}@{}:{}", dep.url, refname, Manifest::FILENAME)),
                reason: format!(
                    "git-source `{}/{}` points at a manifest declaring `{}/{}` — refusing to install",
                    dep.group, pkgref.name, meta.group, meta.name
                ),
            });
        }
        // Verify the optional version constraint, if the operator declared one.
        if let Some(spec) = &dep.version
            && !spec.matches(&meta.version)
        {
            return Err(RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{}@{}:{}", dep.url, refname, Manifest::FILENAME)),
                reason: format!(
                    "git-source `{}/{}@{}` declares version `{}`, which does not satisfy the constraint `{}`",
                    dep.group, pkgref.name, refname, meta.version, spec
                ),
            });
        }
        let resolved = ResolvedPackage {
            group: dep.group.clone(),
            name: pkgref.name.to_string(),
            version: meta.version.clone(),
            source_dir: self.git_source_clone_dir(&dep.group, pkgref.name.as_str()),
        };
        let recorded_reference = direct.reference().to_owned();
        Ok(MultiResolution {
            direct: Some(direct),
            locked: None,
            selected_route: None,
            resolved,
            registry_name: None,
            source_url: dep.url.clone(),
            source_ref: Some(recorded_reference),
            overridden: false,
            is_git_source: true,
            is_path_source: false,
            via_redirect: None,
            from_store: false,
            redirect_target_auth: vibe_core::manifest::AuthKind::None,
            redirect_target_token_env: None,
        })
    }

    /// Resolve a `[requires.packages]` path-source declaration
    /// (PROP-007 §2.5). The package lives in a local directory
    /// (`dep.package_dir`, already canonicalised by the workspace
    /// layer); there is no registry walk and no git clone. Reads the
    /// package's `vibe.toml`, verifies `(kind, name)` matches and the
    /// optional `version` constraint is satisfied, returns a
    /// `MultiResolution` with `is_path_source = true` and the source
    /// recorded as the workspace-relative path (`dep.workspace_rel`).
    pub(super) fn resolve_path_source(
        &self,
        pkgref: &PackageRef,
        dep: &ResolvedPathDep,
    ) -> Result<MultiResolution, RegistryError> {
        let manifest_path = dep.package_dir.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        let meta = manifest
            .require_package()
            .map_err(|e| RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: e.to_string(),
            })?;
        // Sanity: the declaration says `(group, name)` but the package's
        // own manifest declares some other identity. Refuse to install —
        // pulling code under a misnamed slot would silently misroute
        // on disk and confuse downstream commands. `kind` is metadata
        // (PROP-008 §2.3) — not compared here.
        if meta.group != dep.group || meta.name != pkgref.name {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: format!(
                    "path-source `{}/{}` points at a manifest declaring `{}/{}` — refusing to install",
                    dep.group, pkgref.name, meta.group, meta.name
                ),
            });
        }
        // Verify the optional version constraint, if the path-dep
        // carried the dual-form `{ path, version }`. The resolved
        // version is the package's own `[package].version`.
        if let Some(spec) = &dep.version
            && !spec.matches(&meta.version)
        {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: format!(
                    "path-source `{}/{}` at `{}` declares version `{}`, which does not satisfy the constraint `{}`",
                    dep.group, pkgref.name, dep.workspace_rel, meta.version, spec
                ),
            });
        }
        let resolved = ResolvedPackage {
            group: dep.group.clone(),
            name: pkgref.name.to_string(),
            version: meta.version.clone(),
            source_dir: dep.package_dir.clone(),
        };
        Ok(MultiResolution {
            direct: None,
            locked: None,
            selected_route: None,
            resolved,
            registry_name: None,
            // `source_url` records the workspace-relative path, never an
            // absolute path and never a URL — PROP-007 §2.5.
            source_url: dep.workspace_rel.clone(),
            source_ref: None,
            overridden: false,
            is_git_source: false,
            is_path_source: true,
            via_redirect: None,
            from_store: false,
            redirect_target_auth: vibe_core::manifest::AuthKind::None,
            redirect_target_token_env: None,
        })
    }

    /// Where git-source clones live —
    /// `<cache_root>/__git_sources__/<group>.<name>/clone/`. Distinct
    /// from registry-served clones and from override clones so a
    /// package that flips between resolution modes does not share
    /// state across modes. Keyed by `(group, name)` identity (PROP-008).
    fn git_source_clone_dir(&self, group: &Group, name: &str) -> PathBuf {
        self.cache_root
            .join("__git_sources__")
            .join(format!("{group}.{name}"))
            .join("clone")
    }

    /// Fetch a git-source-resolved package into the machine-global
    /// store, write-once (PROP-010 §2.7). Same shape as
    /// `fetch_override` but threads `dep.auth` / `dep.token_env`
    /// through so private targets get token injection and the M1.14
    /// scrub-from-`.git/config` discipline applies.
    pub(super) fn fetch_git_source(
        &self,
        resolution: &MultiResolution,
        store_root: &Path,
        _expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        let group = &resolution.resolved.group;
        let name = resolution.resolved.name.as_str();
        let qualified = format!("{group}/{name}");
        let dep =
            self.git_packages
                .get(&qualified)
                .ok_or_else(|| RegistryError::UnknownPackage {
                    group: group.clone(),
                    name: name.to_string(),
                })?;
        let refname = resolution
            .source_ref
            .clone()
            .unwrap_or_else(|| dep.ref_kind.as_str().to_string());

        // Synthesise a single-package registry just to leverage its
        // `package_repo_url` / `credentialed_url` plumbing for token
        // injection + scrub. The synthetic registry's clone path is
        // not used here — we clone into our own `__git_sources__`
        // sub-tree so the cache stays organised by resolution mode.
        let legacy_registry;
        let reg = if let Some(lease) = &resolution.direct {
            lease.registry()
        } else {
            let clone_url = located_git_source(dep)?;
            legacy_registry = GitPerPackageRegistry::open_single_package(
                &format!("git-source-{group}-{name}"),
                &clone_url.url,
                &refname,
                &self.cache_root,
                self.backend.clone(),
                DEFAULT_FRESHNESS_SECS,
                dep.auth,
                dep.token_env.as_deref(),
            )?;
            &legacy_registry
        };
        let plain_url = reg.package_repo_url(group, name)?;
        let credentialed = reg.credentialed_url(&plain_url);

        let clone_dir = self.git_source_clone_dir(group, name);
        if resolution.direct.is_some() {
            reg.ensure_selected_clone(reg.org_ref(), &clone_dir)?;
        } else {
            ensure_clone_at(self.backend.as_ref(), &credentialed, &refname, &clone_dir)?;
        }
        if let Some(lease) = &resolution.direct
            && Manifest::read(clone_dir.join(Manifest::FILENAME))? != *lease.manifest()
        {
            return Err(RegistryError::MalformedMeta {
                path: clone_dir.join(Manifest::FILENAME),
                reason: "direct source manifest changed after selection".into(),
            });
        }
        // Token-discipline (M1.14): scrub any credentialed URL from
        // the freshly-bootstrapped `.git/config` so the token does not
        // persist on disk. Best-effort — if the backend has no
        // `set_remote_url` impl, the default is a no-op (the
        // credentialed URL was only ever in-memory anyway for
        // backends that don't write a `.git/config`).
        if credentialed != plain_url {
            self.backend
                .set_remote_url(&clone_dir, "origin", &plain_url)
                .ok();
        }

        // Hash the clone directly (the recipe skips `.git`), then
        // insert write-once into the machine store; an already-present
        // entry is returned untouched, its manifest read back.
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
            source_uri: plain_url,
            // Nothing redirected this fetch (`PROP-066#provenance`).
            fetched_from: None,
            registry_name: None,
            source_ref: Some(refname),
            resolved_commit,
            overridden: false,
            is_git_source: true,
            is_path_source: false,
            is_embedded: false,
            is_local: false,
            via_redirect: None,
        })
    }

    /// Fetch a path-source-resolved package (PROP-007 §2.5) — and the
    /// one deliberate NON-store fetch: a path source is a live
    /// workspace member, mutable under the same version, so an
    /// identity-keyed write-once entry would freeze stale bytes the
    /// first time it was fetched and serve them forever. The member's
    /// own directory IS the content source: `cache_dir` points at it,
    /// the hash is computed over it (the recipe skips any `.git/`),
    /// and `vibedeps/` materialisation — which always re-materialises
    /// a `source_mutable` package — strips `.git/` as it copies.
    pub(super) fn fetch_path_source(
        &self,
        resolution: &MultiResolution,
        _store_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        let group = &resolution.resolved.group;
        let name = resolution.resolved.name.as_str();
        // The resolver stored the canonicalised package directory on
        // `resolved.source_dir`; `workspace_rel` is in `source_url`.
        let package_dir = resolution.resolved.source_dir.clone();
        let workspace_rel = resolution.source_url.clone();

        let manifest_path = package_dir.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: "registry package manifest must carry a [package] table".to_string(),
            });
        }
        let content_hash = compute_content_hash(&package_dir)?;

        Ok(CachedPackage {
            locked_selection: None,
            route_publication: vibe_core::RoutePublication::Local,
            recorded_source_kind: None,
            store_reused: Some(false),
            acquisition: Some(vibe_wire::generated::shared::SourceAcquisition::Directory),
            resolved: ResolvedPackage {
                group: group.clone(),
                name: name.to_string(),
                version: resolution.resolved.version.clone(),
                source_dir: package_dir.clone(),
            },
            cache_dir: package_dir,
            manifest,
            content_hash,
            // `source_uri` records the workspace-relative path — the
            // lockfile `source_url` for a path entry. Never a URL,
            // never absolute.
            source_uri: workspace_rel,
            // Nothing redirected this fetch (`PROP-066#provenance`).
            fetched_from: None,
            registry_name: None,
            source_ref: None,
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: true,
            is_embedded: false,
            is_local: false,
            via_redirect: None,
        })
    }
}

#[cfg(test)]
#[path = "sources/tests.rs"]
mod tests;
