//! Clone / update orchestration for the per-package registry —
//! mirror-aware fetch dispatch with the cross-source content-hash
//! gate, then write-once insertion of the accepted payload into the
//! machine-global store `~/.vibe/cache/` (PROP-002 §2.3 / §2.6;
//! PROP-010 §2.7). The clone-free lookup half (version listing,
//! archive-first manifest reads) lives in [`super::lookup`].

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;
use crate::store::{self, InsertOutcome};

mod bring;
use bring::BringIntent;

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
        let tag = format!("v{}", resolved.version);
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

    /// Fetch the resolved package into the machine-global store.
    /// Clones (or updates) the per-package repo at the requested tag,
    /// then — once a source is accepted — inserts the `.git`-stripped
    /// worktree into `<store_root>/<group>/<name>/v<version>/`
    /// write-once (PROP-010 §2.7).
    ///
    /// Mirror-aware: the primary URL is tried first, then each mirror
    /// in priority order. Whichever source lands the clone first wins
    /// and the store entry is inserted from that clone. The
    /// [`CachedPackage::source_uri`] is **always** the canonical
    /// primary URL — mirror URLs are an availability detail, not a
    /// lockfile-recorded identity (PROP-002 §2.3 step 3).
    ///
    /// No content_hash gate at this layer — see
    /// [`Self::fetch_with_expected_hash`] for the cross-source
    /// integrity check.
    pub fn fetch(
        &self,
        resolved: &ResolvedPackage,
        store_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        self.fetch_with_expected_hash(resolved, store_root, None)
    }

    /// Mirror-aware fetch with an optional cross-source content_hash
    /// gate, inserting the accepted payload into the machine-global
    /// store (PROP-010 §2.7).
    ///
    /// Walks the URL chain primary-first; for each URL that yields a
    /// working clone, computes the content hash and applies the gate:
    ///
    /// - If `expected_hash` is `None` (no lockfile pin), accept the
    ///   first source that lands content. Equivalent to [`Self::fetch`].
    /// - If `expected_hash` is `Some(h)`, accept the first source
    ///   whose computed hash equals `h`. Sources serving a disagreeing
    ///   hash trigger a `tracing::warn!` (mirror-integrity event) and
    ///   the walk continues to the next URL — as a source switch
    ///   (clone-beside-and-swap), so a poisoned source's bytes can
    ///   never survive into the next attempt's clone while nothing on
    ///   disk is deleted along the way (PROP-010 §2.6). This is the
    ///   supply-chain check from
    ///   [PROP-002 §2.3](../../../vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml#mirror).
    ///
    /// **The store insert happens only for an accepted source** — the
    /// store is written once (`~/.vibe/cache/`, PROP-010 §2.7), so a
    /// rejected mirror's bytes must never become the entry. The
    /// returned [`CachedPackage::cache_dir`] is the store entry's
    /// path: `vibedeps/` materialisation reads its bytes from there.
    /// When the entry already existed, `insert_at` rewrites nothing,
    /// and — with a pin on record — the entry itself is verified
    /// against the pin before it becomes the materialisation source:
    /// a tampered store is named, never silently used and never
    /// silently re-downloaded (PROP-010 §2.7, mismatch-is-named).
    /// Without a pin the entry is trusted as-is; re-hashing the store
    /// on every resolve is `vibe cache check`'s job, not a tax the
    /// ordinary path pays.
    ///
    /// If every URL is reached but none matches, the **last
    /// successful fetch's** [`CachedPackage`] is returned (with the
    /// disagreeing hash — nothing was inserted); it is the caller's
    /// responsibility — today `vibe-install`'s `plan_install` — to
    /// convert the stored hash vs. lockfile-pin mismatch into the
    /// user-actionable `ContentDrift` error. Its `cache_dir` points
    /// at the clone directory purely so the value is well-formed; the
    /// caller consumes only `content_hash`, the drift signal, and the
    /// clone itself stays as the last attempt left it. This
    /// split keeps registry-layer concerns (sources, fallback,
    /// integrity attempts) separate from install-layer concerns
    /// (lockfile-aware error rendering).
    ///
    /// If every URL fails at the network layer (no source produced
    /// any content), the **primary's** error is surfaced — same
    /// "primary is canonical and its diagnostic is most useful"
    /// semantics as [`Self::try_lookup`].
    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "no-unwrap-in-domain: primary_err is Some whenever the source loop \
                  exhausts — the primary URL exists by package_urls' type and its \
                  failure is recorded before any continue; lifting the primary out \
                  of the loop (the try_lookup shape) would duplicate the three-step \
                  per-source body this fn shares between primary and mirrors"
    )]
    pub fn fetch_with_expected_hash(
        &self,
        resolved: &ResolvedPackage,
        store_root: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        // PROP-002 §2.2.1 — bail before any clone work when this
        // registry is `auth = "token-env"` but the env-var resolved
        // empty.
        self.ensure_token_loaded()?;
        let canonical_url = self.package_repo_url(&resolved.group, &resolved.name)?;
        let tag = format!("v{}", resolved.version);
        let (primary, mirrors) = self.package_urls(&resolved.group, &resolved.name)?;
        let clone_dir = self.package_clone_dir(&resolved.group, &resolved.name);

        let mut primary_err: Option<RegistryError> = None;
        let mut last_cached: Option<CachedPackage> = None;

        for (i, url) in std::iter::once(&primary).chain(mirrors.iter()).enumerate() {
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
            if let Err(e) = self.bring_clone_to_ref(url, &tag, &clone_dir, intent) {
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

            // 2. Read the manifest from the clone to learn how the package
            //    wants to be materialised (PROP-022 §2.1) before paying any
            //    copy cost, and capture the commit the tag resolved to —
            //    recorded so a re-clone reconstructs identical content incl.
            //    submodule gitlinks (PROP-021 §2.4) and an in-place slot's
            //    identity is its commit (PROP-022 §2.5). The clone retains
            //    `.git`; the store entry is `.git`-stripped.
            let clone_manifest_path = clone_dir.join(Manifest::FILENAME);
            let manifest = Manifest::read(&clone_manifest_path)?;
            let pkg = manifest
                .package
                .as_ref()
                .ok_or_else(|| RegistryError::MalformedMeta {
                    path: clone_manifest_path.clone(),
                    reason: "registry package manifest must carry a [package] table".to_string(),
                })?;
            let resolved_commit = self.backend.head_commit(&clone_dir)?;

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
                compute_content_hash(&clone_dir)?
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
                    resolved: resolved.clone(),
                    cache_dir: clone_dir.clone(),
                    manifest,
                    content_hash: content_hash.clone(),
                    source_uri: canonical_url.clone(),
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
            return Ok(CachedPackage {
                resolved: resolved.clone(),
                cache_dir,
                manifest,
                content_hash,
                source_uri: canonical_url.clone(),
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
            return Ok(cached);
        }
        Err(primary_err.expect("primary URL must exist"))
    }
}

/// The one entry-under-pin verification gate (PROP-010 §2.7,
/// mismatch-is-named): re-hash the store entry and, when it no longer
/// matches the lockfile pin, name the package and the entry. Shared by
/// the fetch path's `AlreadyPresent` branch and the store-backed
/// resolution's fetch short-circuit, so both apply the SAME gate, not
/// a copy. `pub(crate)` for the multi-registry resolver's offline
/// module; `Group`/version arrive by reference so the error carries
/// the identity verbatim.
pub(crate) fn verify_store_entry_against_pin(
    entry: &Path,
    expected: &str,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<(), RegistryError> {
    let actual = compute_content_hash(entry)?;
    if actual != expected {
        return Err(RegistryError::StoreEntryMismatch {
            detail: Box::new(crate::error::StoreEntryMismatchDetail {
                group: group.clone(),
                name: name.to_string(),
                version: version.clone(),
                path: entry.to_path_buf(),
                expected: expected.to_string(),
                actual,
            }),
        });
    }
    Ok(())
}

/// A cheap, stable `content_hash` for an `in-place` package — `sha256` of the
/// resolved commit, not a tree walk (PROP-022 §2.4/§2.5). The lockfile's
/// `content_hash` field is non-optional, so an in-place slot still records a
/// well-formed `sha256:<hex>`; identity, though, is the `resolved_commit`.
/// An empty commit (a backend that reported none) hashes the empty string —
/// deterministic and harmless, since in-place requires a real git source.
fn commit_content_hash(commit: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(commit.as_bytes());
    let hex = digest.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(&mut s, "{b:02x}");
        s
    });
    format!("sha256:{hex}")
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
