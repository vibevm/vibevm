//! Held lock rows choose source before the mutable registry catalog.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::offline::version_matches;
use super::*;
use vibe_core::manifest::{LockedPackage, SourceKind};

/// Runtime selection carried through the existing resolution, never a wire field.
/// ```
/// # fn inspect(r: &vibe_registry::MultiResolution) {
/// let held: bool = r.locked.is_some();
/// # let _ = held;
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct LockedSelection {
    original: Arc<LockedPackage>,
    pub(crate) row: LockedPackage,
    pub(crate) revision: LockedRevision,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LockedRevision {
    Replay,
    Update,
}

fn refusal(reason: &str) -> RegistryError {
    RegistryError::MalformedMeta {
        path: PathBuf::from(Lockfile::FILENAME),
        reason: format!(
            "locked source: {reason}; violates spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW; fix: restore the locked source or explicitly update its version"
        ),
    }
}
impl LockedSelection {
    /// ```
    /// # fn inspect(r: &vibe_registry::MultiResolution) {
    /// if let Some(held) = &r.locked { let _original_kind = held.source_row().source_kind; }
    /// # }
    /// ```
    /// Original held row before an explicitly granted same-source version update.
    pub fn original_row(&self) -> &LockedPackage {
        &self.original
    }
    pub fn source_row(&self) -> &LockedPackage {
        &self.row
    }
    /// Original revision facts exist only for replay, not explicit version update.
    /// ```
    /// # fn inspect(r: &vibe_registry::MultiResolution) {
    /// if let Some(held) = &r.locked { let _replay = held.replayed_row(); }
    /// # }
    /// ```
    pub fn replayed_row(&self) -> Option<&LockedPackage> {
        (self.revision == LockedRevision::Replay).then_some(&self.row)
    }
    pub(crate) fn project_evidence(
        &self,
        evidence: &mut vibe_wire::generated::shared::SourceEvidence,
    ) {
        let held = crate::locked_source_evidence(&self.row);
        evidence.registry = held.registry;
        evidence.source_url = held.source_url;
        evidence.source_ref = held.source_ref;
        evidence.source_kind = held.source_kind;
        evidence.via_redirect = held.via_redirect;
        if self.revision == LockedRevision::Replay {
            evidence.resolved_commit = held.resolved_commit;
        }
    }

    fn reference(&self) -> Result<&str, RegistryError> {
        self.row
            .resolved_commit
            .as_deref()
            .or(self.row.source_ref.as_deref())
            .ok_or_else(|| refusal("Git row has neither commit nor ref"))
    }
    fn expected(&self) -> Option<&str> {
        (self.revision == LockedRevision::Replay).then_some(self.row.content_hash.as_str())
    }
    fn verify(&self, hash: &str, commit: Option<&str>) -> Result<(), RegistryError> {
        if let Some(expected) = self.expected().filter(|expected| *expected != hash) {
            return Err(RegistryError::LockedContentMismatch {
                detail: Box::new(crate::LockedContentMismatchDetail {
                    group: self.row.group.to_string(),
                    name: self.row.name.to_string(),
                    version: self.row.version.clone(),
                    expected: expected.to_owned(),
                    actual: hash.to_owned(),
                }),
            });
        }
        if self
            .row
            .resolved_commit
            .as_deref()
            .is_some_and(|expected| Some(expected) != commit)
        {
            return Err(refusal(
                "acquired commit differs from the held object identity",
            ));
        }
        Ok(())
    }
    fn apply_evidence(&self, cached: &mut CachedPackage) {
        cached.locked_selection = Some(self.clone());
        let actual = cached
            .fetched_from
            .clone()
            .unwrap_or_else(|| cached.source_uri.clone());
        if cached.acquisition != Some(vibe_wire::generated::shared::SourceAcquisition::Store) {
            cached.fetched_from = (vibe_core::progress::transfer::safe_remote(&actual)
                != vibe_core::progress::transfer::safe_remote(self.row.source_url.as_str()))
            .then(|| vibe_core::progress::transfer::safe_remote(&actual));
        }
        cached.source_uri = self.row.source_url.to_string();
        // Store reuse supplies historical identity; an actual Git acquisition
        // keeps its observed commit. Durable replay projects the retained row.
        if self.revision == LockedRevision::Replay
            && cached.acquisition == Some(vibe_wire::generated::shared::SourceAcquisition::Store)
        {
            cached.resolved_commit = self.row.resolved_commit.clone();
        }
        cached.source_ref = self.row.source_ref.clone();
        cached.registry_name = self.row.registry.clone();
        cached.recorded_source_kind = self.row.source_kind;
        cached.overridden = self.row.overridden;
        cached.via_redirect = self.row.via_redirect.clone();
        cached.is_git_source = self.row.source_kind == Some(SourceKind::Git);
        cached.is_path_source = self.row.source_kind == Some(SourceKind::Path);
        cached.is_embedded = self.row.source_kind == Some(SourceKind::Embedded);
        cached.is_local = self.row.source_kind == Some(SourceKind::Local);
    }
}
impl MultiRegistryResolver {
    /// Whether this coordinate is held, without probing any source.
    /// ```
    /// # fn inspect(r: &vibe_registry::MultiRegistryResolver, g: &vibe_core::Group) {
    /// let _held: bool = r.has_locked_package(g, "example");
    /// # }
    /// ```
    pub fn has_locked_package(&self, group: &Group, name: &str) -> bool {
        self.locked.contains_key(&format!("{group}/{name}"))
    }
    /// Explicit update permits version selection at held endpoints, never a source reset.
    /// ```
    /// # fn update(r: vibe_registry::MultiRegistryResolver) {
    /// let _same_sources = r.with_locked_updates();
    /// # }
    /// ```
    pub fn with_locked_updates(mut self) -> Self {
        self.candidate_owner = Arc::new(());
        self.locked_updates = true;
        self
    }
    /// Held replay acquires from the locked source itself, never from a
    /// verified store entry (`vibe reinstall --force`). The caller still
    /// verifies the acquired bytes against the locked content hash.
    /// ```
    /// # fn force(r: vibe_registry::MultiRegistryResolver) {
    /// let _r = r.with_forced_refetch();
    /// # }
    /// ```
    pub fn with_forced_refetch(mut self) -> Self {
        self.candidate_owner = Arc::new(());
        self.forced_refetch = true;
        self
    }
    /// Explicit owner request to choose these held coordinates' sources again
    /// through the current registry configuration, exactly as new packages.
    /// Unnamed held coordinates keep replaying their locked source.
    /// ```
    /// # fn update(r: vibe_registry::MultiRegistryResolver) {
    /// let _r = r.with_reselected_sources(["org.example/demo".to_string()]);
    /// # }
    /// ```
    pub fn with_reselected_sources(mut self, keys: impl IntoIterator<Item = String>) -> Self {
        self.candidate_owner = Arc::new(());
        for key in keys {
            self.locked.remove(&key);
            self.locked_checkouts.remove(&key);
        }
        self
    }
    /// Refresh these held coordinates from their locked directory (an
    /// in-workspace mutable `file://` working tree, PROP-011 §2.6): the
    /// same source, re-hashed, never satisfied by the held hash or a store
    /// entry. A row whose locked source is not a directory keeps replaying.
    /// ```
    /// # fn refresh(r: vibe_registry::MultiRegistryResolver) {
    /// let _r = r.with_mutable_locked_sources(["org.example/demo".to_string()]);
    /// # }
    /// ```
    pub fn with_mutable_locked_sources(mut self, keys: impl IntoIterator<Item = String>) -> Self {
        self.candidate_owner = Arc::new(());
        self.mutable_locked.extend(keys);
        self
    }
    /// ```
    /// # fn inspect(r: &vibe_registry::MultiRegistryResolver, p: &vibe_core::PackageRef) {
    /// let _replay = r.replays_locked_source(p);
    /// # }
    /// ```
    pub fn replays_locked_source(&self, pkg: &PackageRef) -> bool {
        let key = pkg.qualified_name();
        !self.updates_locked(&key) && self.locked.contains_key(&key)
    }
    /// ```
    /// # fn bind(r: vibe_registry::MultiRegistryResolver) {
    /// let _r = r.with_locked_checkouts(std::collections::HashMap::new());
    /// # }
    /// ```
    pub fn with_locked_checkouts(mut self, paths: HashMap<String, PathBuf>) -> Self {
        self.candidate_owner = Arc::new(());
        self.locked_checkouts = paths;
        self
    }
    fn reuse_locked_checkout(
        &self,
        resolution: &MultiResolution,
        selection: &LockedSelection,
        slot: Option<&Path>,
    ) -> Result<Option<CachedPackage>, RegistryError> {
        if selection.revision != LockedRevision::Replay
            || !selection.row.materialization.is_in_place()
        {
            return Ok(None);
        }
        let Some(path) = slot.or_else(|| {
            self.locked_checkouts
                .get(&format!("{}/{}", selection.row.group, selection.row.name))
                .map(PathBuf::as_path)
        }) else {
            return Ok(None);
        };
        if !path.join(".git").exists() {
            return Ok(None);
        }
        let commit = self.backend.head_commit(path)?;
        let dirty = self.backend.working_tree_dirty(path)?;
        if slot.is_none() && (dirty || commit != selection.row.resolved_commit) {
            // A metadata read is not a placement: the checkout is left as-is
            // and the held manifest comes from the store or locked source.
            return Ok(None);
        }
        if dirty {
            return Err(refusal(
                "held checkout contains local modifications; left untouched",
            ));
        }
        if commit != selection.row.resolved_commit {
            return Err(refusal(
                "held checkout HEAD differs from the locked commit; left untouched",
            ));
        }
        let Some(commit) = commit else {
            return Err(refusal("checkout has no verified commit"));
        };
        let hash = crate::git_package_registry::locked_commit_hash(&commit);
        selection.verify(&hash, Some(&commit))?;
        let manifest = Manifest::read(path.join(Manifest::FILENAME))?;
        let mut cached = CachedPackage {
            resolved: ResolvedPackage {
                source_dir: path.to_path_buf(),
                ..resolution.resolved.clone()
            },
            cache_dir: path.to_path_buf(),
            manifest,
            content_hash: hash,
            source_uri: selection.row.source_url.to_string(),
            fetched_from: None,
            registry_name: selection.row.registry.clone(),
            source_ref: selection.row.source_ref.clone(),
            resolved_commit: Some(commit),
            locked_selection: None,
            route_publication: vibe_core::RoutePublication::Local,
            recorded_source_kind: selection.row.source_kind,
            acquisition: Some(vibe_wire::generated::shared::SourceAcquisition::GitRepository),
            store_reused: Some(false),
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_local: false,
            is_embedded: false,
            via_redirect: None,
        };
        selection.apply_evidence(&mut cached);
        Ok(Some(cached))
    }
    pub(super) fn locked_manifest(
        &self,
        resolution: &MultiResolution,
        locked: &LockedSelection,
    ) -> Result<Manifest, RegistryError> {
        let metadata_store = self
            .metadata_store_root
            .clone()
            .unwrap_or_else(|| self.cache_root.join("metadata-store"));
        match self.fetch_locked(resolution, locked, &metadata_store, None)? {
            crate::PackageAcquisition::Cached(cached) => Ok(cached.manifest),
            crate::PackageAcquisition::InPlace(_) => {
                Err(refusal("metadata returned a workspace-bound preparation"))
            }
        }
    }
    fn held_ref_version(&self, row: &LockedPackage) -> Result<semver::Version, RegistryError> {
        let reference = self.held_update_reference(row)?;
        let mut selected = LockedSelection {
            original: Arc::new(row.clone()),
            row: row.clone(),
            revision: LockedRevision::Update,
        };
        selected.row.resolved_commit = None;
        selected.row.source_ref = Some(reference.clone());
        let manifest = self.locked_registry(&selected)?.fetch_manifest_at_ref(
            &row.group,
            row.name.as_str(),
            &reference,
        )?;
        let package = manifest.require_package()?;
        if package.group != row.group || package.name != row.name {
            return Err(refusal("held ref declares another coordinate"));
        }
        Ok(package.version.clone())
    }
    fn locked_registry(
        &self,
        selection: &LockedSelection,
    ) -> Result<GitPerPackageRegistry, RegistryError> {
        let row = &selection.row;
        let mut artifact =
            vibe_core::remap::Artifact::new(vibe_core::remap::ArtifactKind::GitSourceDep)
                .with_url(row.source_url.as_str())
                .with_package(format!("{}/{}@{}", row.group, row.name, row.version));
        if let Some(digest) = row.content_hash.as_str().strip_prefix("sha256:") {
            artifact = artifact.with_digest(digest);
        }
        let chosen = vibe_core::remap::active()
            .choose_operator_candidate(&artifact, row.registry.as_deref())
            .map_err(|error| RegistryError::MalformedMeta {
                path: "operator-remap".into(),
                reason: error.to_string(),
            })?;
        let physical = chosen.target.display();
        let publication = chosen.publication_scope();
        let auth = crate::resolve_registry_auth(crate::RegistryAuthInput::LockedSource {
            source_url: &physical,
            auth: None,
            token_env: None,
        })?;
        GitPerPackageRegistry::open_single_package_with_deferred_auth(
            "locked-source",
            &physical,
            selection.reference()?,
            &self.cache_root,
            Arc::clone(&self.backend),
            DEFAULT_FRESHNESS_SECS,
            auth.kind,
            auth.token_env_name.as_deref(),
        )
        .map(|registry| {
            registry
                .with_locked_source()
                .with_source_publication(publication)
        })
    }
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW"
    )]
    pub(super) fn resolve_locked(
        &self,
        pkg: &PackageRef,
        row: &LockedPackage,
    ) -> Result<MultiResolution, RegistryError> {
        let mut selection = LockedSelection {
            original: Arc::new(row.clone()),
            row: row.clone(),
            revision: LockedRevision::Replay,
        };
        if self.updates_locked(&format!("{}/{}", row.group, row.name)) {
            let (version, directory) = if let Some(directory) = self.locked_directory(row)? {
                let resolved = self.resolve_locked_directory(pkg, row, &directory)?;
                (resolved.version, Some(resolved.source_dir))
            } else if Self::updates_explicit_ref(row) {
                (self.held_ref_version(row)?, None)
            } else if row.source_kind == Some(SourceKind::Registry) {
                (
                    self.locked_registry(&selection)?.resolve(pkg)?.version,
                    None,
                )
            } else {
                return Err(refusal(
                    "legacy source family is ambiguous for version update; replay remains supported",
                ));
            };
            if version != row.version {
                selection.row.version = version;
                selection.row.resolved_commit = None;
                selection.revision = LockedRevision::Update;
                if let Some(directory) = directory {
                    if row.source_kind != Some(SourceKind::Path) {
                        selection.row.source_url = vibe_core::SourceUrl::new(
                            crate::local_registry::source_uri_for_local(&directory),
                        );
                    }
                } else if Self::updates_explicit_ref(row) {
                    selection.row.source_ref = Some(self.held_update_reference(row)?);
                } else {
                    selection.row.source_ref = Some(format!("v{}", selection.row.version));
                }
            }
        }
        if !version_matches(&pkg.version, &selection.row.version) {
            return Err(refusal("request is incompatible with the held version"));
        }
        if selection.revision == LockedRevision::Replay
            && self
                .mutable_locked
                .contains(&format!("{}/{}", row.group, row.name))
            && self.locked_directory(&selection.row)?.is_some()
        {
            selection.revision = LockedRevision::Update;
        }
        let row = &selection.row;
        let entry = self
            .store_root
            .as_ref()
            .and_then(|root| store::lookup_at(root, &row.group, row.name.as_str(), &row.version));
        let directory = self.locked_directory(row)?;
        let from_store =
            !self.forced_refetch && entry.is_some() && selection.revision == LockedRevision::Replay;
        let entry = entry.filter(|_| from_store);
        let source_dir = entry.or(directory).unwrap_or_default();
        Ok(MultiResolution {
            direct: None,
            locked: Some(selection.clone()),
            selected_route: None,
            resolved: ResolvedPackage {
                group: row.group.clone(),
                name: row.name.to_string(),
                version: row.version.clone(),
                source_dir,
            },
            registry_name: row.registry.clone(),
            source_url: row.source_url.to_string(),
            source_ref: row.source_ref.clone(),
            overridden: row.overridden,
            is_git_source: row.source_kind == Some(SourceKind::Git),
            is_path_source: row.source_kind == Some(SourceKind::Path),
            via_redirect: row.via_redirect.clone(),
            from_store,
            redirect_target_auth: vibe_core::manifest::AuthKind::None,
            redirect_target_token_env: None,
        })
    }
    pub(super) fn locked_versions(
        &self,
        row: &LockedPackage,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        if !self.updates_locked(&format!("{}/{}", row.group, row.name)) {
            return Ok(vec![row.version.clone()]);
        }
        if let Some(directory) = self.locked_directory(row)? {
            if let Some(registry) = self.locked_directory_registry(row, &directory)? {
                return registry.list_versions(&row.group, row.name.as_str());
            }
            let manifest = Manifest::read(directory.join(Manifest::FILENAME))?;
            return Ok(vec![manifest.require_package()?.version.clone()]);
        }
        if Self::updates_explicit_ref(row) {
            return Ok(vec![self.held_ref_version(row)?]);
        }
        if row.source_kind != Some(SourceKind::Registry) {
            return Err(refusal(
                "legacy source family is ambiguous for version update; replay remains supported",
            ));
        }
        let selection = LockedSelection {
            original: Arc::new(row.clone()),
            row: row.clone(),
            revision: LockedRevision::Replay,
        };
        self.locked_registry(&selection)?
            .list_versions(&row.group, row.name.as_str())
    }
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW"
    )]
    pub(super) fn fetch_locked(
        &self,
        resolution: &MultiResolution,
        selection: &LockedSelection,
        store_root: &Path,
        slot: Option<&Path>,
    ) -> Result<crate::PackageAcquisition, RegistryError> {
        if let Some(cached) = self.reuse_locked_checkout(resolution, selection, slot)? {
            return Ok(crate::PackageAcquisition::Cached(Box::new(cached)));
        }
        if resolution.from_store {
            let mut cached = self.fetch_from_store(resolution, selection.expected())?;
            selection.apply_evidence(&mut cached);
            return Ok(crate::PackageAcquisition::Cached(Box::new(cached)));
        }
        if let Some(directory) = self.locked_directory(&selection.row)? {
            let mut resolved = resolution.resolved.clone();
            resolved.source_dir = directory.clone();
            let hash = compute_content_hash(&directory)?;
            selection.verify(&hash, None)?;
            let registry = LocalRegistry::new(&directory)?;
            let mut cached = registry.fetch(&resolved, store_root)?;
            selection.verify(&cached.content_hash, None)?;
            selection.apply_evidence(&mut cached);
            return Ok(crate::PackageAcquisition::Cached(Box::new(cached)));
        }
        if self.offline {
            return Err(refusal(
                "locked bytes are absent from the verified offline store",
            ));
        }
        let registry = self.locked_registry(selection)?;
        let result = registry.fetch_locked_selection(
            &resolution.resolved,
            store_root,
            selection.expected(),
            slot,
        )?;
        match result {
            crate::PackageAcquisition::Cached(mut cached) => {
                selection.verify(&cached.content_hash, cached.resolved_commit.as_deref())?;
                selection.apply_evidence(&mut cached);
                Ok(crate::PackageAcquisition::Cached(cached))
            }
            crate::PackageAcquisition::InPlace(prepared) => {
                selection.verify(prepared.content_hash(), Some(prepared.resolved_commit()))?;
                Ok(crate::PackageAcquisition::InPlace(
                    prepared.with_locked_evidence(selection)?,
                ))
            }
        }
    }
    pub(super) fn prepare_locked(
        &self,
        resolution: &MultiResolution,
        selection: &LockedSelection,
        slot: &Path,
    ) -> Result<crate::PreparedInPlace, RegistryError> {
        if self.offline {
            return Err(refusal(
                "offline in-place recovery requires retained Git objects",
            ));
        }
        let registry = self.locked_registry(selection)?;
        // Plain replay preserves local modifications; an explicit version update
        // resets the held slot exactly as an ordinary in-place update does.
        let preserve = selection.revision == LockedRevision::Replay;
        let prepared = registry.prepare_locked_selection(&resolution.resolved, slot, preserve)?;
        selection.verify(prepared.content_hash(), Some(prepared.resolved_commit()))?;
        prepared.with_locked_evidence(selection)
    }
}

mod directory;
mod explicit;
pub use explicit::LockedUpdateAvailability;
mod reference;
