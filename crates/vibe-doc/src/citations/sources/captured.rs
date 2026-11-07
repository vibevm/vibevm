//! Captured point documents retain their selected observation through rendering.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-POINT",
    r = 1
);

use super::*;
use std::sync::Arc;
use vibe_core::manifest::{LockedPackage, Manifest, SourceInterpretation};
use vibe_safefs::Project;
use vibe_spec::source_policy::{
    CitedNativeSource, PackagePolicyObservation, PackageSourceMode, observe_package_policy,
};

/// A native document and the finite observation that must survive its use.
/// ```
/// use vibe_doc::citations::SpecSources;
/// use vibe_spec::SpecAddress;
/// let address = SpecAddress::parse("spec://com.example/manual/guide#root").unwrap();
/// assert!(SpecSources::new().capture_document(&address).unwrap().is_none());
/// ```
#[derive(Debug)]
pub struct CapturedDocument {
    pub(crate) path: PathBuf,
    pub(crate) relative: String,
    pub(crate) source: Source,
    pub(crate) lang: String,
    pub(crate) interpretation: SourceInterpretation,
    pub(crate) media: String,
    pub(crate) bytes: Vec<u8>,
    guard: Guard,
}

#[derive(Debug)]
enum Guard {
    Authored(PackagePolicyObservation, CitedNativeSource),
    Legacy(PackagePolicyObservation),
    Installed {
        resolver: FileResolver,
        group: String,
        name: String,
        original: String,
        project: Arc<Project>,
        lock: Vec<u8>,
    },
}

impl CapturedDocument {
    /// Original logical relative path, independent of a converted output suffix.
    pub fn relative(&self) -> &str {
        &self.relative
    }
    /// Recheck retained source selection after parsing or rendering.
    pub fn recheck(&self) -> Result<(), String> {
        match &self.guard {
            Guard::Authored(policy, source) => {
                reject_materialized(policy)?;
                policy
                    .recheck_cited_native(source)
                    .map_err(|e| e.to_string())
            }
            Guard::Legacy(policy) => policy.recheck_unchanged().map_err(|e| e.to_string()),
            Guard::Installed {
                resolver,
                group,
                name,
                original,
                project,
                lock,
            } => {
                recheck_lock(project, lock)?;
                let bytes = resolver
                    .read_installed_source(group, name, original, None)
                    .map_err(|e| e.to_string())?;
                if bytes != self.bytes {
                    return Err(refused("selected document changed"));
                }
                recheck_lock(project, lock)
            }
        }
    }

    /// Parse captured native bytes directly, without a Markdown round trip.
    pub fn parse(&self) -> Result<vibe_specdoc::doc::SpecDoc, String> {
        let text = std::str::from_utf8(&self.bytes).map_err(|e| refused(e.to_string()))?;
        let result = match (self.interpretation, self.media.as_str()) {
            (SourceInterpretation::VibeSpec, "text/markdown") => vibe_specdoc::from_markdown(text),
            (SourceInterpretation::VibeSpec, "application/xml") => vibe_specdoc::from_xml(text),
            (SourceInterpretation::VibeDoc, "application/xml") => {
                vibe_specdoc::from_xml_with(text, vibe_specdoc::Vocabulary::Doc)
            }
            _ => return Err(refused("unsupported native media/interpretation pair")),
        };
        result.map_err(|e| refused(e.to_string()))
    }
}

fn refused(reason: impl fmt::Display) -> String {
    format!(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-POINT: {reason}; fix surface: selected document, source policy and independent lock input format"
    )
}

fn language(manifest: Option<&Manifest>) -> String {
    manifest
        .map(|m| m.i18n.canonical.clone())
        .unwrap_or_else(|| vibe_core::manifest::i18n::DEFAULT_CANONICAL_LANGUAGE.into())
}

fn reject_materialized(policy: &PackagePolicyObservation) -> Result<(), String> {
    if policy
        .materialized_record_present()
        .map_err(|e| e.to_string())?
    {
        return Err(refused(
            "materialized source requires independent lock binding",
        ));
    }
    Ok(())
}

fn recheck_lock(project: &Project, bytes: &[u8]) -> Result<(), String> {
    let root = project.root_dir().map_err(|e| e.to_string())?;
    if project
        .read_source_bytes_in(&root, Lockfile::FILENAME)
        .map_err(|e| e.to_string())?
        .as_deref()
        != Some(bytes)
    {
        return Err(refused("selected lock changed"));
    }
    Ok(())
}

impl SpecSources {
    /// Capture one selected native document. Absence is distinct from invalid selection.
    ///
    /// ```
    /// use vibe_doc::citations::SpecSources;
    /// use vibe_spec::SpecAddress;
    /// let root = tempfile::tempdir().unwrap();
    /// std::fs::write(root.path().join("vibe.toml"),
    ///     "[project]\ngroup='com.example'\nname='host'\nversion='1.0.0'\n\
    ///      [[project.sources]]\nfile='guide.payload'\nmedia='application/xml'\n\
    ///      interpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
    /// std::fs::write(root.path().join("guide.payload"),
    ///     "<spec xmlns='https://vibevm.org/spec/1'><title id='root'>Guide</title></spec>").unwrap();
    /// let world = SpecSources::for_checkout(root.path(), Some("com.example"), "host");
    /// let address = SpecAddress::parse("spec://com.example/host/guide.payload#root").unwrap();
    /// let captured = world.capture_document(&address).unwrap().unwrap();
    /// assert_eq!(captured.relative(), "guide.payload");
    /// assert_eq!(captured.parse().unwrap().title.unwrap().text, "Guide");
    /// captured.recheck().unwrap();
    /// ```
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-POINT",
        r = 1
    )]
    pub fn capture_document(
        &self,
        address: &SpecAddress,
    ) -> Result<Option<CapturedDocument>, String> {
        let Authority::Package {
            group,
            name,
            version,
        } = &address.authority
        else {
            return Err(refused("address names no package"));
        };
        if let Some((coord, root)) = &self.checkout
            && coord.group.as_deref() == Some(group)
            && coord.name == *name
        {
            if version.is_some() {
                return Err(refused("self coordinate is unversioned"));
            }
            return self.capture_authored(
                address,
                Instance {
                    source: Source::Checkout,
                    version: String::new(),
                    root: root.clone(),
                },
            );
        }
        let parsed = Group::parse(group).map_err(|e| e.to_string())?;
        if let Some(root) = &self.in_tree
            && let Some(instance) = registry_instance(root, Source::InTree, &parsed, name)
        {
            return self.capture_authored(address, instance);
        }
        if let Some(workspace) = &self.lock {
            let project = Arc::new(Project::open(workspace).map_err(|e| e.to_string())?);
            let root = project.root_dir().map_err(|e| e.to_string())?;
            if let Some(bytes) = project
                .read_source_bytes_in(&root, Lockfile::FILENAME)
                .map_err(|e| e.to_string())?
            {
                let lock =
                    Lockfile::parse_str(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                if let Some(selected) = lock
                    .packages
                    .iter()
                    .find(|p| p.group == parsed && p.name.as_str() == name)
                {
                    return self.capture_installed(address, workspace, project, bytes, selected);
                }
            }
        }
        if let Some(root) = &self.store
            && let Some(instance) = registry_instance(root, Source::Store, &parsed, name)
        {
            return self.capture_authored(address, instance);
        }
        Ok(None)
    }

    fn capture_authored(
        &self,
        address: &SpecAddress,
        instance: Instance,
    ) -> Result<Option<CapturedDocument>, String> {
        let Authority::Package {
            group,
            name,
            version,
        } = &address.authority
        else {
            unreachable!()
        };
        if let Some(version) = version
            && version.trim_start_matches('=') != instance.version
        {
            return Err(refused(format!(
                "requested version {version} differs from selected {}",
                instance.version
            )));
        }
        let policy = observe_package_policy(&instance.root).map_err(|e| e.to_string())?;
        let lang = language(policy.manifest());
        if matches!(
            policy.mode(),
            PackageSourceMode::Explicit | PackageSourceMode::ExplicitEmpty
        ) {
            reject_materialized(&policy)?;
            if instance.source != Source::Checkout
                && policy
                    .package()
                    .is_some_and(|package| package.version.to_string() != instance.version)
            {
                return Err(refused(
                    "authored manifest version differs from selected instance",
                ));
            }
            let source = match policy.capture_addressed_native(&format!("{group}/{name}"), &address.doc_path) {
                Ok(source) => source,
                Err(error) if error.kind == vibe_spec::source_policy::SourcePolicyErrorKind::MissingNativeAddress => return Ok(None),
                Err(error) => return Err(error.to_string()),
            };
            let captured = CapturedDocument {
                path: instance.root.join(&source.relative),
                relative: source.relative.clone(),
                source: instance.source,
                lang,
                interpretation: source.interpretation,
                media: source.media.clone(),
                bytes: source.observation.bytes().to_vec(),
                guard: Guard::Authored(policy, source),
            };
            captured.recheck()?;
            return Ok(Some(captured));
        }
        let path = match self
            .resolver_for(group, name, &instance)
            .resolve_file(address)
        {
            Ok(path) => path,
            Err(vibe_spec::ResolveError::DocNotFound { .. }) => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        let relative = fwd(path
            .strip_prefix(&instance.root)
            .map_err(|e| e.to_string())?);
        let bytes = policy
            .read_source_bytes(&relative)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| refused("legacy document missing"))?;
        let interpretation = if policy
            .package()
            .is_some_and(|p| p.kind == vibe_core::PackageKind::Doc)
        {
            SourceInterpretation::VibeDoc
        } else {
            SourceInterpretation::VibeSpec
        };
        let media = if path.extension().is_some_and(|e| e == "xml") {
            "application/xml"
        } else {
            "text/markdown"
        }
        .into();
        let captured = CapturedDocument {
            path,
            relative,
            source: instance.source,
            lang,
            interpretation,
            media,
            bytes,
            guard: Guard::Legacy(policy),
        };
        captured.recheck()?;
        Ok(Some(captured))
    }

    fn capture_installed(
        &self,
        address: &SpecAddress,
        workspace: &Path,
        project: Arc<Project>,
        lock: Vec<u8>,
        selected: &LockedPackage,
    ) -> Result<Option<CapturedDocument>, String> {
        let root = vibe_workspace::vibedeps::slot_abs_path(
            workspace,
            &selected.group,
            &selected.name,
            &selected.version,
        );
        let relative = fwd(root.strip_prefix(workspace).map_err(|e| e.to_string())?);
        let slot = project
            .dir(&relative.split('/').collect::<Vec<_>>(), false)
            .map_err(|e| e.to_string())?;
        let expected = vibe_spec::InstalledSourceExpectation {
            kind: selected.kind,
            group: selected.group.clone(),
            name: selected.name.to_string(),
            version: selected.version.to_string(),
            source_hash: selected.content_hash.clone(),
            materialized_format: self.materialized_format.unwrap_or_default(),
        };
        let binding = vibe_workspace::vibedeps::prepare_declared_query_read(
            &project, &slot, &relative, &expected,
        )
        .map_err(|e| e.to_string())?;
        let instance = Instance {
            source: Source::Lock,
            version: expected.version.clone(),
            root,
        };
        let Some(binding) = binding else {
            return self.capture_authored(address, instance);
        };
        if self.materialized_format.is_none() {
            return Err(refused(
                "declared installed source requires selected materialized format",
            ));
        }
        if selected.materialization.is_in_place() {
            return Err(refused("declared in-place sources are unsupported"));
        }
        let lang = language(binding.manifest());
        let resolver = self
            .resolver_for(expected.group.as_str(), &expected.name, &instance)
            .with_installed_source_bindings(project.clone(), vec![binding.clone()], None)
            .map_err(|e| e.to_string())?;
        let path = match resolver.resolve_file(address) {
            Ok(path) => path,
            Err(vibe_spec::ResolveError::DocNotFound { .. }) => {
                resolver
                    .recheck_installed_package(expected.group.as_str(), &expected.name)
                    .map_err(|e| e.to_string())?;
                recheck_lock(&project, &lock)?;
                return Ok(None);
            }
            Err(error) => return Err(error.to_string()),
        };
        let row = binding
            .outputs()
            .iter()
            .find(|row| instance.root.join(&row.output_path) == path)
            .ok_or_else(|| refused("selected native output unavailable"))?;
        let interpretation = row
            .interpretation
            .ok_or_else(|| refused("selected source is raw"))?;
        let bytes = resolver
            .read_installed_source(
                expected.group.as_str(),
                &expected.name,
                &row.source_path,
                None,
            )
            .map_err(|e| e.to_string())?;
        let captured = CapturedDocument {
            path,
            relative: row.source_path.clone(),
            source: Source::Lock,
            lang,
            interpretation,
            media: row.media.clone(),
            bytes,
            guard: Guard::Installed {
                resolver,
                group: expected.group.to_string(),
                name: expected.name,
                original: row.source_path.clone(),
                project,
                lock,
            },
        };
        captured.recheck()?;
        Ok(Some(captured))
    }
}
