//! Selected page membership and bytes retained across one rendering operation.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-CENSUS",
    r = 1
);
use super::*;
use crate::pages::{Page, PageSet, UnreadablePage};
use std::sync::Arc;
use vibe_core::manifest::{SelectedSource, SourceInterpretation, select_source_rows};
use vibe_safefs::Project;
use vibe_spec::source_policy::{
    CapturedHostInventory, PackagePolicyObservation, PackageSourceMode, observe_package_policy,
};

/// A checked page snapshot with a retained finite source observation.
/// ```
/// use vibe_doc::citations::SpecSources;
/// let root = tempfile::tempdir().unwrap();
/// let pages = SpecSources::new().capture_pages(root.path()).unwrap();
/// assert!(pages.documents().is_empty());
/// pages.recheck().unwrap();
/// ```
#[derive(Debug)]
pub struct CapturedPages {
    set: PageSet,
    guard: Guard,
}
#[derive(Debug)]
enum Guard {
    Authored(PackagePolicyObservation, CapturedHostInventory),
    Legacy(PackagePolicyObservation),
    LegacyInstalled {
        policy: PackagePolicyObservation,
        project: Arc<Project>,
        lock: Vec<u8>,
        expected: vibe_spec::InstalledSourceExpectation,
        relative: String,
    },
    Installed {
        resolver: FileResolver,
        project: Arc<Project>,
        lock: Vec<u8>,
        group: String,
        name: String,
        bytes: Vec<(String, Vec<u8>)>,
    },
}
impl CapturedPages {
    pub(crate) fn canonical_addresses(&self) -> bool {
        matches!(self.guard, Guard::Authored(..) | Guard::Installed { .. })
    }
    pub fn set(&self) -> &PageSet {
        &self.set
    }
    pub fn documents(&self) -> Vec<String> {
        let mut names: Vec<_> = self
            .set
            .pages
            .iter()
            .map(|page| crate::pages::document_of(&page.rel).to_owned())
            .chain(
                self.set
                    .unreadable
                    .iter()
                    .map(|page| crate::pages::document_of(&page.rel).to_owned()),
            )
            .collect();
        names.sort();
        names
    }
    pub fn recheck(&self) -> Result<(), String> {
        match &self.guard {
            Guard::Authored(policy, inventory) => policy
                .recheck_host_inventory(inventory)
                .map_err(|e| e.to_string()),
            Guard::Legacy(policy) => policy.recheck_unchanged().map_err(|e| e.to_string()),
            Guard::LegacyInstalled {
                policy,
                project,
                lock,
                expected,
                relative,
            } => {
                super::captured::recheck_lock(project, lock)?;
                policy.recheck_unchanged().map_err(|e| e.to_string())?;
                let slot = project
                    .dir(&relative.split('/').collect::<Vec<_>>(), false)
                    .map_err(|e| e.to_string())?;
                if vibe_workspace::vibedeps::prepare_declared_query_read(
                    project, &slot, relative, expected,
                )
                .map_err(|e| e.to_string())?
                .is_some()
                {
                    return Err(refused("legacy selection became declared"));
                }
                super::captured::recheck_lock(project, lock)
            }
            Guard::Installed {
                resolver,
                project,
                lock,
                group,
                name,
                bytes,
            } => {
                super::captured::recheck_lock(project, lock)?;
                resolver
                    .recheck_installed_package(group, name)
                    .map_err(|e| e.to_string())?;
                for (relative, expected) in bytes {
                    if resolver
                        .read_installed_source(group, name, relative, None)
                        .map_err(|e| e.to_string())?
                        != *expected
                    {
                        return Err(refused("selected page changed"));
                    }
                }
                super::captured::recheck_lock(project, lock)
            }
        }
    }
}
fn refused(reason: impl fmt::Display) -> String {
    format!(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-CENSUS: {reason}; fix surface: selected Doc source policy and independent installed binding"
    )
}
fn append(
    set: &mut PageSet,
    original: &str,
    path: PathBuf,
    bytes: &[u8],
    media: &str,
) -> Result<(), String> {
    if media != "application/xml" {
        return Err(refused("Doc Markdown input is unsupported"));
    }
    let rel = original
        .strip_prefix("vibevm/vibespecs/")
        .unwrap_or(original)
        .to_owned();
    let parsed = std::str::from_utf8(bytes)
        .map_err(|e| e.to_string())
        .and_then(|text| {
            vibe_specdoc::from_xml_with(text, vibe_specdoc::Vocabulary::Doc)
                .map_err(|e| e.to_string())
        });
    match parsed {
        Ok(doc) => set.pages.push(Page { rel, path, doc }),
        Err(message) => set.unreadable.push(UnreadablePage { rel, path, message }),
    }
    Ok(())
}
impl SpecSources {
    /// Capture exactly the supplied package root; coordinate lookup cannot replace it.
    /// Installed declarations require this world's independently selected lock/format.
    /// ```
    /// use vibe_doc::citations::SpecSources;
    /// let root = tempfile::tempdir().unwrap();
    /// let captured = SpecSources::new().capture_pages(root.path()).unwrap();
    /// assert_eq!(captured.set().total(), 0);
    /// captured.recheck().unwrap();
    /// ```
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-CENSUS",
        r = 1
    )]
    pub fn capture_pages(&self, package_dir: &Path) -> Result<CapturedPages, String> {
        let policy = observe_package_policy(package_dir).map_err(|e| e.to_string())?;
        if let Some(workspace) = &self.lock
            && (package_dir.starts_with(workspace.join(vibe_core::layout::current_vibedeps_root()))
                || policy
                    .materialized_record_present()
                    .map_err(|e| e.to_string())?)
        {
            let project = Arc::new(Project::open(workspace).map_err(|e| e.to_string())?);
            let root = project.root_dir().map_err(|e| e.to_string())?;
            if let Some(bytes) = project
                .read_source_bytes_in(&root, Lockfile::FILENAME)
                .map_err(|e| e.to_string())?
            {
                let lock =
                    Lockfile::parse_str(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                for selected in &lock.packages {
                    let slot = vibe_workspace::vibedeps::slot_abs_path(
                        workspace,
                        &selected.group,
                        &selected.name,
                        &selected.version,
                    );
                    if slot == package_dir {
                        return self.capture_slot_pages(
                            package_dir,
                            workspace,
                            project,
                            bytes,
                            selected,
                            policy,
                        );
                    }
                }
            }
        }
        self.capture_original_pages(package_dir, policy)
    }
    fn capture_original_pages(
        &self,
        root: &Path,
        policy: PackagePolicyObservation,
    ) -> Result<CapturedPages, String> {
        let result = match policy.mode() {
            PackageSourceMode::ExplicitEmpty
                if !policy
                    .package()
                    .is_some_and(|package| package.kind == vibe_core::PackageKind::Doc) =>
            {
                if policy
                    .materialized_record_present()
                    .map_err(|e| e.to_string())?
                {
                    return Err(refused(
                        "materialized root requires independent lock binding",
                    ));
                }
                CapturedPages {
                    set: PageSet::default(),
                    guard: Guard::Legacy(policy),
                }
            }
            PackageSourceMode::Explicit | PackageSourceMode::ExplicitEmpty => {
                let package = policy.package().ok_or_else(|| {
                    refused("explicitly classified documentation requires an actual Doc package")
                })?;
                let coordinate = format!("{}/{}", package.group, package.name);
                let inventory = policy
                    .capture_native_inventory(&coordinate, SourceInterpretation::VibeDoc)
                    .map_err(|e| e.to_string())?;
                let mut set = PageSet::default();
                for resource in inventory.resources() {
                    let SelectedSource::Descriptor { descriptor, .. } = select_source_rows(
                        &coordinate,
                        policy.source_rows().unwrap_or_default(),
                        resource.relative(),
                    )
                    .map_err(|e| refused(format!("{e:?}")))?
                    else {
                        return Err(refused("selected native descriptor disappeared"));
                    };
                    append(
                        &mut set,
                        resource.relative(),
                        root.join(resource.relative()),
                        resource.bytes(),
                        descriptor.media(),
                    )?;
                }
                CapturedPages {
                    set,
                    guard: Guard::Authored(policy, inventory),
                }
            }
            PackageSourceMode::Legacy | PackageSourceMode::MissingManifest => {
                if policy
                    .materialized_record_present()
                    .map_err(|e| e.to_string())?
                {
                    return Err(refused(
                        "materialized root requires independent lock binding",
                    ));
                }
                let set = crate::pages::read_legacy(root).map_err(|e| e.to_string())?;
                CapturedPages {
                    set,
                    guard: Guard::Legacy(policy),
                }
            }
        };
        result.recheck()?;
        Ok(result)
    }
    fn capture_slot_pages(
        &self,
        root: &Path,
        workspace: &Path,
        project: Arc<Project>,
        lock: Vec<u8>,
        selected: &vibe_core::manifest::LockedPackage,
        policy: PackagePolicyObservation,
    ) -> Result<CapturedPages, String> {
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
        let Some(binding) = binding else {
            let set = crate::pages::read_legacy(root).map_err(|e| e.to_string())?;
            let result = CapturedPages {
                set,
                guard: Guard::LegacyInstalled {
                    policy,
                    project,
                    lock,
                    expected,
                    relative,
                },
            };
            result.recheck()?;
            return Ok(result);
        };
        if self.materialized_format.is_none()
            || selected.materialization.is_in_place()
            || expected.kind != vibe_core::PackageKind::Doc
        {
            return Err(refused(
                "declared installed Doc requires selected materialized format and supported Doc binding",
            ));
        }
        let instance = Instance {
            source: Source::Lock,
            version: expected.version.clone(),
            root: root.to_owned(),
        };
        let resolver = self
            .resolver_for(expected.group.as_str(), &expected.name, &instance)
            .with_installed_source_bindings(project.clone(), vec![binding.clone()], None)
            .map_err(|e| e.to_string())?;
        let mut set = PageSet::default();
        let mut captured = Vec::new();
        for row in binding
            .outputs()
            .iter()
            .filter(|row| row.interpretation == Some(SourceInterpretation::VibeDoc))
        {
            let bytes = resolver
                .read_installed_source(
                    expected.group.as_str(),
                    &expected.name,
                    &row.source_path,
                    None,
                )
                .map_err(|e| e.to_string())?;
            append(
                &mut set,
                &row.source_path,
                root.join(&row.output_path),
                &bytes,
                &row.media,
            )?;
            captured.push((row.source_path.clone(), bytes));
        }
        let result = CapturedPages {
            set,
            guard: Guard::Installed {
                resolver,
                project,
                lock,
                group: expected.group.to_string(),
                name: expected.name,
                bytes: captured,
            },
        };
        result.recheck()?;
        Ok(result)
    }
}
