//! A held lookup/read epoch; logical paths remain the exact overlay keys.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-OBSERVED-SOURCE-READ"
);
use super::*;
use crate::source_policy::{PackagePolicyCheckpoint, PackageSourceMode, observe_package_policy_in};
use vibe_safefs::{Pinned, Project};

pub(crate) struct SourceLocation {
    pub(crate) resolved: ResolvedSourceFile,
    reader: LocationReader,
}

enum LocationReader {
    Legacy {
        project: Project,
        parent: Pinned,
        name: String,
        policy: Box<PackagePolicyCheckpoint>,
    },
    Declared(super::declared::DeclaredRead),
    Host(std::sync::Arc<super::host::HostRead>),
}

impl SourceLocation {
    pub(super) fn declared(
        resolved: ResolvedSourceFile,
        reader: super::declared::DeclaredRead,
    ) -> Self {
        Self {
            resolved,
            reader: LocationReader::Declared(reader),
        }
    }
    pub(super) fn host(reader: std::sync::Arc<super::host::HostRead>) -> Self {
        Self {
            resolved: reader.resolved(),
            reader: LocationReader::Host(reader),
        }
    }
    pub(crate) fn declared_media(
        &self,
    ) -> Option<(vibe_core::manifest::SourceInterpretation, &str)> {
        match &self.reader {
            LocationReader::Declared(reader) => Some(reader.media()),
            LocationReader::Host(reader) => Some(reader.media()),
            _ => None,
        }
    }
    pub(crate) fn read_with_overlay<'a>(
        &self,
        overlay: Option<&'a [u8]>,
    ) -> Result<std::borrow::Cow<'a, [u8]>, ResolveError> {
        let LocationReader::Legacy {
            project,
            parent,
            name,
            policy,
        } = &self.reader
        else {
            return match &self.reader {
                LocationReader::Declared(reader) => reader.read(overlay),
                LocationReader::Host(reader) => reader.read(overlay),
                LocationReader::Legacy { .. } => unreachable!(),
            };
        };
        policy
            .recheck_unchanged()
            .map_err(|error| failure(&self.resolved.path, error))?;
        let bytes = if let Some(bytes) = overlay {
            std::borrow::Cow::Borrowed(bytes)
        } else {
            std::borrow::Cow::Owned(
                project
                    .read_source_bytes_in(parent, name)
                    .map_err(|error| failure(&self.resolved.path, error))?
                    .ok_or_else(|| {
                        failure(
                            &self.resolved.path,
                            "selected source disappeared before reading",
                        )
                    })?,
            )
        };
        policy
            .recheck_unchanged()
            .map_err(|error| failure(&self.resolved.path, error))?;
        Ok(bytes)
    }
}

/// One held dependency layout for ordinary installed-set enumeration.
/// This private lookup context is not a captured world or public authority.
pub(super) struct InstalledSlots {
    project: Project,
    slots: Pinned,
    logical: PathBuf,
}

impl FileResolver {
    pub(crate) fn locate_source(
        &self,
        address: &SpecAddress,
        formats: &[String],
    ) -> Result<SourceLocation, ResolveError> {
        self.locate_source_using(address, formats, None)
    }
    pub(crate) fn locate_source_with_host(
        &self,
        address: &SpecAddress,
        formats: &[String],
        observed: &mut super::host::HostObservations,
    ) -> Result<SourceLocation, ResolveError> {
        self.locate_source_using(address, formats, Some(observed))
    }
    fn locate_source_using(
        &self,
        address: &SpecAddress,
        formats: &[String],
        observed: Option<&mut super::host::HostObservations>,
    ) -> Result<SourceLocation, ResolveError> {
        paths::source(&address.doc_path, formats)?;
        if glob::is_pattern(address) {
            return Err(ResolveError::PatternNotExpanded {
                given: address.raw.clone(),
            });
        }
        if let Some(declared) = self.declared_location(address, formats)? {
            return Ok(declared);
        }
        if let Some(observed) = observed
            && let Some(host) = self.host_location(address, observed)?
        {
            return Ok(host);
        }
        let (project, package, logical) = self.observed_root(address)?;
        let (parent, resolved, policy) =
            locate_in(&project, &package, &logical, &address.doc_path, formats)?;
        let name = resolved
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| failure(&resolved.path, "selected filename unavailable"))?
            .to_owned();
        Ok(SourceLocation {
            resolved,
            reader: LocationReader::Legacy {
                project,
                parent,
                name,
                policy: Box::new(policy),
            },
        })
    }

    pub(super) fn installed_slots(&self) -> Result<Option<InstalledSlots>, ResolveError> {
        let (project, root, logical) = admit(&self.ws_root)?;
        let Some((slots, logical)) = layout(
            &project,
            &root,
            &logical,
            lookup::NEW_VIBEDEPS_ROOT,
            lookup::LEGACY_VIBEDEPS_ROOT,
        )?
        else {
            return Ok(None);
        };
        Ok(Some(InstalledSlots {
            project,
            slots,
            logical,
        }))
    }
    fn observed_root(
        &self,
        address: &SpecAddress,
    ) -> Result<(Project, Pinned, PathBuf), ResolveError> {
        match &address.authority {
            Authority::Host(given) => Err(ResolveError::LegacyHostAuthority {
                given: given.clone(),
                hint: self.legacy_host_hint(given),
            }),
            Authority::Package {
                group,
                name,
                version,
            } if self.is_self(group, name) => {
                if let Some(version) = version {
                    return Err(ResolveError::SelfCoordinateVersioned {
                        self_group: group.clone(),
                        self_name: name.clone(),
                        version: version.clone(),
                    });
                }
                if let Some(relative) = self.derived_slot_relative(&self.self_root) {
                    return self.observed_descendant(&self.self_root, &relative);
                }
                admit(&self.self_root)
            }
            Authority::Package {
                group,
                name,
                version,
            } => {
                if let Some(selected) = &self.selected {
                    let chosen = selected
                        .get(&(group.clone(), name.clone()))
                        .ok_or_else(|| ResolveError::UnselectedPackage {
                            given: format!("{group}/{name}"),
                        })?;
                    if let Some(requested) = version {
                        let requested = requested.trim_start_matches('=');
                        if requested != chosen.version {
                            return Err(ResolveError::SelectedVersionMismatch {
                                given: format!("{group}/{name}"),
                                requested: requested.into(),
                                selected: chosen.version.clone(),
                            });
                        }
                    }
                    if let Some(relative) = self.derived_slot_relative(&chosen.root) {
                        return self.observed_descendant(&chosen.root, &relative);
                    }
                    return admit(&chosen.root);
                }
                paths::package_component(group, name, version.as_deref())?;
                let slots = self
                    .installed_slots()?
                    .ok_or_else(|| ResolveError::PackageSlotNotFound(name.clone()))?;
                let (package, logical) =
                    slots.package_root(group, name, version.as_deref(), &address.doc_path)?;
                Ok((slots.project, package, logical))
            }
        }
    }
    pub(super) fn derived_slot_relative(&self, root: &Path) -> Option<PathBuf> {
        let physical = self.ws_root.canonicalize().ok();
        root.strip_prefix(&self.ws_root)
            .ok()
            .or_else(|| {
                physical
                    .as_ref()
                    .and_then(|base| root.strip_prefix(base).ok())
            })
            .filter(|rel| {
                rel.starts_with(vibe_core::layout::current_vibedeps_root())
                    || rel.starts_with(vibe_core::layout::LEGACY_VIBEDEPS_DIR)
            })
            .map(Path::to_path_buf)
    }
    fn observed_descendant(
        &self,
        logical: &Path,
        relative: &Path,
    ) -> Result<(Project, Pinned, PathBuf), ResolveError> {
        let (project, _, _) = admit(&self.ws_root)?;
        let raw = relative
            .to_str()
            .ok_or_else(|| failure(logical, "derived slot path is not UTF-8"))?;
        let spelling = if cfg!(windows) {
            raw.replace('\\', "/")
        } else {
            raw.to_owned()
        };
        vibe_core::manifest::declarant_path(Path::new(&spelling))
            .map_err(|e| failure(logical, e.reason()))?;
        let pinned = project
            .dir(&spelling.split('/').collect::<Vec<_>>(), false)
            .map_err(|e| failure(logical, e))?;
        Ok((project, pinned, logical.to_path_buf()))
    }
}

impl InstalledSlots {
    pub(super) fn names(&self) -> Result<Vec<String>, ResolveError> {
        self.project
            .child_names(&self.slots)
            .map_err(|error| failure(&self.logical, error))
    }

    pub(super) fn locate_document(
        &self,
        group: &str,
        name: &str,
        version: Option<&str>,
        document: &str,
    ) -> Result<(), ResolveError> {
        let (package, logical) = self.package_root(group, name, version, document)?;
        locate_in(&self.project, &package, &logical, document, &[]).map(|_| ())
    }

    fn package_root(
        &self,
        group: &str,
        name: &str,
        version: Option<&str>,
        document: &str,
    ) -> Result<(Pinned, PathBuf), ResolveError> {
        let component = paths::package_component(group, name, version)?;
        let absent = || ResolveError::PackageSlotNotFound(name.into());
        let logical = self.logical.join(&component);
        let package = self
            .slots
            .open_child_checked(&component)
            .map_err(|error| failure(&logical, error))?
            .ok_or_else(absent)?;
        let version = match version {
            Some(version) => version.to_owned(),
            None => {
                let mut versions = Vec::new();
                for entry in self
                    .project
                    .child_names(&package)
                    .map_err(|error| failure(&logical, error))?
                {
                    if entry.starts_with(|c: char| c.is_ascii_digit())
                        && !self
                            .project
                            .source_file_present_in(&package, &entry)
                            .map_err(|error| failure(&logical.join(&entry), error))?
                        && package
                            .open_child_checked(&entry)
                            .map_err(|error| failure(&logical.join(&entry), error))?
                            .is_some()
                    {
                        versions.push(entry);
                    }
                }
                version_order::newest(versions.iter().map(String::as_str))
                    .ok_or_else(absent)?
                    .to_owned()
            }
        };
        let logical = logical.join(&version);
        let package = package
            .open_child_checked(&version)
            .map_err(|error| failure(&logical, error))?
            .ok_or_else(|| missing(&logical.join(lookup::LEGACY_SPECS_ROOT), document))?;
        Ok((package, logical))
    }
}

fn locate_in(
    project: &Project,
    package: &Pinned,
    logical: &Path,
    document: &str,
    formats: &[String],
) -> Result<(Pinned, ResolvedSourceFile, PackagePolicyCheckpoint), ResolveError> {
    let policy =
        observe_package_policy_in(project, package).map_err(|error| failure(logical, error))?;
    match policy.mode() {
        PackageSourceMode::ExplicitEmpty => return Err(missing(logical, document)),
        PackageSourceMode::Explicit => {
            return Err(failure(logical, policy.unsupported("spec address lookup")));
        }
        PackageSourceMode::Legacy | PackageSourceMode::MissingManifest => {}
    }
    let (specs, base) = layout(
        project,
        package,
        logical,
        lookup::NEW_SPECS_ROOT,
        lookup::LEGACY_SPECS_ROOT,
    )?
    .ok_or_else(|| missing(&logical.join(lookup::LEGACY_SPECS_ROOT), document))?;
    let (parents, last) = document
        .rsplit_once('/')
        .map_or(("", document), |(parents, last)| (parents, last));
    let mut parent = specs;
    let mut directory = base.clone();
    if !parents.is_empty() {
        for component in parents.split('/') {
            directory.push(component);
            parent = parent
                .open_child_checked(component)
                .map_err(|error| failure(&directory, error))?
                .ok_or_else(|| missing(&base, document))?;
        }
    }
    let mut names = if lookup::is_id_stem(last) {
        project
            .child_names(&parent)
            .map_err(|error| failure(&directory, error))?
    } else {
        ["md", "xml"]
            .into_iter()
            .chain(formats.iter().map(String::as_str))
            .map(|format| format!("{last}.{format}"))
            .collect()
    };
    names.sort();
    names.dedup();
    let mut candidates = Vec::new();
    for name in names {
        let candidate = if lookup::is_id_stem(last) {
            lookup::id_file_matches_with_formats(Path::new(&name), last, formats)
        } else {
            ["md", "xml"]
                .into_iter()
                .chain(formats.iter().map(String::as_str))
                .any(|format| name == format!("{last}.{format}"))
        };
        if candidate
            && project
                .source_file_present_in(&parent, &name)
                .map_err(|error| failure(&directory.join(&name), error))?
        {
            candidates.push(directory.join(name));
        }
    }
    let selected = lookup::select_doc(&base, document, formats, &candidates)?;
    policy
        .recheck_unchanged()
        .map_err(|error| failure(logical, error))?;
    Ok((
        parent,
        ResolvedSourceFile {
            path: selected.path,
            format: if matches!(selected.extension.as_str(), "md" | "xml") {
                "markdown".into()
            } else {
                selected.extension
            },
            physical_stem: selected.physical_stem,
        },
        policy,
    ))
}

fn admit(logical: &Path) -> Result<(Project, Pinned, PathBuf), ResolveError> {
    // Only caller-supplied authority is resolved here; never its descendants.
    let physical = logical
        .canonicalize()
        .map_err(|error| failure(logical, error))?;
    let project = Project::open(&physical).map_err(|error| failure(logical, error))?;
    let pinned = project
        .root_dir()
        .map_err(|error| failure(logical, error))?;
    Ok((project, pinned, logical.to_path_buf()))
}

fn layout(
    project: &Project,
    parent: &Pinned,
    logical: &Path,
    current: &str,
    legacy: &str,
) -> Result<Option<(Pinned, PathBuf)>, ResolveError> {
    let mut chain = current.split('/');
    let first = chain
        .next()
        .ok_or_else(|| failure(logical, "empty layout"))?;
    let mut pinned = layout_child(project, parent, first, &logical.join(first))?;
    for component in chain {
        pinned = match pinned {
            Some(directory) => {
                layout_child(project, &directory, component, &logical.join(current))?
            }
            None => None,
        };
    }
    if let Some(directory) = pinned {
        return Ok(Some((directory, logical.join(current))));
    }
    Ok(parent
        .open_child_checked(legacy)
        .map_err(|error| failure(&logical.join(legacy), error))?
        .map(|directory| (directory, logical.join(legacy))))
}

fn layout_child(
    project: &Project,
    parent: &Pinned,
    name: &str,
    path: &Path,
) -> Result<Option<Pinned>, ResolveError> {
    if project
        .source_file_present_in(parent, name)
        .map_err(|error| failure(path, error))?
    {
        return Ok(None);
    }
    parent
        .open_child_checked(name)
        .map_err(|error| failure(path, error))
}

fn missing(base: &Path, document: &str) -> ResolveError {
    ResolveError::DocNotFound {
        doc_path: document.into(),
        base: base.display().to_string(),
    }
}

pub(crate) fn failure(path: &Path, reason: impl std::fmt::Display) -> ResolveError {
    ResolveError::SourceObservation {
        path: path.display().to_string(),
        reason: format!("{reason:#}"),
    }
}
