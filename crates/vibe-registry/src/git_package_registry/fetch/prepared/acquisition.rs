//! Acquire native objects before binding the actual package materialisation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION");
use super::*;
impl GitPerPackageRegistry {
    pub(crate) fn prepare_declared_native_target(
        &self,
        resolved: &ResolvedPackage,
        slot: &Path,
    ) -> Result<PreparedInPlace, RegistryError> {
        let target = self.package_repo_url(&resolved.group, &resolved.name)?;
        let refresh = slot.join(".git").exists()
            && self.backend.recorded_origin_url(slot)? == strip_git_plus_prefix(&target);
        self.prepare_native_attempt(resolved, &target, slot, refresh, Some(slot))
            .map_err(|error| match error {
                PreparationFailure::Acquisition(error) | PreparationFailure::Admission(error) => {
                    error
                }
            })
    }
    pub(super) fn native_credentialed_url(
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
    pub(in crate::git_package_registry::fetch) fn prepare_native_attempt(
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
            let requested = self.package_reference(resolved);
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
                via_redirect: None,
                serving_endpoint: self.backend.prepared_serving_endpoint(&objects)?,
                _temporary: temporary,
                objects,
                slot,
                backend: self.backend.clone(),
                before,
                refresh,
                origin,
                canonical: self.package_repo_url(&resolved.group, &resolved.name)?,
                authenticated_origin: None,
                content_hash: super::super::commit_content_hash(&revision.commit),
                requested,
                revision,
                manifest,
            })))
        })()
        .map_err(PreparationFailure::Admission)
    }
}
