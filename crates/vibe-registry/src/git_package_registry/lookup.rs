//! Clone-free lookups for the per-package registry — tag-based version
//! listing, manifest reads that try HTTPS first and archive next before
//! falling back to a clone, and version resolution against the upstream
//! tag set (PROP-002 §2.3 / §2.5). The clone / cache-materialisation
//! half lives in [`super::fetch`].

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;
use crate::git_failure::GitOperation;
use crate::index_client::IndexVersion;

impl GitPerPackageRegistry {
    /// Fetch `vibe.toml` at an arbitrary git ref (tag, branch,
    /// or commit SHA) — used by the git-source resolver path
    /// (PROP-002 §2.4.1) where the operator declared `tag = "..."` /
    /// `branch = "..."` / `rev = "..."` and we cannot enumerate
    /// versions through `list_versions` (which is tag-shaped).
    ///
    /// Same read ladder as [`Self::fetch_dep_manifest`] — HTTPS, then
    /// archive, then clone — and the same auth / token-injection
    /// discipline: token from env at construction, credentialed URL only
    /// for the spawned `git archive` (or, on the HTTPS step, for that
    /// request's `Authorization` header), plain URL recorded in error
    /// messages.
    ///
    /// `refname` is any git ref, not only a version tag: the git-source
    /// path passes whatever the operator declared, so a branch name or a
    /// commit SHA arrives here too. Every step of the ladder takes it as
    /// given.
    pub fn fetch_manifest_at_ref(
        &self,
        group: &Group,
        name: &str,
        refname: &str,
    ) -> Result<Manifest, RegistryError> {
        self.ensure_token_loaded()?;
        let plain_url = self.package_repo_url(group, name)?;
        let fetch_url = self.selected_credentialed_url(&plain_url)?;
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
            Err(other) => {
                return Err(self.git_failure(
                    GitOperation::read_file(group, name, refname, Manifest::FILENAME),
                    &plain_url,
                    other,
                ));
            }
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
    /// error is returned, except that any typed contact refusal is terminal.
    /// For ordinary errors the primary is the
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
        self.try_lookup_with_route(group, name, f)
            .map(|(value, _)| value)
    }

    fn try_lookup_with_route<T, F>(
        &self,
        group: &Group,
        name: &str,
        f: F,
    ) -> Result<(T, PublicationRoute), RegistryError>
    where
        F: Fn(&str) -> Result<T, RegistryError>,
    {
        let (primary, mirrors) = self.package_candidates(group, name)?;
        let mut first_error = None;
        let mut excluded = None;
        for url in std::iter::once(primary).chain(mirrors) {
            match f(&url.url) {
                Ok(value) => return Ok((value, url)),
                Err(error)
                    if self.backend.uses_native_materialisation() && excluded_route(&error) =>
                {
                    tracing::debug!(registry=%self.name,"concrete registry candidate excluded; trying independently admitted alternative");
                    excluded.get_or_insert(error);
                }
                Err(error) if super::route::terminal_lookup_error(&error) => return Err(error),
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        Err(first_error
            .or(excluded)
            .unwrap_or_else(|| RegistryError::UnknownPackage {
                group: group.clone(),
                name: name.into(),
            }))
    }

    pub(crate) fn read_selected_file(
        &self,
        resolved: &ResolvedPackage,
        reference: &str,
        path: &str,
        route: Option<&RegistryRoute>,
    ) -> Result<Vec<u8>, RegistryError> {
        self.ensure_token_loaded()?;
        let read = |url: &str| {
            let plain = strip_git_plus_prefix(url);
            self.backend
                .fetch_file_at_ref(&self.selected_credentialed_url(plain)?, reference, path)
                .map_err(|error| {
                    self.git_failure(
                        GitOperation::read_file(&resolved.group, &resolved.name, reference, path),
                        plain,
                        error,
                    )
                })
        };
        match route {
            Some(route) => read(route.validate(self, resolved)?),
            None => self.try_lookup(&resolved.group, &resolved.name, read),
        }
    }
    pub(crate) fn read_package_file(
        &self,
        group: &Group,
        name: &str,
        reference: &str,
        path: &str,
    ) -> Result<Vec<u8>, RegistryError> {
        self.ensure_token_loaded()?;
        self.try_lookup(group, name, |url| {
            let plain = strip_git_plus_prefix(url);
            self.backend
                .fetch_file_at_ref(&self.selected_credentialed_url(plain)?, reference, path)
                .map_err(|error| {
                    self.git_failure(
                        GitOperation::read_file(group, name, reference, path),
                        plain,
                        error,
                    )
                })
        })
    }

    /// Enumerate available versions for `<group>/<name>` *without
    /// cloning*. Tags that don't match `v<semver>` are silently
    /// dropped.
    ///
    /// Enumeration, not selection: the list carries every version the
    /// sources name, capability-blind by contract. The selector
    /// ([`Self::resolve`]) walks the same candidates with their
    /// `must_understand` declarations in hand instead (B-080).
    pub fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        let produced = self
            .produced_candidates(group, name)?
            .0
            .into_iter()
            .map(|c| c.version)
            .collect();
        // **Call site 1 of PROP-075 `#TEMPLATE-DOMAIN-SEAM` on this
        // registry shape** (`#TEMPLATE-IN-A-REPO`): the concrete
        // publications, unioned with the versions the template's source
        // lists. With no template this is `produced` unchanged, which is
        // every package in a registry.
        self.union_with_template(group, name, produced)
    }

    /// The candidates a listing or a selection may see: every row the
    /// index or the tag list named, **minus the template revisions**
    /// (PROP-075 `#TEMPLATE-REVISION-NOT-A-VERSION`).
    ///
    /// The split is here, in one place above both the listing and the
    /// selector, so the two cannot disagree — and it is the same split
    /// `discover_in_directory` makes from the manifests on disk. The
    /// rows it drops are not lost: [`Self::active_template`] reads the
    /// unsplit set, because a revision is exactly what discovery is
    /// looking for.
    fn produced_candidates(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<(Vec<IndexVersion>, Option<PublicationRoute>), RegistryError> {
        let (entries, route) = self.list_candidates_with_route(group, name)?;
        let produced = entries
            .into_iter()
            .filter(|entry| !entry.template)
            .collect();
        Ok((produced, route))
    }

    /// The candidate set [`Self::resolve`] picks over: every version
    /// the index fast path or the git tag path can name, each carrying
    /// the `must_understand` declarations its source stated (PROP-044
    /// §4.5). The index path reads them off the `by-name` record; the
    /// git tag path has no record to read at selection time, so its
    /// candidates declare nothing — a picked version's manifest is
    /// read (and its declarations honoured) only after the pick.
    ///
    /// Mirror-aware on the git half: tries the primary URL first,
    /// then each mirror in priority order. The first URL that yields
    /// a tag list wins. If every URL says `RepoNotFound`, the result
    /// is `UnknownPackage` (treated identically to the primary-only
    /// path).
    pub(super) fn list_candidates_with_route(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<(Vec<IndexVersion>, Option<PublicationRoute>), RegistryError> {
        // Index fast path (PROP-005 §2.10 slice 10). When the
        // registry has an upstream index attached, query it first.
        // 200 → return versions; 404 → fall through to git path
        // (UnknownPackage from the index does not authoritatively
        // mean "absent" — the index may be stale); other errors →
        // also fall through with a debug-level log.
        if let Some(client) = self.discover_index()? {
            match client.list_version_entries(group, name) {
                Ok(Some(entries)) => {
                    tracing::debug!(
                        target: "vibe_registry::index",
                        registry = %self.name,
                        group = %group,
                        name = %name,
                        count = entries.len(),
                        revisions = entries.iter().filter(|e| e.template).count(),
                        "list_versions served from index"
                    );
                    return Ok((entries, None));
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
                Err(crate::IndexError::ContactRefused(crate::ContactRefusal::ExcludedHost {
                    ..
                })) if self.backend.uses_native_materialisation() => {
                    tracing::debug!(registry=%self.name,"index route excluded; checking independently admitted Git candidates");
                }
                Err(crate::IndexError::ContactRefused(refusal)) => return Err(refusal.into()),
                Err(e) => {
                    tracing::debug!(
                        target: "vibe_registry::index",
                        registry = %self.name,
                        group = %group,
                        name = %name,
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
        self.try_lookup_with_route(group, name, move |url| {
            let plain = strip_git_plus_prefix(url);
            let fetch_url = self.selected_credentialed_url(plain)?;
            let tags = backend.list_tags(&fetch_url).map_err(|e| match e {
                GitError::RepoNotFound { .. } => RegistryError::UnknownPackage {
                    group: owned_group.clone(),
                    name: owned_name.clone(),
                },
                other => self.git_failure(
                    GitOperation::list_versions(&owned_group, &owned_name),
                    plain,
                    other,
                ),
            })?;
            let mut versions: Vec<semver::Version> = tags
                .iter()
                .filter_map(|t| {
                    let stripped = t.strip_prefix('v')?;
                    semver::Version::parse(stripped).ok()
                })
                .collect();
            versions.sort();
            Ok(versions
                .into_iter()
                .map(|version| IndexVersion {
                    version,
                    must_understand: Vec::new(),
                    // A tag list carries no manifest, so this path
                    // cannot tell a template revision from a produced
                    // version and does not pretend to: it declares
                    // nothing, exactly as it declares no
                    // `must_understand` (PROP-075
                    // `#TEMPLATE-REVISION-NOT-A-VERSION`).
                    template: false,
                })
                .collect())
        })
        .map(|(entries, route)| (entries, Some(route)))
    }

    /// Pick the best tag matching `pkgref.version` from the upstream tag list.
    /// Returns a [`ResolvedPackage`] whose `source_dir` points at the
    /// (not-yet-populated) clone directory under the cache bucket.
    ///
    /// The registry resolves by `(group, name)` identity (PROP-008); a
    /// pkgref reaching this point without a `group` is an
    /// [`RegistryError::UnqualifiedPkgref`] — short names must be
    /// qualified at the CLI boundary first.
    ///
    /// The pick quarantines before it selects (PROP-044 §4.5): a
    /// candidate whose `must_understand` names a reader capability
    /// this build lacks is skipped with a warn and never picked, and a
    /// candidate set with nothing left to pick refuses honestly (see
    /// [`pick_version`]) instead of landing on a version this reader
    /// cannot act on (B-080).
    pub fn resolve(&self, pkgref: &PackageRef) -> Result<ResolvedPackage, RegistryError> {
        self.resolve_with_route(pkgref)
            .map(|(resolved, _)| resolved)
    }

    pub(crate) fn resolve_with_route(
        &self,
        pkgref: &PackageRef,
    ) -> Result<(ResolvedPackage, Option<RegistryRoute>), RegistryError> {
        let group = pkgref
            .group
            .as_ref()
            .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;
        let name = pkgref.name.as_str();
        let (candidates, route) = self.produced_candidates(group, name)?;
        // **A channel lowers to an exact pin before anything selects**,
        // the same order `LocalRegistry::resolve` takes it in: lowering
        // at the one seam every entry path passes through is strictly
        // stronger than lowering per entry path, and an unlowered
        // channel still matches nothing (PROP-075 `#CHANNEL-SYNTAX`).
        let lowered = self.lower_through_template(pkgref)?;
        let pkgref = lowered.as_ref().unwrap_or(pkgref);
        // The pick is over the produced candidates unioned with what the
        // template produces, so a version only the template names is
        // selectable and a revision is not.
        let version = self.pick_with_template(group, name, &candidates, &pkgref.version)?;
        // **Call site 3**: a version the template produces and the
        // registry does not publish is rendered and materialised as an
        // ordinary package directory, and every caller above this gets
        // the `ResolvedPackage` it already expects.
        if let Some(resolved) = self.materialise_from_template(group, name, &version)? {
            return Ok((resolved, None));
        }
        let resolved = ResolvedPackage {
            group: group.clone(),
            name: pkgref.name.to_string(),
            version,
            source_dir: self.package_clone_dir(group, name),
        };
        let route = route
            .map(|url| RegistryRoute::selected(self, &resolved, url))
            .transpose()?;
        Ok((resolved, route))
    }

    /// Read a candidate version's `vibe.toml` *without cloning*. The
    /// depsolver calls this during the resolve walk to read declared
    /// `[requires]` of a candidate before committing to install. A walk
    /// over N candidates of one package costs N single-file reads, not N
    /// clones.
    ///
    /// Three ways to read one file, in order of what they cost:
    ///
    /// 1. **One HTTPS GET**, on the hosts that serve a single file that
    ///    way — GitHub and GitVerse (the `raw_http` module inside
    ///    [`git_backend::shell`](crate::git_backend::shell)). No
    ///    subprocess, no working tree, one round-trip. This is the
    ///    step that makes the "not N clones" promise true on GitHub,
    ///    where `git archive` is refused and step 2 could never keep it.
    /// 2. **`git archive --remote`**, for every other host that allows
    ///    `upload-archive` — also no working tree, one subprocess.
    /// 3. **A shallow clone**, when the host refuses the archive and the
    ///    read is not one step 1 could serve. The clone lands in the
    ///    per-package cache directory the install would use anyway, so
    ///    it doubles as pre-warming.
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
        let resolved = ResolvedPackage {
            group: group.clone(),
            name: name.into(),
            version: version.clone(),
            source_dir: self.package_clone_dir(group, name),
        };
        self.fetch_dep_manifest_selected(&resolved, None)
    }

    pub(crate) fn fetch_dep_manifest_selected(
        &self,
        resolved: &ResolvedPackage,
        route: Option<&RegistryRoute>,
    ) -> Result<Manifest, RegistryError> {
        let group = &resolved.group;
        let name = resolved.name.as_str();
        let version = &resolved.version;
        // **Call site 2 of PROP-075 `#TEMPLATE-DOMAIN-SEAM` on this
        // registry shape** (`#TEMPLATE-IN-A-REPO`): a version the
        // template produces and the registry does not publish has no tag
        // to read a manifest at, and the skeleton render is what the
        // resolve walk needs — no digest document, no request.
        if let Some(manifest) = self.template_skeleton(group, name, version)? {
            return Ok(manifest);
        }
        self.ensure_token_loaded()?;
        let tag = format!("v{version}");
        let archive_result = self.read_selected_file(resolved, &tag, Manifest::FILENAME, route);
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

#[cfg(test)]
mod contact_refusal_tests {
    use super::*;

    #[test]
    #[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
    fn lookup_never_discards_a_primary_or_mirror_contact_refusal() {
        let section: vibe_core::manifest::RegistrySection =
            toml::from_str("name='test'\nurl='https://github.com/o'").unwrap();
        let cache = tempfile::tempdir().unwrap();
        let reg = GitPerPackageRegistry::open_with_auth(
            &section.name,
            &section.url,
            &section.r#ref,
            section.naming,
            vec!["https://gitverse.ru/o".into()],
            cache.path(),
            Arc::new(crate::ShellGit::new()),
            0,
            crate::resolve_registry_auth(crate::RegistryAuthInput::SelectedRegistry(&section))
                .unwrap()
                .kind,
            None,
        )
        .unwrap();
        let group = Group::parse("org.example").unwrap();
        for denied_attempt in [0, 1] {
            let count = std::cell::Cell::new(0);
            let result: Result<(), RegistryError> = reg.try_lookup(&group, "p", |_| {
                let attempt = count.get();
                count.set(attempt + 1);
                if attempt == denied_attempt {
                    Err(crate::ContactRefusal::NativeTransportUnavailable.into())
                } else {
                    Err(RegistryError::UnknownPackage {
                        group: group.clone(),
                        name: "p".into(),
                    })
                }
            });
            assert!(matches!(
                result,
                Err(RegistryError::ContactRefused(
                    crate::ContactRefusal::NativeTransportUnavailable
                ))
            ));
            assert_eq!(count.get(), denied_attempt + 1);
        }
    }
}
