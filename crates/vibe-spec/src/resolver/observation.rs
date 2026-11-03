//! A held lookup/read epoch; logical paths remain the exact overlay keys.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-OBSERVED-SOURCE-READ"
);
use super::*;
use vibe_safefs::{Pinned, Project};

pub(crate) struct SourceLocation {
    project: Project,
    parent: Pinned,
    name: String,
    pub(crate) resolved: ResolvedSourceFile,
}

impl SourceLocation {
    pub(crate) fn read(&self) -> Result<Vec<u8>, ResolveError> {
        self.project
            .read_source_bytes_in(&self.parent, &self.name)
            .map_err(|error| failure(&self.resolved.path, error))?
            .ok_or_else(|| {
                failure(
                    &self.resolved.path,
                    "selected source disappeared before reading",
                )
            })
    }
}

impl FileResolver {
    pub(crate) fn locate_source(
        &self,
        address: &SpecAddress,
        formats: &[String],
    ) -> Result<SourceLocation, ResolveError> {
        paths::source(&address.doc_path, formats)?;
        if glob::is_pattern(address) {
            return Err(ResolveError::PatternNotExpanded {
                given: address.raw.clone(),
            });
        }
        let (project, package, logical) = self.observed_root(address)?;
        let (specs, base) = layout(
            &project,
            &package,
            &logical,
            lookup::NEW_SPECS_ROOT,
            lookup::LEGACY_SPECS_ROOT,
        )?
        .ok_or_else(|| missing(&logical.join(lookup::LEGACY_SPECS_ROOT), &address.doc_path))?;
        let (parents, last) = address
            .doc_path
            .rsplit_once('/')
            .map_or(("", address.doc_path.as_str()), |(parents, last)| {
                (parents, last)
            });
        let mut parent = specs;
        let mut directory = base.clone();
        if !parents.is_empty() {
            for component in parents.split('/') {
                directory.push(component);
                parent = parent
                    .open_child_checked(component)
                    .map_err(|error| failure(&directory, error))?
                    .ok_or_else(|| missing(&base, &address.doc_path))?;
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
        let selected = lookup::select_doc(&base, &address.doc_path, formats, &candidates)?;
        let name = selected
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| failure(&selected.path, "selected filename unavailable"))?
            .to_owned();
        Ok(SourceLocation {
            project,
            parent,
            name,
            resolved: ResolvedSourceFile {
                path: selected.path,
                format: if matches!(selected.extension.as_str(), "md" | "xml") {
                    "markdown".into()
                } else {
                    selected.extension
                },
                physical_stem: selected.physical_stem,
            },
        })
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
                    return admit(&chosen.root);
                }
                let component = paths::package_component(group, name, version.as_deref())?;
                let (project, root, logical) = admit(&self.ws_root)?;
                let absent = || ResolveError::PackageSlotNotFound(name.clone());
                let (deps, logical) = layout(
                    &project,
                    &root,
                    &logical,
                    lookup::NEW_VIBEDEPS_ROOT,
                    lookup::LEGACY_VIBEDEPS_ROOT,
                )?
                .ok_or_else(absent)?;
                let logical = logical.join(&component);
                let package = deps
                    .open_child_checked(&component)
                    .map_err(|error| failure(&logical, error))?
                    .ok_or_else(absent)?;
                let version = match version {
                    Some(version) => version.clone(),
                    None => {
                        let mut versions = Vec::new();
                        for entry in project
                            .child_names(&package)
                            .map_err(|error| failure(&logical, error))?
                        {
                            if entry.starts_with(|character: char| character.is_ascii_digit())
                                && !project
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
                    .ok_or_else(|| {
                        missing(&logical.join(lookup::LEGACY_SPECS_ROOT), &address.doc_path)
                    })?;
                Ok((project, package, logical))
            }
        }
    }
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
