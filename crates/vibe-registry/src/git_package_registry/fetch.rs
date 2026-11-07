//! Clone / update orchestration for the per-package registry —
//! mirror-aware fetch dispatch with the cross-source content-hash
//! gate, then write-once insertion of the accepted payload into the
//! machine-global store `~/.vibe/cache/` (PROP-002 §2.3 / §2.6;
//! PROP-010 §2.7). The clone-free lookup half (version listing,
//! archive-first manifest reads) lives in [`super::lookup`].

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;
use crate::store::{self, InsertOutcome};

mod api;
mod identity;
use identity::commit_content_hash;
pub(crate) use identity::verify_store_entry_against_pin;
mod bring;
use bring::BringIntent;
mod prepared;
pub use prepared::{PackageAcquisition, PreparedInPlace};

impl GitPerPackageRegistry {
    /// Bring the per-package clone at `package_clone_dir(kind, name)`
    /// to `refname` by trying the primary URL first, then each mirror
    /// URL in priority order. Returns the URL that ultimately served
    /// the clone (canonical or a mirror) so the caller can record /
    /// log it.
    ///
    /// Mirror dispatch on this path is the cache-mutating sibling of
    /// [`Self::try_lookup`]: same primary-first ordering, same
    /// "primary's error is the most informative" semantics on full
    /// failure. The first attempt is a refresh of whatever copy is
    /// already there; every later attempt is a source switch that
    /// clones beside and swaps in only on success — a flapping primary
    /// can never cost the copy already on disk (PROP-010 §2.6).
    ///
    /// **Mirror integrity** is **not** checked here: the content from
    /// whichever URL succeeds is taken verbatim. The caller (typically
    /// [`Self::fetch_with_expected_hash`]) layers a content_hash
    /// gate on top when a lockfile pin is available.
    fn ensure_clone_against_sources(
        &self,
        group: &Group,
        name: &str,
        refname: &str,
    ) -> Result<String, RegistryError> {
        let clone_dir = self.package_clone_dir(group, name);
        self.bootstrap_chain_into(group, name, refname, &clone_dir)
    }

    /// Bring `dest` to `refname` against the package's source chain —
    /// the primary URL first as a refresh of any existing copy, then
    /// each mirror as a source switch (clone-beside-and-swap) —
    /// returning the URL that served it. The mirror-aware, auth-aware
    /// core shared by the cache clone ([`ensure_clone_against_sources`])
    /// and the direct in-place slot placement ([`materialise_in_place`]);
    /// only the destination differs.
    fn bootstrap_chain_into(
        &self,
        group: &Group,
        name: &str,
        refname: &str,
        dest: &Path,
    ) -> Result<String, RegistryError> {
        let (primary, mirrors) = self.package_urls(group, name)?;
        // Primary outside the mirror loop — its error is a plain value.
        // The primary attempt refreshes the existing copy in place; a
        // failure surfaces without touching the copy.
        let primary_err =
            match self.bring_clone_to_ref(&primary, refname, dest, BringIntent::RefreshExisting) {
                Ok(()) => return Ok(primary),
                Err(e) => e,
            };
        for (i, url) in mirrors.iter().enumerate() {
            // A mirror serves a DIFFERENT source than the copy on disk
            // — the switch clones beside and swaps in only on success.
            match self.bring_clone_to_ref(url, refname, dest, BringIntent::SwitchSource) {
                Ok(()) => {
                    tracing::info!(
                        target: "vibe_registry",
                        registry = %self.name,
                        primary = %primary,
                        served_by = %url,
                        mirror_index = i,
                        "fetch served by mirror"
                    );
                    return Ok(url.clone());
                }
                Err(e) => {
                    tracing::debug!(
                        target: "vibe_registry",
                        registry = %self.name,
                        mirror = %url,
                        error = %e,
                        "mirror switch failed; trying next"
                    );
                }
            }
        }
        Err(primary_err)
    }

    /// Place an `in-place` package directly into its project slot (PROP-022
    /// §2.4): a fresh `git clone --recurse-submodules` when `slot` is absent,
    /// an incremental `git fetch` + checkout when it already carries `.git` —
    /// so a version bump on a giant repo transfers only changed objects rather
    /// than re-downloading the whole tree (the deferred incremental update the
    /// move-based path could not do). Bypasses the `.git`-stripped cache copy
    /// and the machine store entirely. Auth / mirror handling is the
    /// same [`Self::bring_clone_to_ref`] chain the cache path uses — the
    /// token is injected and stripped identically, a present slot is
    /// refreshed in place, a mirror is a source switch; only the
    /// working-tree destination changes. Returns the canonical source URL,
    /// the version tag, the resolved commit (the in-place identity, §2.5),
    /// and the slot's manifest.
    pub fn materialise_in_place(
        &self,
        resolved: &ResolvedPackage,
        slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        self.ensure_token_loaded()?;
        let canonical_url = self.package_repo_url(&resolved.group, &resolved.name)?;
        let tag = self.package_reference(resolved);
        let existed = slot.join(".git").exists();
        let before_head = if existed {
            self.backend.head_commit(slot)?
        } else {
            None
        };
        let dirty = existed && self.backend.working_tree_dirty(slot)?;
        self.bootstrap_chain_into(&resolved.group, &resolved.name, &tag, slot)?;
        self.backend.clean_worktree(slot)?;
        let manifest_path = slot.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path,
                reason: "in-place package manifest must carry a [package] table".to_string(),
            });
        }
        let resolved_commit = self.backend.head_commit(slot)?;
        let content_hash = commit_content_hash(resolved_commit.as_deref().unwrap_or_default());
        let changed = !existed || dirty || before_head != resolved_commit;
        Ok(InPlaceMaterialised {
            source_uri: canonical_url,
            source_ref: tag,
            resolved_commit,
            content_hash,
            changed,
            manifest,
        })
    }

    /// Refresh the per-package clone for `(group, name)` against `refname`
    /// without touching the per-project cache. If the clone exists, runs
    /// `update`; otherwise bootstraps a fresh clone. Mirror-aware:
    /// the primary URL is tried first, then each mirror in priority
    /// order — the first source that lands a working clone wins.
    ///
    /// Used by `vibe registry sync` to walk lockfile entries and pull
    /// upstream changes for everything currently installed, without
    /// re-applying writes (that's `vibe update`'s job, not sync's).
    pub fn refresh_package(
        &self,
        group: &Group,
        name: &str,
        refname: &str,
    ) -> Result<(), RegistryError> {
        self.ensure_clone_against_sources(group, name, refname)?;
        Ok(())
    }

    fn fetch_acquisition(
        &self,
        resolved: &ResolvedPackage,
        store_root: &Path,
        expected_hash: Option<&str>,
        workspace_slot: Option<&Path>,
        route: Option<&RegistryRoute>,
    ) -> Result<PackageAcquisition, RegistryError> {
        // PROP-002 §2.2.1 — bail before any clone work when this
        // registry is `auth = "token-env"` but the env-var resolved
        // empty.
        self.ensure_token_loaded()?;
        let canonical_url = self.package_repo_url(&resolved.group, &resolved.name)?;
        let tag = self.package_reference(resolved);
        let (primary, mirrors) = self.package_urls(&resolved.group, &resolved.name)?;
        let clone_dir = self.package_clone_dir(&resolved.group, &resolved.name);

        let mut candidates: Vec<String> = std::iter::once(primary.clone())
            .chain(mirrors.iter().cloned())
            .collect();
        if let Some(route) = route {
            let selected = route.validate(self, resolved)?.to_owned();
            // Existing unbound cache refreshes use their own origin first and
            // keep the legacy primary/mirror fallback order. Native candidates
            // retain their explicitly admitted route and checked switch logic.
            if self.backend.uses_native_materialisation() || !clone_dir.join(".git").exists() {
                candidates.retain(|url| url != &selected);
                candidates.insert(0, selected);
            }
        }
        let mut excluded = None;
        let mut primary_err: Option<RegistryError> = None;
        let mut last_cached: Option<CachedPackage> = None;

        for (i, url) in candidates.iter().enumerate() {
            // 1. Bring the local clone to `tag` from this URL. The
            //    first attempt refreshes whatever copy is already on
            //    disk (in place, never deleted on failure); every
            //    later attempt serves a different source and is a
            //    switch — clone beside, swap in only on success
            //    (PROP-010 §2.6).
            let intent = if i == 0 {
                BringIntent::RefreshExisting
            } else {
                BringIntent::SwitchSource
            };
            let native = if self.backend.uses_native_materialisation() {
                let refresh = i == 0
                    && (route.is_none()
                        || !clone_dir.join(".git").exists()
                        || self.backend.recorded_origin_url(&clone_dir)?
                            == strip_git_plus_prefix(url));
                match self.prepare_native_attempt(
                    resolved,
                    url,
                    &clone_dir,
                    refresh,
                    workspace_slot,
                ) {
                    Ok(prepared) => Some(prepared),
                    Err(prepared::PreparationFailure::Admission(error))
                        if excluded_route(&error) =>
                    {
                        excluded.get_or_insert(error);
                        continue;
                    }
                    Err(prepared::PreparationFailure::Admission(error)) => return Err(error),
                    Err(prepared::PreparationFailure::Acquisition(error)) => {
                        primary_err.get_or_insert(error);
                        continue;
                    }
                }
            } else {
                None
            };
            if native.is_none()
                && let Err(e) = self.bring_clone_to_ref(url, &tag, &clone_dir, intent)
            {
                if i == 0 {
                    primary_err = Some(e);
                } else {
                    tracing::debug!(
                        target: "vibe_registry",
                        registry = %self.name,
                        mirror = %url,
                        error = %e,
                        "mirror fetch failed; trying next"
                    );
                }
                continue;
            }
            if let Some(prepared) = &native
                && workspace_slot.is_some()
                && prepared
                    .manifest()
                    .package
                    .as_ref()
                    .is_some_and(|p| p.materialization.is_in_place())
                && expected_hash.is_none_or(|expected| expected == prepared.content_hash())
            {
                return Ok(PackageAcquisition::InPlace(native.ok_or_else(|| {
                    RegistryError::MalformedMeta {
                        path: clone_dir.clone(),
                        reason: "native acquisition disappeared".into(),
                    }
                })?));
            }
            let native_stage = native
                .as_ref()
                .map(PreparedInPlace::stage_native)
                .transpose()?
                .flatten();
            let content = native_stage
                .as_ref()
                .map_or(clone_dir.as_path(), |s| s.staged_path());

            // 2. Read the manifest from the clone to learn how the package
            //    wants to be materialised (PROP-022 §2.1) before paying any
            //    copy cost, and capture the commit the tag resolved to —
            //    recorded so a re-clone reconstructs identical content incl.
            //    submodule gitlinks (PROP-021 §2.4) and an in-place slot's
            //    identity is its commit (PROP-022 §2.5). The clone retains
            //    `.git`; the store entry is `.git`-stripped.
            let clone_manifest_path = content.join(Manifest::FILENAME);
            let manifest = Manifest::read(&clone_manifest_path)?;
            let pkg = manifest
                .package
                .as_ref()
                .ok_or_else(|| RegistryError::MalformedMeta {
                    path: clone_manifest_path.clone(),
                    reason: "registry package manifest must carry a [package] table".to_string(),
                })?;
            let resolved_commit = match &native {
                Some(prepared) => Some(prepared.resolved_commit().to_owned()),
                None => self.backend.head_commit(&clone_dir)?,
            };

            // An `in-place` package (PROP-022 §2.4) is placed as a git working
            // tree, so vibevm never walks its tree: skip the `.git`-stripped
            // store copy and the content-hash tree walk — the very cost the
            // mode exists to avoid for a giant repo. The live clone is handed
            // back as the content dir for the move-into-slot step, and identity
            // is the commit (§2.5), recorded as a cheap commit-derived hash
            // rather than a tree hash.
            let in_place = pkg.materialization.is_in_place();
            let content_hash = if in_place {
                commit_content_hash(resolved_commit.as_deref().unwrap_or_default())
            } else {
                // Hash straight off the clone: the recipe's exclude set
                // skips `.git` (and build output) at every depth, so this
                // equals the hash of the `.git`-stripped shippable tree
                // the store entry will hold — no intermediate copy is
                // needed to know what inserting would pin.
                compute_content_hash(content)?
            };

            // 3. Cross-source content_hash gate — BEFORE any store
            //    insert (the store is written once; a source serving
            //    disagreeing bytes must never become the entry).
            let accepted = match expected_hash {
                None => true,
                Some(expected) => expected == content_hash,
            };
            if !accepted {
                tracing::warn!(
                    target: "vibe_registry",
                    registry = %self.name,
                    url = %url,
                    expected = %expected_hash.unwrap_or_default(),
                    actual = %content_hash,
                    "source served content with unexpected content_hash; \
                     falling through to next source"
                );
                // `cache_dir` points at the clone purely so the
                // disagreeing `CachedPackage` is well-formed; the
                // caller consumes only `content_hash` (the drift
                // signal). Nothing is ever materialised from this
                // value, and the clone is left as this attempt left
                // it — the next URL in the chain takes over as a
                // source switch (clone-beside-and-swap), so a
                // poisoned mirror's tree can never survive INTO the
                // next attempt's clone, without deleting anything on
                // the way (PROP-010 §2.6).
                last_cached = Some(CachedPackage {
                    recorded_source_kind: None,
                    store_reused: None,
                    acquisition: Some(
                        vibe_wire::generated::shared::SourceAcquisition::GitRepository,
                    ),
                    resolved: resolved.clone(),
                    cache_dir: clone_dir.clone(),
                    manifest,
                    content_hash: content_hash.clone(),
                    source_uri: canonical_url.clone(),
                    // The URL this attempt actually read, recorded even
                    // on a drifting source: the reader of the refusal
                    // wants to know which mirror served the other bytes
                    // (`PROP-066#provenance`).
                    fetched_from: native.as_ref().map_or_else(
                        || (url != &canonical_url).then(|| url.to_string()),
                        |p| p.source_evidence().fetched_from,
                    ),
                    registry_name: Some(self.name.clone()),
                    source_ref: Some(tag.clone()),
                    resolved_commit,
                    overridden: false,
                    is_git_source: false,
                    is_path_source: false,
                    is_embedded: false,
                    is_local: false,
                    via_redirect: None,
                });
                continue;
            }

            // 4. Accepted — insert into the machine store, write-once.
            //    The entry path is what the resolution materialises
            //    `vibedeps/` from: the bytes flow from the STORE, not
            //    from this clone.
            if let Some(stage) = &native_stage {
                self.publish_native_cache(stage, &clone_dir)?;
            }
            let mut store_reused = false;
            let cache_dir = if in_place {
                clone_dir.clone()
            } else {
                match store::insert_at(
                    store_root,
                    &clone_dir,
                    &resolved.group,
                    &resolved.name,
                    &resolved.version,
                )? {
                    InsertOutcome::Inserted(entry) => entry,
                    // The §2.5 read gate: the entry pre-dated this
                    // fetch, so the bytes vibedeps will copy are the
                    // ENTRY's, not the fresh clone's that just passed
                    // the pin — verify the entry itself and name a
                    // mismatch instead of silently using altered bytes
                    // (PROP-010 §2.7, mismatch-is-named). Without a pin
                    // the entry is trusted as-is: re-hashing the store
                    // on every fetch is `vibe cache check`'s job, not a
                    // tax the ordinary path pays.
                    InsertOutcome::AlreadyPresent(entry) => {
                        store_reused = true;
                        if let Some(expected) = expected_hash {
                            verify_store_entry_against_pin(
                                &entry,
                                expected,
                                &resolved.group,
                                &resolved.name,
                                &resolved.version,
                            )?;
                        }
                        entry
                    }
                }
            };

            if i > 0 {
                if expected_hash.is_some() {
                    tracing::info!(
                        target: "vibe_registry",
                        registry = %self.name,
                        primary = %primary,
                        served_by = %url,
                        mirror_index = i - 1,
                        "fetch served by mirror; content_hash matches lockfile pin"
                    );
                } else {
                    tracing::info!(
                        target: "vibe_registry",
                        registry = %self.name,
                        primary = %primary,
                        served_by = %url,
                        mirror_index = i - 1,
                        "fetch served by mirror"
                    );
                }
            }
            return Ok(PackageAcquisition::Cached(Box::new(CachedPackage {
                recorded_source_kind: None,
                store_reused: Some(store_reused),
                acquisition: Some(vibe_wire::generated::shared::SourceAcquisition::GitRepository),
                resolved: resolved.clone(),
                cache_dir,
                manifest,
                content_hash,
                source_uri: canonical_url.clone(),
                // The candidate that served the bytes, when it was not
                // the canonical origin (`PROP-066#provenance`). The
                // manifest and the lock keep the canonical URL, so a
                // lock stays portable between networks and the place the
                // bytes came from is still recoverable.
                fetched_from: native.as_ref().map_or_else(
                    || (url != &canonical_url).then(|| url.to_string()),
                    |p| p.source_evidence().fetched_from,
                ),
                registry_name: Some(self.name.clone()),
                source_ref: Some(tag.clone()),
                resolved_commit,
                overridden: false,
                is_git_source: false,
                is_path_source: false,
                is_embedded: false,
                is_local: false,
                via_redirect: None,
            })));
        }

        // Every URL was exhausted.
        if let Some(cached) = last_cached {
            // At least one source served content; none matched the
            // expected hash. Return the last one — `vibe-install`'s
            // `plan_install` will lift this into a `ContentDrift`
            // error against the lockfile pin and surface the actionable
            // message. Doing the rendering here would duplicate that
            // logic and lose the lockfile context the install layer
            // already carries.
            return Ok(PackageAcquisition::Cached(Box::new(cached)));
        }
        Err(primary_err
            .or(excluded)
            .unwrap_or_else(|| RegistryError::MalformedMeta {
                path: clone_dir,
                reason: "source chain produced neither a package nor an acquisition error".into(),
            }))
    }
}

#[cfg(test)]
#[path = "fetch/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "fetch/tests_in_place.rs"]
mod tests_in_place;

#[cfg(test)]
#[path = "fetch/store_gate_tests.rs"]
mod store_gate_tests;

#[cfg(test)]
#[path = "fetch/refresh_switch_tests.rs"]
mod refresh_switch_tests;
