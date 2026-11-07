//! One ephemeral source-bound candidate for pre-effect in-place admission.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION");
use super::*;
use crate::git_backend::GitPreparedRevision;

#[derive(Clone)]
struct Before {
    present: bool,
    repository: bool,
    head: Option<String>,
    origin: Option<String>,
}
struct Prepared {
    registry_name: String,
    _temporary: tempfile::TempDir,
    objects: PathBuf,
    slot: PathBuf,
    backend: Arc<dyn GitBackend>,
    before: Before,
    refresh: bool,
    origin: String,
    canonical: String,
    authenticated_origin: Option<String>,
    content_hash: String,
    requested: String,
    revision: GitPreparedRevision,
    manifest: Manifest,
}

/// An admitted candidate's immutable source objects and manifest, bound to one
/// destination. Clone shares an operation-local lifetime, never a durable store.
/// An existing checkout supplies borrowed base objects until apply: replacing
/// that checkout invalidates the candidate. Successful results own their objects
/// independently of this value; dropping the last clone removes only the cache
/// preparation, not the installed checkout.
///
/// ```
/// use vibe_registry::PreparedInPlace;
/// fn inspect(candidate: &PreparedInPlace) {
///     let _definition = candidate.manifest();
///     assert!(!candidate.resolved_commit().is_empty());
/// }
/// ```
#[derive(Clone)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub struct PreparedInPlace(Arc<Prepared>);

/// One acquired package; in-place bytes remain retained until workspace apply.
#[derive(Debug)]
pub enum PackageAcquisition {
    Cached(Box<CachedPackage>),
    InPlace(PreparedInPlace),
}
impl std::fmt::Debug for PreparedInPlace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedInPlace")
            .field("slot", &self.0.slot)
            .field("commit", &self.0.revision.commit)
            .finish_non_exhaustive()
    }
}
fn refusal(path: &Path, reason: &str) -> RegistryError {
    RegistryError::PreparedInPlace {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}
fn io(path: &Path, source: std::io::Error) -> RegistryError {
    RegistryError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn observed_origin(backend: &dyn GitBackend, slot: &Path) -> Result<Option<String>, RegistryError> {
    match backend.origin_url(slot) {
        Ok(origin) => Ok(Some(origin)),
        Err(GitError::OriginMissing { .. }) => Ok(None),
        Err(error) => Err(error.into()),
    }
}
impl PreparedInPlace {
    /// Credential-redacted facts of this held registry preparation, without
    /// querying the source again or applying its objects to the destination.
    ///
    /// ```
    /// # fn inspect(candidate: &vibe_registry::PreparedInPlace) {
    /// let evidence = candidate.source_evidence();
    /// assert_eq!(evidence.resolved_commit.as_deref(), Some(candidate.resolved_commit()));
    /// assert_eq!(evidence.store_reused, Some(false));
    /// # }
    /// ```
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-EVIDENCE-PROJECT-EVENTS"
    )]
    pub fn source_evidence(&self) -> vibe_wire::generated::shared::SourceEvidence {
        use vibe_core::progress::transfer::safe_remote;
        use vibe_wire::generated::shared::{
            SourceAcquisition, SourceEvidence, SourceSelectionKind,
        };
        let canonical = safe_remote(&self.0.canonical);
        let actual = safe_remote(&self.0.origin);
        SourceEvidence {
            registry: Some(self.0.registry_name.clone()),
            fetched_from: (actual != canonical).then_some(actual),
            source_url: Some(canonical),
            source_ref: Some(self.0.requested.clone()),
            resolved_commit: Some(self.0.revision.commit.clone()),
            source_kind: Some(SourceSelectionKind::Registry),
            acquisition: Some(SourceAcquisition::GitRepository),
            store_reused: Some(false),
            via_redirect: None,
        }
    }
    /// Actual prospective manifest read from the retained commit object.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION"
    )]
    pub fn manifest(&self) -> &Manifest {
        &self.0.manifest
    }
    /// Full peeled commit identity; no missing-HEAD placeholder is admitted.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION"
    )]
    pub fn resolved_commit(&self) -> &str {
        &self.0.revision.commit
    }
    /// Canonical registry source for the incumbent lockfile vocabulary. The
    /// actual refresh origin or mirror is privately retained for application.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION"
    )]
    pub fn source_uri(&self) -> &str {
        &self.0.canonical
    }
    /// Requested version ref; its resolved namespace/object is retained too.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION"
    )]
    pub fn source_ref(&self) -> &str {
        &self.0.requested
    }
    /// The existing commit-derived in-place content hash.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION"
    )]
    pub fn content_hash(&self) -> &str {
        &self.0.content_hash
    }
    /// Bound absolute destination, without reinterpreting symlink aliases.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION"
    )]
    pub fn slot(&self) -> &Path {
        &self.0.slot
    }

    /// Stage native bytes before any caller metadata publication. The returned
    /// directory may only be published through a caller's checked undo owner.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
    )]
    pub fn stage_native(
        &self,
    ) -> Result<
        Option<crate::git_backend::shell::native_admission::NativeMaterialisation>,
        RegistryError,
    > {
        let p = &self.0;
        let repository = p.slot.join(".git").exists();
        if p.slot.exists() != p.before.present || repository != p.before.repository {
            return Err(refusal(
                &p.slot,
                "destination identity changed before staging",
            ));
        }
        if repository
            && (p.backend.head_commit(&p.slot)? != p.before.head
                || observed_origin(p.backend.as_ref(), &p.slot)? != p.before.origin)
        {
            return Err(refusal(
                &p.slot,
                "destination HEAD or origin changed before staging",
            ));
        }
        let staged = self
            .0
            .backend
            .stage_native_materialisation(&self.0.objects, &self.0.revision)?;
        if let Some(staged) = &staged {
            if Manifest::read(staged.staged_path().join(Manifest::FILENAME))? != self.0.manifest {
                return Err(refusal(
                    &self.0.slot,
                    "staged manifest differs from admitted source",
                ));
            }
        } else {
            self.0.backend.preflight_materialisation()?;
        }
        Ok(staged)
    }

    /// Apply the captured objects without resolving a source or mutable ref.
    /// HEAD/origin replacement refuses before checkout; tracked/untracked/ignored
    /// dirt is repaired by the incumbent clean owner only after admission.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION"
    )]
    pub fn materialise(&self) -> Result<InPlaceMaterialised, RegistryError> {
        let p = &self.0;
        p.backend.preflight_materialisation()?;
        let repository = p.slot.join(".git").exists();
        if p.slot.exists() != p.before.present || repository != p.before.repository {
            return Err(refusal(
                &p.slot,
                "destination existence or repository identity changed since preparation",
            ));
        }
        if repository
            && (p.backend.head_commit(&p.slot)? != p.before.head
                || observed_origin(p.backend.as_ref(), &p.slot)? != p.before.origin)
        {
            return Err(refusal(
                &p.slot,
                "checkout HEAD or origin changed since preparation",
            ));
        }
        let dirty = repository && p.backend.working_tree_dirty(&p.slot)?;
        if p.refresh {
            p.backend.apply_prepared_ref(
                &p.objects,
                &p.revision,
                &p.origin,
                p.authenticated_origin.as_deref(),
                &p.slot,
                true,
            )?;
            p.backend.clean_worktree(&p.slot)?;
            p.verify(&p.slot)?;
        } else {
            let parent = p
                .slot
                .parent()
                .ok_or_else(|| refusal(&p.slot, "destination has no parent"))?;
            fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
            let temporary = tempfile::Builder::new()
                .prefix(".inplace-apply-")
                .tempdir_in(parent)
                .map_err(|e| io(parent, e))?;
            let candidate = temporary.path().join("checkout");
            p.backend.apply_prepared_ref(
                &p.objects,
                &p.revision,
                &p.origin,
                p.authenticated_origin.as_deref(),
                &candidate,
                false,
            )?;
            p.backend.clean_worktree(&candidate)?;
            p.verify(&candidate)?;
            if p.slot.exists() {
                fs::remove_dir_all(&p.slot).map_err(|e| io(&p.slot, e))?;
            }
            fs::rename(&candidate, &p.slot).map_err(|e| io(&p.slot, e))?;
        }
        Ok(InPlaceMaterialised {
            source_uri: p.canonical.clone(),
            source_ref: p.requested.clone(),
            resolved_commit: Some(p.revision.commit.clone()),
            content_hash: p.content_hash.clone(),
            changed: !repository || dirty || p.before.head.as_ref() != Some(&p.revision.commit),
            manifest: p.manifest.clone(),
        })
    }
}
impl Prepared {
    fn verify(&self, path: &Path) -> Result<(), RegistryError> {
        if self.backend.head_commit(path)?.as_ref() != Some(&self.revision.commit) {
            return Err(refusal(
                path,
                "applied checkout does not identify the admitted commit",
            ));
        }
        if Manifest::read(path.join(Manifest::FILENAME))? != self.manifest {
            return Err(refusal(
                path,
                "applied manifest differs from the admitted source object",
            ));
        }
        Ok(())
    }
}
impl GitPerPackageRegistry {
    fn native_credentialed_url(
        &self,
        origin: &str,
        reuse: Option<&Path>,
        primary: &str,
        mirrors: &[String],
    ) -> Result<String, RegistryError> {
        if self.effective_token.is_some()
            && let Some(repo) = reuse
        {
            let recorded = self.backend.recorded_origin_url(repo)?;
            let parse = |value: &str| {
                reqwest::Url::parse(strip_git_plus_prefix(value)).map(|url| url.origin())
            };
            let recorded_origin = parse(&recorded).ok();
            let permitted = recorded_origin.is_some()
                && parse(origin).ok() == recorded_origin
                && std::iter::once(primary)
                    .chain(mirrors.iter().map(String::as_str))
                    .any(|url| parse(url).ok() == recorded_origin);
            if !permitted {
                return Err(GitError::NativeContext{reason:"warm token authentication cannot cross the configured credential origin"}.into());
            }
        }
        Ok(self.credentialed_url(origin))
    }
    pub(super) fn prepare_native_attempt(
        &self,
        resolved: &ResolvedPackage,
        url: &str,
        source: &Path,
        refresh: bool,
        workspace_slot: Option<&Path>,
    ) -> Result<PreparedInPlace, PreparationFailure> {
        let acquired = (|| {
            self.ensure_token_loaded()?;
            let source = std::path::absolute(source).map_err(|e| io(source, e))?;
            let reuse = (refresh && source.join(".git").exists()).then_some(source.as_path());
            let origin = match reuse {
                Some(source) => self.backend.origin_url(source)?,
                None => strip_git_plus_prefix(url).to_owned(),
            };
            let (primary, mirrors) = self.package_urls(&resolved.group, &resolved.name)?;
            let authenticated = self.native_credentialed_url(&origin, reuse, &primary, &mirrors)?;
            let cache =
                std::path::absolute(&self.cache_root).map_err(|e| io(&self.cache_root, e))?;
            fs::create_dir_all(&cache).map_err(|e| io(&cache, e))?;
            let temporary = tempfile::Builder::new()
                .prefix("prepared-native-")
                .tempdir_in(&cache)
                .map_err(|e| io(&cache, e))?;
            let objects = temporary.path().join("objects.git");
            let requested = format!("v{}", resolved.version);
            let token = self
                .backend
                .prepare_native_source(&authenticated, &requested, reuse, &objects)?
                .ok_or_else(|| refusal(&source, "native source capability disappeared"))?;
            Ok::<_, RegistryError>((temporary, objects, token, origin, requested))
        })()
        .map_err(PreparationFailure::acquisition)?;
        let (temporary, objects, token, origin, requested) = acquired;
        (|| {
            let bytes = self.backend.file_at_commit(
                &objects,
                &token.revision().commit,
                Manifest::FILENAME,
            )?;
            let manifest = Manifest::parse_str(
                std::str::from_utf8(&bytes)
                    .map_err(|_| refusal(source, "prepared manifest is not UTF-8"))?,
            )?;
            let package = manifest
                .package
                .as_ref()
                .ok_or_else(|| refusal(source, "prepared manifest has no package identity"))?;
            if package.group != resolved.group
                || package.name != resolved.name
                || package.version != resolved.version
            {
                return Err(refusal(
                    source,
                    "acquired package identity differs from resolved coordinate",
                ));
            }
            let destination = if package.materialization.is_in_place() {
                workspace_slot.unwrap_or(source)
            } else {
                source
            };
            let slot = std::path::absolute(destination).map_err(|e| io(destination, e))?;
            let repository = slot.join(".git").exists();
            let before = Before {
                present: slot.exists(),
                repository,
                head: if repository {
                    self.backend.head_commit(&slot)?
                } else {
                    None
                },
                origin: if repository {
                    observed_origin(self.backend.as_ref(), &slot)?
                } else {
                    None
                },
            };
            let revision = token.bind(&slot)?;
            Ok(PreparedInPlace(Arc::new(Prepared {
                registry_name: self.name.clone(),
                _temporary: temporary,
                objects,
                slot,
                backend: self.backend.clone(),
                before,
                refresh,
                origin,
                canonical: self.package_repo_url(&resolved.group, &resolved.name)?,
                authenticated_origin: None,
                content_hash: super::commit_content_hash(&revision.commit),
                requested,
                revision,
                manifest,
            })))
        })()
        .map_err(PreparationFailure::Admission)
    }
    /// Prepare the actual existing origin, or a fresh primary/mirror source,
    /// without changing project semantics. Existing-origin acquisition writes
    /// only Git objects and FETCH_HEAD in the original repository, preserving
    /// its native transport context. The caller must admit `manifest()` before
    /// calling the bound candidate's `materialise` method.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION"
    )]
    pub fn prepare_in_place(
        &self,
        resolved: &ResolvedPackage,
        slot: &Path,
    ) -> Result<PreparedInPlace, RegistryError> {
        self.ensure_token_loaded()?;
        let slot = std::path::absolute(slot).map_err(|e| io(slot, e))?;
        let repository = slot.join(".git").exists();
        let head = if repository {
            self.backend.head_commit(&slot)?
        } else {
            None
        };
        let (primary, mirrors) = self.package_urls(&resolved.group, &resolved.name)?;
        let requested = format!("v{}", resolved.version);
        let known_origin = if repository {
            observed_origin(self.backend.as_ref(), &slot)?
        } else {
            None
        };
        let current = if repository {
            known_origin
                .clone()
                .ok_or_else(|| GitError::OriginMissing { path: slot.clone() })
        } else {
            Ok(strip_git_plus_prefix(&primary).to_owned())
        };
        let before = Before {
            present: slot.exists(),
            repository,
            head,
            origin: known_origin,
        };
        let attempt = |origin: &str,
                       refresh: bool,
                       authenticate: bool|
         -> Result<PreparedInPlace, PreparationFailure> {
            let (temporary, objects, url, revision) = (|| {
                let cache_root =
                    std::path::absolute(&self.cache_root).map_err(|e| io(&self.cache_root, e))?;
                fs::create_dir_all(&cache_root).map_err(|e| io(&cache_root, e))?;
                let temporary = tempfile::Builder::new()
                    .prefix("prepared-inplace-")
                    .tempdir_in(&cache_root)
                    .map_err(|e| io(&cache_root, e))?;
                let objects = temporary.path().join("objects.git");
                let url = if self.backend.uses_native_materialisation() {
                    self.native_credentialed_url(
                        origin,
                        refresh.then_some(slot.as_path()),
                        &primary,
                        &mirrors,
                    )?
                } else if authenticate {
                    self.credentialed_url(origin)
                } else {
                    origin.to_owned()
                };
                let revision = self.backend.prepare_ref(
                    &url,
                    &requested,
                    refresh.then_some(slot.as_path()),
                    &objects,
                    &slot,
                )?;
                Ok::<_, RegistryError>((temporary, objects, url, revision))
            })()
            .map_err(PreparationFailure::acquisition)?;
            (|| {
                let recorded = self.backend.recorded_origin_url(&objects)?;
                let applied_origin = if refresh || url != origin {
                    // For an injected HTTPS credential, retain the caller's plain
                    // source, never the authenticated spelling Git temporarily wrote.
                    origin.to_owned()
                } else {
                    recorded
                };
                // Bare cloning may temporarily record the credentialed bootstrap
                // origin; the held preparation exposes/stores only the plain form.
                self.backend
                    .set_remote_url(&objects, "origin", &applied_origin)?;
                let bytes =
                    self.backend
                        .file_at_commit(&objects, &revision.commit, Manifest::FILENAME)?;
                let text = std::str::from_utf8(&bytes)
                    .map_err(|_| refusal(&slot, "prepared manifest is not UTF-8"))?;
                let manifest = Manifest::parse_str(text)?;
                let package = manifest
                    .package
                    .as_ref()
                    .ok_or_else(|| refusal(&slot, "prepared manifest has no package identity"))?;
                if package.group != resolved.group
                    || package.name != resolved.name
                    || package.version != resolved.version
                {
                    return Err(refusal(
                        &slot,
                        "actual source package identity differs from the resolved coordinate",
                    ));
                }
                Ok(PreparedInPlace(Arc::new(Prepared {
                    registry_name: self.name.clone(),
                    _temporary: temporary,
                    objects,
                    slot: slot.clone(),
                    backend: self.backend.clone(),
                    before: before.clone(),
                    refresh,
                    origin: applied_origin,
                    canonical: primary.clone(),
                    authenticated_origin: (url != origin).then_some(url),
                    content_hash: super::commit_content_hash(&revision.commit),
                    requested: requested.clone(),
                    revision,
                    manifest,
                })))
            })()
            .map_err(PreparationFailure::Admission)
        };
        let primary_failure = match current {
            Ok(origin) => match attempt(&origin, repository, !repository) {
                Ok(value) => return Ok(value),
                Err(error) => error,
            },
            Err(error) => PreparationFailure::acquisition(error.into()),
        };
        let primary_error = match primary_failure {
            PreparationFailure::Acquisition(error) => error,
            PreparationFailure::Admission(error) => return Err(error),
        };
        for mirror in &mirrors {
            match attempt(strip_git_plus_prefix(mirror), false, true) {
                Ok(value) => return Ok(value),
                Err(PreparationFailure::Admission(error)) => return Err(error),
                Err(PreparationFailure::Acquisition(_)) => {}
            }
        }
        Err(primary_error)
    }
}

// Preserve the original RegistryError while making the source-selection phase
// explicit. Only acquisition failure can advance to another source. All reads
// after a returned revision belong to that acquired source's admission.
pub(super) enum PreparationFailure {
    Acquisition(RegistryError),
    Admission(RegistryError),
}
impl PreparationFailure {
    fn acquisition(error: RegistryError) -> Self {
        if matches!(&error, RegistryError::Git(error) if !matches!(
            error,
            GitError::ContactRefused(_)
                | GitError::NativeUnavailable { .. }
                | GitError::NativeContext { .. }
                | GitError::PreparedCheckout { .. }
                | GitError::PreparedCheckoutUnsupported
                | GitError::PreparedValidation { .. }
        )) {
            Self::Acquisition(error)
        } else {
            Self::Admission(error)
        }
    }
}

#[cfg(test)]
mod tests;
