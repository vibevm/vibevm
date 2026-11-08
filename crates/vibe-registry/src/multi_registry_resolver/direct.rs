//! A direct source's selected route and objects survive metadata selection.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES");
use super::*;
use std::sync::Mutex;
use vibe_core::manifest::SourceKind;
use vibe_wire::generated::shared::{SourceEvidence, SourceSelectionKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirectKind {
    Git,
    Override,
}
impl DirectKind {
    fn source_kind(self) -> SourceKind {
        match self {
            Self::Git => SourceKind::Git,
            Self::Override => SourceKind::Override,
        }
    }
}
pub(super) struct DirectSourceInput<'a> {
    pub group: &'a Group,
    pub name: &'a str,
    pub canonical: &'a str,
    pub physical: &'a str,
    pub publication: vibe_core::RoutePublication,
    pub reference: &'a str,
    pub execution_reference: &'a str,
    pub kind: DirectKind,
    pub auth: vibe_core::manifest::AuthKind,
    pub token_env: Option<&'a str>,
}
/// Runtime-only retained direct-source owner; fields cannot be retargeted.
/// ```
/// # fn inspect(resolution: &vibe_registry::MultiResolution) {
/// let _retained = resolution.direct.is_some();
/// # }
/// ```
pub struct DirectSourceLease {
    registry: Arc<GitPerPackageRegistry>,
    canonical: String,
    physical: String,
    reference: String,
    kind: DirectKind,
    publication: vibe_core::RoutePublication,
    manifest: Manifest,
    commit: Option<String>,
    serving: Option<String>,
    native: Option<Mutex<Option<crate::git_package_registry::RetainedNativePackage>>>,
}
impl std::fmt::Debug for DirectSourceLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectSourceLease")
            .field("kind", &self.kind)
            .field("commit", &self.commit)
            .finish_non_exhaustive()
    }
}
impl DirectSourceLease {
    pub(super) fn reference(&self) -> &str {
        &self.reference
    }
    pub(crate) fn publication_scope(&self) -> vibe_core::RoutePublication {
        self.publication
    }
    pub(crate) fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub(crate) fn evidence(&self) -> SourceEvidence {
        let canonical = vibe_core::progress::transfer::safe_remote(&self.canonical);
        SourceEvidence {
            registry: None,
            source_url: Some(canonical.clone()),
            source_ref: Some(self.reference.clone()),
            resolved_commit: self.commit.clone(),
            source_kind: Some(match self.kind {
                DirectKind::Git => SourceSelectionKind::Git,
                DirectKind::Override => SourceSelectionKind::Override,
            }),
            via_redirect: None,
            fetched_from: self
                .serving
                .as_deref()
                .map(vibe_core::progress::transfer::safe_remote)
                .filter(|url| url != &canonical),
            acquisition: None,
            store_reused: None,
        }
    }
    pub(super) fn registry(&self) -> &GitPerPackageRegistry {
        &self.registry
    }
    fn validate(&self, resolution: &MultiResolution) -> Result<(), RegistryError> {
        let meta = self.manifest.require_package()?;
        if resolution.locked.is_some()
            || resolution.from_store
            || resolution.source_url != self.canonical
            || resolution.source_ref.as_deref() != Some(self.reference.as_str())
            || resolution.resolved.group != meta.group
            || resolution.resolved.name != meta.name
            || resolution.resolved.version != meta.version
            || resolution.overridden != (self.kind == DirectKind::Override)
            || resolution.is_git_source != (self.kind == DirectKind::Git)
        {
            return Err(direct_refusal(
                "direct source resolution changed after selection",
            ));
        }
        Ok(())
    }
    fn project_cached(&self, cached: &mut CachedPackage) {
        cached.route_publication = self.publication_scope();
        cached.source_uri = self.canonical.clone();
        cached.source_ref = Some(self.reference.clone());
        cached.registry_name = None;
        cached.overridden = self.kind == DirectKind::Override;
        cached.is_git_source = self.kind == DirectKind::Git;
        cached.recorded_source_kind = Some(self.kind.source_kind());
        let actual = self.serving.as_deref().unwrap_or(&self.physical);
        cached.fetched_from = self.evidence().fetched_from.or_else(|| {
            (actual != self.canonical).then(|| vibe_core::progress::transfer::safe_remote(actual))
        });
    }
}
fn direct_refusal(reason: &str) -> RegistryError {
    RegistryError::MalformedMeta {
        path: PathBuf::from("direct-source"),
        reason: reason.into(),
    }
}
impl MultiRegistryResolver {
    pub(super) fn retain_direct_source(
        &self,
        input: DirectSourceInput<'_>,
    ) -> Result<Arc<DirectSourceLease>, RegistryError> {
        let DirectSourceInput {
            group,
            name,
            canonical,
            physical,
            publication,
            reference,
            execution_reference,
            kind,
            auth,
            token_env,
        } = input;
        // An operator relocation never forwards a declared HTTPS token to a
        // different origin. It does not acquire a target registry's credentials.
        let crosses_origin = match (
            reqwest::Url::parse(strip_git_plus_prefix(canonical)),
            reqwest::Url::parse(strip_git_plus_prefix(physical)),
        ) {
            (Ok(a), Ok(b)) => a.origin() != b.origin(),
            _ => canonical != physical,
        };
        let (auth, token_env) = if crosses_origin && auth == vibe_core::manifest::AuthKind::TokenEnv
        {
            (vibe_core::manifest::AuthKind::None, None)
        } else {
            (auth, token_env)
        };
        let registry = Arc::new(
            GitPerPackageRegistry::open_single_package_with_deferred_auth(
                "direct-source",
                physical,
                reference,
                &self.cache_root,
                self.backend.clone(),
                DEFAULT_FRESHNESS_SECS,
                auth,
                token_env,
            )?
            .with_source_publication(publication),
        );
        let native = if registry.native_source_for(physical)? {
            Some(registry.retain_native_package(group, name, physical, execution_reference)?)
        } else {
            None
        };
        let manifest = match &native {
            Some(source) => source.manifest().clone(),
            None => registry.fetch_manifest_at_ref(group, name, reference)?,
        };
        let commit = native.as_ref().map(|source| source.commit().to_owned());
        let serving = native
            .as_ref()
            .and_then(|source| source.serving_endpoint().map(str::to_owned));
        let recorded_reference = native
            .as_ref()
            .map(|source| source.reference())
            .unwrap_or(reference)
            .to_owned();
        Ok(Arc::new(DirectSourceLease {
            registry,
            canonical: canonical.into(),
            physical: physical.into(),
            reference: recorded_reference,
            kind,
            publication: publication.observed(physical, serving.as_deref().unwrap_or(physical)),
            manifest,
            commit,
            serving,
            native: native.map(|source| Mutex::new(Some(source))),
        }))
    }
    pub(crate) fn acquire_direct_source(
        &self,
        resolution: &MultiResolution,
        store: &Path,
        expected: Option<&str>,
        slot: Option<&Path>,
    ) -> Result<crate::PackageAcquisition, RegistryError> {
        let lease = resolution
            .direct
            .as_ref()
            .ok_or_else(|| direct_refusal("direct acquisition has no retained owner"))?;
        lease.validate(resolution)?;
        let acquisition = if let Some(native) = &lease.native {
            let source = native
                .lock()
                .map_err(|_| direct_refusal("retained source lock poisoned"))?
                .take()
                .ok_or_else(|| direct_refusal("retained native source was already consumed"))?;
            lease.registry.consume_retained_native(
                source,
                &resolution.resolved,
                store,
                expected,
                slot,
            )?
        } else {
            let cached = match lease.kind {
                DirectKind::Git => self.fetch_git_source(resolution, store, expected)?,
                DirectKind::Override => self.fetch_override(resolution, store)?,
            };
            crate::PackageAcquisition::Cached(Box::new(cached))
        };
        match acquisition {
            crate::PackageAcquisition::Cached(mut cached) => {
                lease.project_cached(&mut cached);
                Ok(crate::PackageAcquisition::Cached(cached))
            }
            crate::PackageAcquisition::InPlace(prepared) => Ok(crate::PackageAcquisition::InPlace(
                prepared
                    .with_direct_evidence(
                        &lease.canonical,
                        &lease.reference,
                        lease.kind.source_kind(),
                    )?
                    .with_publication(lease.publication, &lease.physical)?,
            )),
        }
    }
    pub(super) fn prepare_direct_source(
        &self,
        resolution: &MultiResolution,
        slot: &Path,
    ) -> Result<crate::PreparedInPlace, RegistryError> {
        let lease = resolution
            .direct
            .as_ref()
            .ok_or_else(|| direct_refusal("direct preparation has no retained owner"))?;
        if lease.native.is_none()
            || !lease
                .manifest
                .require_package()?
                .materialization
                .is_in_place()
        {
            return Err(RegistryError::InPlaceUnsupported {
                group: resolution.resolved.group.clone(),
                name: resolution.resolved.name.clone(),
            });
        }
        match self.acquire_direct_source(resolution, &self.cache_root, None, Some(slot))? {
            crate::PackageAcquisition::InPlace(prepared) => Ok(prepared),
            crate::PackageAcquisition::Cached(_) => Err(direct_refusal(
                "this direct source does not supply a controlled in-place preparation",
            )),
        }
    }
}
