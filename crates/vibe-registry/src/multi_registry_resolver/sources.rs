//! Path-source and git-source dispatch — the `[requires.packages]`
//! table-form declarations that bypass the registry walk: git-source
//! resolution / fetch (PROP-002 §2.4.1) and path-source resolution /
//! fetch (PROP-007 §2.5).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#git-source");

use super::*;

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
        let synthetic_name = format!("git-source-{}-{}", dep.group, dep.name);
        let refname = dep.ref_kind.as_str().to_string();
        let reg = GitPerPackageRegistry::open_single_package(
            &synthetic_name,
            &dep.url,
            &refname,
            &self.cache_root,
            Arc::clone(&self.backend),
            DEFAULT_FRESHNESS_SECS,
            dep.auth,
            dep.token_env.as_deref(),
        )?;
        let manifest = reg.fetch_manifest_at_ref(&dep.group, &dep.name, &refname)?;
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
        Ok(MultiResolution {
            resolved,
            registry_name: None,
            source_url: dep.url.clone(),
            source_ref: Some(refname),
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
        let synthetic_name = format!("git-source-{group}-{name}");
        let reg = GitPerPackageRegistry::open_single_package(
            &synthetic_name,
            &dep.url,
            &refname,
            &self.cache_root,
            Arc::clone(&self.backend),
            DEFAULT_FRESHNESS_SECS,
            dep.auth,
            dep.token_env.as_deref(),
        )?;
        let plain_url = reg.package_repo_url(group, name)?;
        let credentialed = reg.credentialed_url(&plain_url);

        let clone_dir = self.git_source_clone_dir(group, name);
        ensure_clone_at(self.backend.as_ref(), &credentialed, &refname, &clone_dir)?;
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
        let dest = store::insert_current_at(
            store_root,
            &clone_dir,
            group,
            name,
            &resolution.resolved.version,
            &content_hash,
        )?
        .into_entry();
        let manifest_path = dest.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: "registry package manifest must carry a [package] table".to_string(),
            });
        }

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
            source_uri: plain_url,
            registry_name: None,
            source_ref: Some(refname),
            resolved_commit: None,
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
