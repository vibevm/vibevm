//! Reached host inputs retained by the existing SectionSource operation.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-READ",
    r = 1
);
use super::*;
use crate::source_policy::{
    CitedNativeSource, PackagePolicyObservation, PackageSourceMode, observe_package_policy,
};
use std::sync::Arc;

#[derive(Default)]
enum HostMode {
    #[default]
    Unseen,
    // Legacy keeps its existing alias/read epochs, but cannot silently change
    // to declared meaning while an operation may retain parsed documents.
    Legacy,
    Declared(Arc<PackagePolicyObservation>),
}
#[derive(Default)]
pub(crate) struct HostObservations {
    mode: HostMode,
    documents: BTreeMap<String, Arc<HostRead>>,
}
pub(super) struct HostRead {
    policy: Arc<PackagePolicyObservation>,
    source: CitedNativeSource,
    path: PathBuf,
    stem: String,
}
impl HostRead {
    pub(super) fn resolved(&self) -> ResolvedSourceFile {
        ResolvedSourceFile {
            path: self.path.clone(),
            format: "markdown".into(),
            physical_stem: self.stem.clone(),
        }
    }
    pub(super) fn media(&self) -> (vibe_core::manifest::SourceInterpretation, &str) {
        (self.source.interpretation, &self.source.media)
    }
    fn recheck(&self) -> Result<(), ResolveError> {
        if self
            .policy
            .materialized_record_present()
            .map_err(|error| super::observation::failure(&self.path, error))?
        {
            return Err(super::observation::failure(
                &self.path,
                "materialized source now requires an independently bound installed reader",
            ));
        }

        self.policy
            .recheck_cited_native(&self.source)
            .map_err(|error| super::observation::failure(&self.path, error))
    }
    pub(super) fn read<'a>(
        &self,
        overlay: Option<&'a [u8]>,
    ) -> Result<std::borrow::Cow<'a, [u8]>, ResolveError> {
        self.recheck()?;
        let bytes = match overlay {
            Some(bytes) if bytes == self.source.observation.bytes() => {
                std::borrow::Cow::Borrowed(bytes)
            }
            Some(_) => {
                return Err(super::observation::failure(
                    &self.path,
                    "declared host overlay differs from captured original bytes",
                ));
            }
            None => std::borrow::Cow::Owned(self.source.observation.bytes().to_vec()),
        };
        self.recheck()?;
        Ok(bytes)
    }
}
impl FileResolver {
    pub(super) fn host_location(
        &self,
        address: &SpecAddress,
        observed: &mut HostObservations,
    ) -> Result<Option<super::observation::SourceLocation>, ResolveError> {
        let Authority::Package {
            group,
            name,
            version,
        } = &address.authority
        else {
            return Ok(None);
        };
        if !self.is_self(group, name) {
            return Ok(None);
        }
        if let Some(version) = version {
            return Err(ResolveError::SelfCoordinateVersioned {
                self_group: group.clone(),
                self_name: name.clone(),
                version: version.clone(),
            });
        }
        // Selected/derived slots are not a new caller-root grant. The checked
        // installed arm has already run; without it, existing refusal remains.
        if matches!(observed.mode, HostMode::Unseen) && self.installed_self_candidate() {
            return Ok(None);
        }
        let policy = match &observed.mode {
            HostMode::Declared(policy) => {
                policy
                    .recheck_unchanged()
                    .map_err(|error| super::observation::failure(&self.self_root, error))?;
                policy.clone()
            }
            HostMode::Unseen | HostMode::Legacy => {
                let policy = observe_package_policy(&self.self_root)
                    .map_err(|error| super::observation::failure(&self.self_root, error))?;
                if policy
                    .materialized_record_present()
                    .map_err(|error| super::observation::failure(&self.self_root, error))?
                {
                    if !matches!(observed.mode, HostMode::Unseen) {
                        return Err(super::observation::failure(
                            &self.self_root,
                            "source root acquired a materialization record during source operation",
                        ));
                    }
                    return Ok(None);
                }
                if matches!(
                    policy.mode(),
                    PackageSourceMode::Explicit | PackageSourceMode::ExplicitEmpty
                ) {
                    if matches!(observed.mode, HostMode::Legacy) {
                        return Err(super::observation::failure(
                            &self.self_root,
                            "host policy changed from legacy to declared during source operation",
                        ));
                    }
                    let policy = Arc::new(policy);
                    observed.mode = HostMode::Declared(policy.clone());
                    policy
                } else {
                    observed.mode = HostMode::Legacy;
                    return Ok(None);
                }
            }
        };
        if policy
            .materialized_record_present()
            .map_err(|error| super::observation::failure(&self.self_root, error))?
        {
            return Err(super::observation::failure(
                &self.self_root,
                "materialized source requires an independently bound installed reader",
            ));
        }
        let source = policy
            .capture_addressed_native(&format!("{group}/{name}"), &address.doc_path)
            .map_err(|error| super::observation::failure(&self.self_root, error))?;
        if let Some(retained) = observed.documents.get(&source.relative) {
            retained.recheck()?;
            return Ok(Some(super::observation::SourceLocation::host(
                retained.clone(),
            )));
        }
        let path = self.self_root.join(&source.relative);
        let stem = path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| super::observation::failure(&path, "native physical stem unavailable"))?
            .to_owned();
        let source = Arc::new(HostRead {
            policy,
            source,
            path,
            stem,
        });
        observed
            .documents
            .insert(source.source.relative.clone(), source.clone());
        Ok(Some(super::observation::SourceLocation::host(source)))
    }
    fn installed_self_candidate(&self) -> bool {
        if let Some(selected) = self.selected.as_ref().and_then(|selected| {
            self.self_coord
                .group
                .as_ref()
                .and_then(|group| selected.get(&(group.clone(), self.self_coord.name.clone())))
        }) && (selected.root == self.self_root
            || selected
                .root
                .canonicalize()
                .ok()
                .zip(self.self_root.canonicalize().ok())
                .is_some_and(|(selected, host)| selected == host))
        {
            return true;
        }
        let in_layout = |path: &Path| path.ancestors().any(slot_root);
        in_layout(&self.self_root)
            || self
                .self_root
                .canonicalize()
                .is_ok_and(|root| in_layout(&root))
    }
    pub(crate) fn admit_host_reference(
        &self,
        address: &SpecAddress,
        observed: &mut HostObservations,
    ) -> Result<(), ResolveError> {
        if self.declared_location(address, &[])?.is_some() {
            return Ok(());
        }
        self.host_location(address, observed).map(|_| ())
    }
    pub(crate) fn guard_host_pattern(
        &self,
        address: &SpecAddress,
        observed: &HostObservations,
    ) -> Result<(), ResolveError> {
        let Authority::Package { group, name, .. } = &address.authority else {
            return Ok(());
        };
        if self.self_coord.group.as_deref() != Some(group)
            || !super::glob::name_matches(name, &self.self_coord.name)
            || (matches!(observed.mode, HostMode::Unseen) && self.installed_self_candidate())
        {
            return Ok(());
        }
        if let HostMode::Declared(policy) = &observed.mode {
            policy
                .recheck_unchanged()
                .map_err(|error| super::observation::failure(&self.self_root, error))?;
        }
        let policy = observe_package_policy(&self.self_root)
            .map_err(|error| super::observation::failure(&self.self_root, error))?;
        if matches!(
            policy.mode(),
            PackageSourceMode::Explicit | PackageSourceMode::ExplicitEmpty
        ) {
            return Err(super::observation::failure(
                &self.self_root,
                policy.unsupported("declared host pattern inventory"),
            ));
        }
        Ok(())
    }
}

// Reserved slot shape, not an arbitrary ancestor with the same directory name.
pub(crate) fn slot_root(path: &Path) -> bool {
    fn deps(path: &Path) -> bool {
        path.ends_with(vibe_core::layout::current_vibedeps_root())
            || path
                .file_name()
                .is_some_and(|name| name == vibe_core::layout::LEGACY_VIBEDEPS_DIR)
    }
    fn coordinate(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.rsplit_once('.'))
            .is_some_and(|(group, name)| {
                vibe_core::Group::parse(group).is_ok()
                    && vibe_core::PackageName::parse(name).is_ok()
            })
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    if deps(parent) && coordinate(path) {
        return true;
    }
    parent.parent().is_some_and(deps)
        && coordinate(parent)
        && path
            .file_name()
            .and_then(|version| version.to_str())
            .is_some_and(|version| {
                vibe_core::VersionSpec::parse(&format!("={version}"))
                    .is_ok_and(|version| version.is_exact_pin())
            })
}
