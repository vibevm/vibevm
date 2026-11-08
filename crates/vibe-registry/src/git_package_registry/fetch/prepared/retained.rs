//! Retain native single-package objects before the final destination is known.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION");
use super::*;

pub(crate) struct RetainedNativePackage {
    temporary: tempfile::TempDir,
    objects: PathBuf,
    token: crate::git_backend::shell::native_admission::NativeSource,
    manifest: Arc<Manifest>,
    origin: String,
    requested: String,
    source: PathBuf,
    serving: Option<String>,
}
impl RetainedNativePackage {
    pub(crate) fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub(crate) fn commit(&self) -> &str {
        &self.token.revision().commit
    }
    pub(crate) fn reference(&self) -> &str {
        &self.token.revision().reference
    }
    pub(crate) fn serving_endpoint(&self) -> Option<&str> {
        self.serving.as_deref()
    }
    pub(crate) fn bind(
        self,
        registry: &GitPerPackageRegistry,
        resolved: &ResolvedPackage,
        target: &Path,
    ) -> Result<PreparedInPlace, RegistryError> {
        let slot = std::path::absolute(target).map_err(|e| io(target, e))?;
        // Existing installed repositories are replayed through held source
        // authority. A new source lease cannot invent their warm context.
        if slot.join(".git").exists() && slot != self.source {
            return Err(refusal(
                &slot,
                "new direct source has no captured installed-checkout context; use its held source selection",
            ));
        }
        let meta = self.manifest.require_package()?;
        if meta.group != resolved.group
            || meta.name != resolved.name
            || meta.version != resolved.version
        {
            return Err(refusal(&slot, "retained direct package identity changed"));
        }
        let repository = slot.join(".git").exists();
        let before = Before {
            present: slot.exists(),
            repository,
            head: if repository {
                registry.backend.head_commit(&slot)?
            } else {
                None
            },
            origin: if repository {
                observed_origin(registry.backend.as_ref(), &slot)?
            } else {
                None
            },
        };
        let revision = self.token.bind(&slot)?;
        Ok(PreparedInPlace(Arc::new(Prepared {
            registry_name: registry.name.clone(),
            locked_selection: None,
            route_publication: registry.source_publication,
            direct_kind: None,
            via_redirect: None,
            _temporary: self.temporary,
            objects: self.objects,
            slot,
            backend: registry.backend.clone(),
            before,
            refresh: repository,
            origin: self.origin,
            serving_endpoint: self.serving,
            canonical: registry.package_repo_url(&resolved.group, &resolved.name)?,
            authenticated_origin: None,
            content_hash: super::super::commit_content_hash(&revision.commit),
            requested: self.requested,
            revision,
            manifest: (*self.manifest).clone(),
        })))
    }
}
impl GitPerPackageRegistry {
    pub(crate) fn retain_native_package(
        &self,
        group: &Group,
        name: &str,
        url: &str,
        reference: &str,
    ) -> Result<RetainedNativePackage, RegistryError> {
        self.backend.preflight_selected_address(url)?;
        let source = std::path::absolute(self.package_clone_dir(group, name))
            .map_err(|e| io(&self.cache_root, e))?;
        let reuse = source.join(".git").exists().then_some(source.as_path());
        let cache = std::path::absolute(&self.cache_root).map_err(|e| io(&self.cache_root, e))?;
        fs::create_dir_all(&cache).map_err(|e| io(&cache, e))?;
        let temporary = tempfile::Builder::new()
            .prefix("retained-direct-")
            .tempdir_in(&cache)
            .map_err(|e| io(&cache, e))?;
        let objects = temporary.path().join("objects.git");
        // The selected URL is authority, while a warm cache supplies captured
        // config/attributes/ref metadata only. Its origin cannot retarget contact.
        let origin = strip_git_plus_prefix(url).to_owned();
        let authenticated = self.selected_credentialed_url(&origin)?;
        let token = self
            .backend
            .prepare_pinned_native_source(&authenticated, reference, reuse, &objects)?
            .ok_or_else(|| refusal(&source, "controlled direct source capability disappeared"))?;
        let bytes = self
            .backend
            .file_at_commit(&objects, &token.revision().commit, Manifest::FILENAME)
            .map_err(|source| GitError::PreparedValidation {
                source: Box::new(source),
            })?;
        let manifest = Manifest::parse_str(
            std::str::from_utf8(&bytes)
                .map_err(|_| refusal(&source, "retained manifest is not UTF-8"))?,
        )?;
        let meta = manifest.require_package()?;
        if &meta.group != group || meta.name.as_str() != name {
            return Err(refusal(
                &source,
                "direct source declares a different coordinate",
            ));
        }
        let serving = self.backend.prepared_serving_endpoint(&objects)?;
        Ok(RetainedNativePackage {
            temporary,
            objects,
            token,
            manifest: Arc::new(manifest),
            origin,
            requested: reference.into(),
            source,
            serving,
        })
    }
    pub(crate) fn consume_retained_native(
        &self,
        source: RetainedNativePackage,
        resolved: &ResolvedPackage,
        store_root: &Path,
        expected: Option<&str>,
        slot: Option<&Path>,
    ) -> Result<PackageAcquisition, RegistryError> {
        let inplace = source
            .manifest()
            .require_package()?
            .materialization
            .is_in_place();
        let cache = self.package_clone_dir(&resolved.group, &resolved.name);
        let destination = if inplace {
            slot.unwrap_or(&cache)
        } else {
            &cache
        };
        let prepared = source.bind(self, resolved, destination)?;
        if inplace && slot.is_some() {
            if expected.is_some_and(|hash| hash != prepared.content_hash()) {
                return Err(refusal(
                    destination,
                    "direct commit differs from expected identity",
                ));
            }
            return Ok(PackageAcquisition::InPlace(prepared));
        }
        let stage = prepared
            .stage_native()?
            .ok_or_else(|| refusal(destination, "controlled direct stage unavailable"))?;
        let hash = if inplace {
            prepared.content_hash().to_owned()
        } else {
            compute_content_hash(stage.staged_path())?
        };
        if expected.is_some_and(|expected| expected != hash) {
            return Err(refusal(
                destination,
                "direct content differs from expected identity",
            ));
        }
        let (cache_dir, reused) = if inplace {
            self.publish_native_cache(&stage, &cache)?;
            (cache, false)
        } else {
            let inserted = store::insert_current_at(
                store_root,
                stage.staged_path(),
                &resolved.group,
                &resolved.name,
                &resolved.version,
                &hash,
            )?;
            let reused = matches!(&inserted, store::InsertOutcome::AlreadyPresent(_));
            let entry = inserted.into_entry();
            super::super::verify_store_entry_against_pin(
                &entry,
                &hash,
                &resolved.group,
                &resolved.name,
                &resolved.version,
            )?;
            self.publish_native_cache(&stage, &cache)?;
            (entry, reused)
        };
        Ok(PackageAcquisition::Cached(Box::new(CachedPackage {
            locked_selection: None,
            route_publication: vibe_core::RoutePublication::Local,
            recorded_source_kind: None,
            store_reused: Some(reused),
            acquisition: Some(vibe_wire::generated::shared::SourceAcquisition::GitRepository),
            resolved: resolved.clone(),
            cache_dir,
            manifest: prepared.manifest().clone(),
            content_hash: hash,
            source_uri: prepared.source_uri().into(),
            fetched_from: prepared.source_evidence().fetched_from,
            registry_name: Some(self.name.clone()),
            source_ref: Some(prepared.source_ref().into()),
            resolved_commit: Some(prepared.resolved_commit().into()),
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_local: false,
            is_embedded: false,
            via_redirect: None,
        })))
    }
}
