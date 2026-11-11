//! Public fetch contracts delegate to the retained acquisition owner.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");
use super::*;

impl GitPerPackageRegistry {
    pub(crate) fn native_source_for(&self, address: &str) -> Result<bool, GitError> {
        self.backend.preflight_selected_address(address)?;
        self.backend.native_materialisation_for(address)
    }
    pub(crate) fn ensure_selected_clone(
        &self,
        reference: &str,
        destination: &Path,
    ) -> Result<(), RegistryError> {
        let address = self.org_url.as_str();
        let refresh = destination.join(".git").exists()
            && self.backend.recorded_origin_url(destination)? == strip_git_plus_prefix(address);
        self.bring_clone_to_ref(
            &crate::git_failure::GitOperation::capture_mirror(self.name()),
            address,
            reference,
            destination,
            if refresh {
                BringIntent::RefreshExisting
            } else {
                BringIntent::SwitchSource
            },
            true,
        )
    }

    pub(crate) fn with_locked_source(mut self) -> Self {
        self.locked_source = true;
        self
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
    pub fn fetch_with_expected_hash(
        &self,
        resolved: &ResolvedPackage,
        store_root: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        match self.fetch_acquisition(resolved, store_root, expected_hash, None, None)? {
            PackageAcquisition::Cached(cached) => Ok(*cached),
            PackageAcquisition::InPlace(prepared) => Err(RegistryError::PreparedInPlace {
                path: prepared.slot().into(),
                reason: "workspace-bound preparation requires the install acquisition consumer"
                    .into(),
            }),
        }
    }

    /// Select once, retaining native in-place objects until checked workspace apply.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
    )]
    pub fn fetch_for_install(
        &self,
        resolved: &ResolvedPackage,
        store_root: &Path,
        expected_hash: Option<&str>,
        workspace_slot: &Path,
    ) -> Result<PackageAcquisition, RegistryError> {
        self.fetch_acquisition(
            resolved,
            store_root,
            expected_hash,
            Some(workspace_slot),
            None,
        )
    }

    pub(crate) fn fetch_on_route(
        &self,
        resolved: &ResolvedPackage,
        store_root: &Path,
        expected_hash: Option<&str>,
        workspace_slot: Option<&Path>,
        route: Option<&RegistryRoute>,
    ) -> Result<PackageAcquisition, RegistryError> {
        self.fetch_acquisition(resolved, store_root, expected_hash, workspace_slot, route)
    }
}

impl GitPerPackageRegistry {
    pub(crate) fn fetch_locked_selection(
        mut self,
        resolved: &ResolvedPackage,
        store: &Path,
        expected: Option<&str>,
        slot: Option<&Path>,
    ) -> Result<PackageAcquisition, RegistryError> {
        self.locked_source = true;
        self.fetch_acquisition(resolved, store, expected, slot, None)
    }
    /// Replay (`preserve_modifications`) refuses a modified held slot; an
    /// explicit version update resets it as an ordinary in-place update does.
    pub(crate) fn prepare_locked_selection(
        mut self,
        resolved: &ResolvedPackage,
        slot: &Path,
        preserve_modifications: bool,
    ) -> Result<PreparedInPlace, RegistryError> {
        self.locked_source = true;
        if preserve_modifications
            && slot.join(".git").exists()
            && self.backend.working_tree_dirty(slot)?
        {
            return Err(RegistryError::PreparedInPlace {
                path: slot.into(),
                reason: "locked replay preserves local checkout modifications".into(),
            });
        }
        if self.backend.uses_native_materialisation() {
            let target = self.package_repo_url(&resolved.group, &resolved.name)?;
            return self
                .prepare_native_attempt(resolved, &target, slot, false, Some(slot))
                .map_err(|error| match error {
                    prepared::PreparationFailure::Acquisition(error)
                    | prepared::PreparationFailure::Admission(error) => error,
                });
        }
        self.prepare_in_place_on_route(resolved, slot, None)
    }
}
