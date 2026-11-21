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
    choice_inventory: Option<ChoiceInventory>,
}

#[derive(Debug)]
enum Guard {
    Authored(PackagePolicyObservation, CitedNativeSource),
    Legacy(PackagePolicyObservation, vibe_safefs::SourceObservation),
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
        if let Some(inventory) = &self.choice_inventory {
            inventory.recheck()?;
        }
        match &self.guard {
            Guard::Authored(policy, source) => {
                reject_materialized(policy)?;
                policy
                    .recheck_cited_native(source)
                    .map_err(|e| e.to_string())
            }
            Guard::Legacy(policy, observed) => {
                policy.recheck_unchanged().map_err(|e| e.to_string())?;
                if self.choice_inventory.is_some() {
                    reject_materialized(policy)?;
                    let current = policy
                        .read_source_observation(&self.relative)
                        .map_err(|e| e.to_string())?;
                    if current.as_ref().is_none_or(|current| {
                        current.bytes() != self.bytes.as_slice()
                            || current.identity() != observed.identity()
                    }) {
                        return Err(refused("answered Doc page changed"));
                    }
                }
                Ok(())
            }
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

pub(super) fn recheck_lock(project: &Project, bytes: &[u8]) -> Result<(), String> {
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
                choice_inventory: None,
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
        let observed = policy
            .read_source_observation(&relative)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| refused("legacy document missing"))?;
        let bytes = observed.bytes().to_vec();
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
            choice_inventory: None,
            guard: Guard::Legacy(policy, observed),
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
            choice_inventory: None,
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

/// Finite candidate inventory held only by answered in-tree reads.
#[derive(Debug)]
struct ChoiceInventory {
    project: Project,
    names: Vec<String>,
    identities: Vec<(String, vibe_safefs::FileIdentity)>,
}
impl ChoiceInventory {
    fn recheck(&self) -> Result<(), String> {
        let rebound = Project::open(self.project.root_path()).map_err(|e| e.to_string())?;
        if rebound.root_identity().map_err(|e| e.to_string())?
            != self.project.root_identity().map_err(|e| e.to_string())?
        {
            return Err(refused("Doc candidate directory changed"));
        }
        let root = self.project.root_dir().map_err(|e| e.to_string())?;
        let mut names = self
            .project
            .child_names_bounded(&root, 128)
            .map_err(|e| e.to_string())?;
        names.sort();
        if names != self.names {
            return Err(refused("Doc candidate inventory changed"));
        }
        for (name, expected) in &self.identities {
            let child = root.open_child(name).map_err(|e| e.to_string())?;
            if child.identity().map_err(|e| e.to_string())? != *expected {
                return Err(refused("Doc candidate identity changed"));
            }
        }
        Ok(())
    }
}
impl CapturedDocument {
    pub(crate) fn choice_manifest(&self) -> Result<&Manifest, String> {
        if self.source != Source::InTree || self.choice_inventory.is_none() {
            return Err(refused(
                "answered Doc requires a unique captured in-tree source",
            ));
        }
        let policy = match &self.guard {
            Guard::Authored(policy, _) | Guard::Legacy(policy, _) => policy,
            _ => return Err(refused("answered installed Doc unsupported")),
        };
        self.recheck()?;
        reject_materialized(policy)?;
        policy
            .manifest()
            .ok_or_else(|| refused("captured Doc manifest missing"))
    }
}
impl SpecSources {
    /// Capture the page and candidate inventory in the SAME lookup. No fallback
    /// to checkout/store/lock is allowed for request-local Doc configuration.
    pub(crate) fn capture_document_with_choice(
        &self,
        address: &SpecAddress,
    ) -> Result<Option<CapturedDocument>, String> {
        let Authority::Package {
            group,
            name,
            version,
        } = &address.authority
        else {
            return Err(refused("Doc choice address must name package"));
        };
        if self
            .checkout
            .as_ref()
            .is_some_and(|(coord, _)| coord.group.as_deref() == Some(group) && coord.name == *name)
        {
            return Err(refused("answered checkout Doc unsupported"));
        }
        let registry = self
            .in_tree
            .as_ref()
            .ok_or_else(|| refused("answered Doc requires in-tree source"))?;
        // Preserve the existing registry layout owner, while holding the exact
        // package directory and bounded names for ambiguity/recheck evidence.
        let parsed = Group::parse(group).map_err(|_| refused("invalid Doc coordinate"))?;
        let package_root = registry.join(parsed.as_str()).join(name);
        let project = Project::open(&package_root).map_err(refused)?;
        let root = project.root_dir().map_err(|e| e.to_string())?;
        let mut names = project
            .child_names_bounded(&root, 128)
            .map_err(|e| e.to_string())?;
        names.sort();
        let mut candidates = Vec::new();
        let mut identities = Vec::new();
        for child in &names {
            let Some(raw) = child.strip_prefix('v') else {
                continue;
            };
            let parsed = semver::Version::parse(raw)
                .map_err(|_| refused("invalid Doc version directory"))?;
            let held = root
                .open_child(child)
                .map_err(|_| refused("Doc candidate must be no-follow directory"))?;
            identities.push((child.clone(), held.identity().map_err(|e| e.to_string())?));
            if version
                .as_ref()
                .is_none_or(|wanted| wanted.trim_start_matches('=') == parsed.to_string())
            {
                candidates.push(parsed);
            }
        }
        if candidates.len() != 1 {
            return Err(refused(
                "answered Doc requires exactly one matching source identity",
            ));
        }
        let selected = candidates.remove(0);
        let inventory = ChoiceInventory {
            project,
            names,
            identities,
        };
        inventory.recheck()?;
        let mut captured = self
            .capture_authored(
                address,
                Instance {
                    source: Source::InTree,
                    version: selected.to_string(),
                    root: store::entry_dir(registry, &parsed, name, &selected),
                },
            )?
            .ok_or_else(|| refused("selected Doc page missing"))?;
        let manifest = match &captured.guard {
            Guard::Authored(policy, _) | Guard::Legacy(policy, _) => policy.manifest(),
            _ => None,
        }
        .ok_or_else(|| refused("selected Doc manifest missing"))?;
        let meta = manifest
            .require_package()
            .map_err(|_| refused("selected Doc package metadata missing"))?;
        if meta.kind != vibe_core::PackageKind::Doc
            || meta.group != parsed
            || meta.name.as_str() != name
            || meta.version != selected
        {
            return Err(refused("selected Doc coordinate/version/kind mismatch"));
        }
        captured.choice_inventory = Some(inventory);
        captured.recheck()?;
        Ok(Some(captured))
    }
}

#[cfg(test)]
#[path = "captured/choice_tests.rs"]
mod choice_tests;
