//! Held monorepo rows (PROP-002 `#registry-type`): a registry row whose
//! `source_url` is `<repository>#<group>/<name>/v<version>` replays by
//! reading that directory of the repository at the held commit — never the
//! registry catalog, never the current branch tip (PROP-073
//! `#LOCKED-SOURCE-LAW`). A version update reads the held branch.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW");
use super::*;
use crate::git_registry::{monorepo_package_path, monorepo_source_url, parse_monorepo_source_url};

/// The repository of a held monorepo row, or `None` for any other row.
pub(in crate::multi_registry_resolver) fn monorepo_repository(row: &LockedPackage) -> Option<&str> {
    if row.source_kind != Some(SourceKind::Registry)
        || row.source_ref.is_none()
        || row.overridden
        || row.via_redirect.is_some()
    {
        return None;
    }
    let (repository, path) = parse_monorepo_source_url(row.source_url.as_str())?;
    let prefix = format!("{}/{}/v", row.group, row.name);
    path.starts_with(&prefix).then_some(repository)
}

impl MultiRegistryResolver {
    /// The held repository at `reference` (the held commit for replay, the
    /// held branch for a version update), in the locked-replay cache — never
    /// the registry walk's clone, so replay cannot move the walk's checkout.
    /// Under a controlled transport it is a native, operation-owned
    /// snapshot. Remap rows choose the physical repository first.
    fn monorepo_checkout(
        &self,
        row: &LockedPackage,
        repository: &str,
        reference: &str,
    ) -> Result<GitMonorepoRegistry, RegistryError> {
        if self.offline {
            return Err(refusal(
                "locked monorepo bytes are absent from the verified offline store",
            ));
        }
        let mut artifact =
            vibe_core::remap::Artifact::new(vibe_core::remap::ArtifactKind::GitSourceDep)
                .with_url(repository)
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
        let cache = self.cache_root.join("monorepo-locked");
        if self.backend.native_materialisation_for(&physical)? {
            return GitMonorepoRegistry::open_native(
                &physical,
                &[],
                reference,
                &cache,
                Arc::clone(&self.backend),
            );
        }
        // Freshness 0: an existing clone is always brought to `reference`.
        GitMonorepoRegistry::open_with(&physical, reference, &cache, Arc::clone(&self.backend), 0)
    }

    /// The held branch's versions of this coordinate.
    pub(super) fn monorepo_versions(
        &self,
        row: &LockedPackage,
        repository: &str,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        let branch = row
            .source_ref
            .as_deref()
            .ok_or_else(|| refusal("monorepo row has no held branch"))?;
        self.monorepo_checkout(row, repository, branch)?
            .local()
            .list_versions(&row.group, row.name.as_str())
    }

    /// Resolve a version update on the held branch.
    pub(super) fn monorepo_resolve(
        &self,
        pkg: &PackageRef,
        row: &LockedPackage,
        repository: &str,
    ) -> Result<semver::Version, RegistryError> {
        let branch = row
            .source_ref
            .as_deref()
            .ok_or_else(|| refusal("monorepo row has no held branch"))?;
        let checkout = self.monorepo_checkout(row, repository, branch)?;
        Ok(checkout.local().resolve(pkg)?.version)
    }

    /// Point an updated selection at the new version's directory.
    pub(super) fn retarget_monorepo_row(row: &mut LockedPackage, repository: &str) {
        row.source_url = vibe_core::SourceUrl::new(monorepo_source_url(
            repository,
            &row.group,
            row.name.as_str(),
            &row.version,
        ));
    }

    /// Acquire a held monorepo row: the package directory at the held
    /// commit (replay) or at the held branch (update), verified against the
    /// held content hash and commit.
    pub(super) fn fetch_locked_monorepo(
        &self,
        resolution: &MultiResolution,
        selection: &LockedSelection,
        store_root: &Path,
        repository: &str,
    ) -> Result<CachedPackage, RegistryError> {
        let row = &selection.row;
        let reference = selection.reference()?;
        let checkout = self.monorepo_checkout(row, repository, reference)?;
        let commit = checkout.head_commit()?;
        let directory = checkout.clone_dir().join(monorepo_package_path(
            &row.group,
            row.name.as_str(),
            &row.version,
        ));
        if !directory.join(Manifest::FILENAME).is_file() {
            return Err(refusal(
                "the held package directory is absent from the held monorepo commit",
            ));
        }
        let mut resolved = resolution.resolved.clone();
        resolved.source_dir = directory;
        let mut cached = checkout.local().fetch(&resolved, store_root)?;
        cached.acquisition = Some(vibe_wire::generated::shared::SourceAcquisition::GitRepository);
        cached.resolved_commit = commit.clone();
        // The clone path is an implementation detail; the evidence names
        // the held repository, and a remapped clone url beside it.
        cached.source_uri = row.source_url.to_string();
        cached.fetched_from =
            (checkout.served_url() != repository).then(|| checkout.served_url().to_string());
        selection.verify(&cached.content_hash, commit.as_deref())?;
        selection.apply_evidence(&mut cached);
        Ok(cached)
    }
}
