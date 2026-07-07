//! Clone / update orchestration for the per-package registry —
//! mirror-aware fetch dispatch with the cross-source content-hash
//! gate, and materialisation into the per-project cache (PROP-002
//! §2.3 / §2.6). The clone-free lookup half (version listing,
//! archive-first manifest reads) lives in [`super::lookup`].

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

impl GitPackageRegistry {
    /// Bootstrap (or refresh) the per-package clone at `clone_dir`
    /// against `url`. Used by [`Self::ensure_clone_against_sources`]
    /// and the mirror-fallback variants of [`Self::fetch`] /
    /// [`Self::refresh_package`].
    ///
    /// On entry: `clone_dir` is either absent, empty, or a previously
    /// populated git working tree. If the working tree exists,
    /// [`GitBackend::update`] is tried first — that preserves the local
    /// clone and is the cheap path. If `update` fails (origin
    /// unreachable, ref missing, etc.), the clone is wiped and we
    /// retry via [`GitBackend::bootstrap`] against the same URL. The
    /// wipe-and-rebootstrap branch is what allows the next mirror in
    /// the chain to take over cleanly even if a previous clone left
    /// stale state behind.
    fn bootstrap_or_update_at(
        &self,
        url: &str,
        refname: &str,
        clone_dir: &Path,
    ) -> Result<(), RegistryError> {
        if clone_dir.join(".git").exists() {
            match self.backend.update(clone_dir, refname) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::debug!(
                        target: "vibe_registry",
                        registry = %self.name,
                        url = %url,
                        error = %e,
                        "update on existing clone failed; wiping and re-bootstrapping"
                    );
                    fs::remove_dir_all(clone_dir).map_err(|source| RegistryError::Io {
                        path: clone_dir.to_path_buf(),
                        source,
                    })?;
                }
            }
        }
        if clone_dir.exists() {
            // Half-populated dir from a prior failed bootstrap — clean.
            fs::remove_dir_all(clone_dir).map_err(|source| RegistryError::Io {
                path: clone_dir.to_path_buf(),
                source,
            })?;
        }
        if let Some(parent) = clone_dir.parent() {
            fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        // PROP-002 §2.2.1 — under `auth = "token-env"` the bootstrap
        // is performed with a credentialised URL, then the recorded
        // origin URL is rewritten to the plain (token-free) form so
        // the freshly-cloned `.git/config` does NOT carry the token
        // on disk. Subsequent `update` calls hit the plain origin
        // (and 401 on a still-private host); the
        // `ensure_clone_against_sources` retry path handles that by
        // wiping and re-bootstrapping. The token only ever lives in
        // memory and inside the spawned `git clone` process.
        let plain_url = strip_git_plus_prefix(url);
        let fetch_url = inject_token(plain_url, self.effective_token.as_deref());
        self.backend.bootstrap(&fetch_url, refname, clone_dir)?;
        if self.effective_token.is_some() {
            self.backend
                .set_remote_url(clone_dir, "origin", plain_url)?;
        }
        Ok(())
    }

    /// Bring the per-package clone at `package_clone_dir(kind, name)`
    /// to `refname` by trying the primary URL first, then each mirror
    /// URL in priority order. Returns the URL that ultimately served
    /// the clone (canonical or a mirror) so the caller can record /
    /// log it.
    ///
    /// Mirror dispatch on this path is the cache-mutating sibling of
    /// [`Self::try_lookup`]: same primary-first ordering, same
    /// "primary's error is the most informative" semantics on full
    /// failure. The crucial difference is per-source state — each
    /// retry that goes through bootstrap wipes the local clone first,
    /// so a flapping primary that left a half-populated dir cannot
    /// poison the mirror attempt.
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

    /// Bootstrap (or incrementally update) `dest` against the package's source
    /// chain — the primary URL first, then each mirror — returning the URL
    /// that served it. The mirror-aware, auth-aware core shared by the cache
    /// clone ([`ensure_clone_against_sources`]) and the direct in-place slot
    /// placement ([`materialise_in_place`]); only the destination differs.
    fn bootstrap_chain_into(
        &self,
        group: &Group,
        name: &str,
        refname: &str,
        dest: &Path,
    ) -> Result<String, RegistryError> {
        let (primary, mirrors) = self.package_urls(group, name)?;
        // Primary outside the mirror loop — its error is a plain value.
        let primary_err = match self.bootstrap_or_update_at(&primary, refname, dest) {
            Ok(()) => return Ok(primary),
            Err(e) => e,
        };
        for (i, url) in mirrors.iter().enumerate() {
            match self.bootstrap_or_update_at(url, refname, dest) {
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
                        "mirror fetch failed; trying next"
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
    /// and the per-project cache clone entirely. Auth / mirror handling is the
    /// same [`bootstrap_or_update_at`] the cache path uses — the token is
    /// injected and stripped identically; only the working-tree destination
    /// changes. Returns the canonical source URL, the version tag, the
    /// resolved commit (the in-place identity, §2.5), and the slot's manifest.
    pub fn materialise_in_place(
        &self,
        resolved: &ResolvedPackage,
        slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        self.ensure_token_loaded()?;
        let canonical_url = self.package_repo_url(&resolved.group, &resolved.name)?;
        let tag = format!("v{}", resolved.version);
        self.bootstrap_chain_into(&resolved.group, &resolved.name, &tag, slot)?;
        let manifest_path = slot.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path,
                reason: "in-place package manifest must carry a [package] table".to_string(),
            });
        }
        let resolved_commit = self.backend.head_commit(slot).ok().flatten();
        let content_hash = commit_content_hash(resolved_commit.as_deref().unwrap_or_default());
        Ok(InPlaceMaterialised {
            source_uri: canonical_url,
            source_ref: tag,
            resolved_commit,
            content_hash,
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

    /// Materialise the resolved package into the per-project cache. Clones
    /// (or updates) the per-package repo at the requested tag, then copies
    /// the worktree into `<cache_root>/<kind>/<name>/v<version>/`,
    /// stripping `.git/`.
    ///
    /// Mirror-aware: the primary URL is tried first, then each mirror
    /// in priority order. Whichever source lands the clone first wins
    /// and the cache is materialised from that clone. The
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
        cache_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        self.fetch_with_expected_hash(resolved, cache_root, None)
    }

    /// Mirror-aware fetch with an optional cross-source content_hash
    /// gate.
    ///
    /// Walks the URL chain primary-first; for each URL that yields a
    /// working clone, materialises the cache and computes the
    /// content_hash:
    ///
    /// - If `expected_hash` is `None` (no lockfile pin), accept the
    ///   first source that lands content. Equivalent to [`Self::fetch`].
    /// - If `expected_hash` is `Some(h)`, accept the first source
    ///   whose computed hash equals `h`. Sources serving a disagreeing
    ///   hash trigger a `tracing::warn!` (mirror-integrity event) and
    ///   the walk continues to the next URL — the cache is wiped
    ///   between attempts so a poisoned source cannot leave bytes
    ///   behind. This is the supply-chain check from
    ///   [PROP-002 §2.3](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#mirror).
    ///
    /// If every URL is reached but none matches, the **last
    /// successful fetch's** [`CachedPackage`] is returned (with the
    /// disagreeing hash); it is the caller's responsibility — today
    /// `vibe-install`'s `plan_install` — to convert the stored hash
    /// vs. lockfile-pin mismatch into the user-actionable
    /// `ContentDrift` error. This split keeps registry-layer concerns
    /// (sources, fallback, integrity attempts) separate from
    /// install-layer concerns (lockfile-aware error rendering).
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
        cache_root: &Path,
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
        let dest_cache = cache_root
            .join(resolved.group.as_str())
            .join(&resolved.name)
            .join(format!("v{}", resolved.version));

        let mut primary_err: Option<RegistryError> = None;
        let mut last_cached: Option<CachedPackage> = None;

        for (i, url) in std::iter::once(&primary).chain(mirrors.iter()).enumerate() {
            // 1. Bring the local clone to `tag` from this URL.
            if let Err(e) = self.bootstrap_or_update_at(url, &tag, &clone_dir) {
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
            //    `.git`; the per-project cache copy is `.git`-stripped.
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
            // cache copy and the content-hash tree walk — the very cost the
            // mode exists to avoid for a giant repo. The live clone is handed
            // back as the content dir for the move-into-slot step, and identity
            // is the commit (§2.5), recorded as a cheap commit-derived hash
            // rather than a tree hash.
            let (cache_dir, content_hash) = if pkg.materialization.is_in_place() {
                (
                    clone_dir.clone(),
                    commit_content_hash(resolved_commit.as_deref().unwrap_or_default()),
                )
            } else {
                if dest_cache.exists() {
                    fs::remove_dir_all(&dest_cache).map_err(|source| RegistryError::Io {
                        path: dest_cache.clone(),
                        source,
                    })?;
                }
                copy_dir_excluding_git(&clone_dir, &dest_cache)?;
                (dest_cache.clone(), compute_content_hash(&dest_cache)?)
            };

            // 3. Cross-source content_hash gate.
            let cached = CachedPackage {
                resolved: resolved.clone(),
                cache_dir,
                manifest,
                content_hash: content_hash.clone(),
                source_uri: canonical_url.clone(),
                registry_name: Some(self.name.clone()),
                source_ref: Some(tag.clone()),
                resolved_commit,
                overridden: false,
                is_git_source: false,
                is_path_source: false,
                via_redirect: None,
            };
            match expected_hash {
                None => {
                    if i > 0 {
                        tracing::info!(
                            target: "vibe_registry",
                            registry = %self.name,
                            primary = %primary,
                            served_by = %url,
                            mirror_index = i - 1,
                            "fetch served by mirror"
                        );
                    }
                    return Ok(cached);
                }
                Some(expected) if expected == content_hash => {
                    if i > 0 {
                        tracing::info!(
                            target: "vibe_registry",
                            registry = %self.name,
                            primary = %primary,
                            served_by = %url,
                            mirror_index = i - 1,
                            "fetch served by mirror; content_hash matches lockfile pin"
                        );
                    }
                    return Ok(cached);
                }
                Some(expected) => {
                    tracing::warn!(
                        target: "vibe_registry",
                        registry = %self.name,
                        url = %url,
                        expected = %expected,
                        actual = %content_hash,
                        "source served content with unexpected content_hash; \
                         falling through to next source"
                    );
                    last_cached = Some(cached);
                    // Wipe the local clone state so the next URL bootstraps
                    // fresh — a poisoned mirror's working tree must not
                    // survive into the next attempt.
                    if clone_dir.exists() {
                        fs::remove_dir_all(&clone_dir).map_err(|source| RegistryError::Io {
                            path: clone_dir.clone(),
                            source,
                        })?;
                    }
                }
            }
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

/// Recursively copy `src` into `dst`, excluding any `.git` directory at any
/// depth. Used to materialise a clone into the package cache without
/// dragging the git index along — the cache holds payload only, identity
/// rides on `content_hash`. `pub(crate)` because the multi-registry
/// resolver shares the same materialisation path for `[[override]]` clones.
pub(crate) fn copy_dir_excluding_git(src: &Path, dst: &Path) -> Result<(), RegistryError> {
    fs::create_dir_all(dst).map_err(|source| RegistryError::Io {
        path: dst.to_path_buf(),
        source,
    })?;
    for entry in walkdir::WalkDir::new(src)
        .into_iter()
        .filter_entry(|e| e.file_name() != std::ffi::OsStr::new(".git"))
        .filter_map(|e| e.ok())
    {
        let rel = entry.path().strip_prefix(src).unwrap_or(entry.path());
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|source| RegistryError::Io {
                path: target.clone(),
                source,
            })?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            fs::copy(entry.path(), &target).map_err(|source| RegistryError::Io {
                path: target.clone(),
                source,
            })?;
        }
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
