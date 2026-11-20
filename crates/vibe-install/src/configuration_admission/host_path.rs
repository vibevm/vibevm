//! Install-only metadata readiness; history never stores a host-ready assertion.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PARAMETERS-ARE-VALUES");
use super::*;
use vibe_core::manifest::PackageConfiguration;

pub(crate) fn unsupported() -> Error {
    vibe_core::manifest::PackageConfigurationError::Unsupported {
        package: "ordinary host path".into(),
        reason: "public path must_exist=true requires a separately specified planning-host base and check; this operation has no install-owned PathCheck".into(),
    }.into()
}

#[derive(Debug)]
pub(crate) struct PathCheck {
    root: vibe_safefs::Project,
    lexical: String,
    parents: Vec<vibe_safefs::FileIdentity>,
    _held_parents: Vec<vibe_safefs::Pinned>,
    _held_target_directory: Option<vibe_safefs::Pinned>,
    directory: bool,
}
impl PathCheck {
    pub(crate) fn recheck(&self) -> Result<()> {
        let current = observe(self.root.root_path(), &self.lexical)?;
        if current.root.root_identity().map_err(|_| unsupported())?
            != self.root.root_identity().map_err(|_| unsupported())?
            || current.parents != self.parents
            || current.directory != self.directory
        {
            return Err(unsupported());
        }
        Ok(())
    }
}
pub(super) fn observe(root: &Path, lexical: &str) -> Result<PathCheck> {
    #[cfg(not(unix))]
    {
        let _ = (root, lexical);
        return Err(unsupported());
    }
    #[cfg(unix)]
    {
        // Errors stay static: do not disclose a resolved target or external path.
        let (parents, name) = vibe_safefs::split_relative(lexical).map_err(|_| unsupported())?;
        let project = vibe_safefs::Project::open(root).map_err(|_| unsupported())?;
        let mut parent = project.root_dir().map_err(|_| unsupported())?;
        let mut identities = vec![parent.identity().map_err(|_| unsupported())?];
        let mut held_parents = Vec::new();
        for component in parents {
            let child = parent
                .open_child_checked(&component)
                .map_err(|_| unsupported())?
                .ok_or_else(unsupported)?;
            held_parents.push(parent);
            parent = child;
            identities.push(parent.identity().map_err(|_| unsupported())?);
        }
        let target_directory = if project
            .source_file_present_in(&parent, &name)
            .map_err(|_| unsupported())?
        {
            None
        } else {
            Some(
                parent
                    .open_child_checked(&name)
                    .map_err(|_| unsupported())?
                    .ok_or_else(unsupported)?,
            )
        };
        let directory = target_directory.is_some();
        held_parents.push(parent);
        Ok(PathCheck {
            root: project,
            lexical: lexical.into(),
            parents: identities,
            _held_parents: held_parents,
            _held_target_directory: target_directory,
            directory,
        })
    }
}

pub(crate) fn capture<'a>(
    workspace: &Workspace,
    selected: &Path,
    manifest: &Manifest,
    records: impl IntoIterator<Item = (&'a str, &'a PackageConfiguration)>,
) -> Result<Vec<PathCheck>> {
    let records = records.into_iter().collect::<Vec<_>>();
    let needs = records
        .iter()
        .filter(|(_, r)| !r.host_path_targets().is_empty())
        .collect::<Vec<_>>();
    if needs.is_empty() {
        return Ok(vec![]);
    }
    if needs.len() != 1
        || records.len() != 1
        || !workspace.is_standalone()
        || workspace.root != selected
        || manifest.workspace.is_some()
        || manifest.requires.packages.len() != 1
        || !manifest.requires.packages[0].version.is_exact_pin()
        || !manifest.requires.git_packages.is_empty()
        || !manifest.requires.path_packages.is_empty()
        || !manifest.requires.var_packages.is_empty()
        || !manifest.requires.remaps.is_empty()
        || manifest.requires.optional.values().any(|v| *v)
    {
        return Err(unsupported());
    }
    let (key, record) = needs[0];
    if manifest.requires.packages[0].qualified_name() != *key
        || record.consumers.len() != 1
        || !record.consumers.contains_key("./vibe.toml")
        || !record.submitted_inputs.is_empty()
        || !record.forwarded_inputs.is_empty()
        || record.automatic_basis.is_some()
        || !record.definition.groups.is_empty()
        || !record.definition.features.is_empty()
        || !record.definition.profiles.is_empty()
        || record.definition.parameters.len() != 1
        || record.host_path_targets().len() != 1
    {
        return Err(unsupported());
    }
    Ok(vec![observe(selected, record.host_path_targets()[0])?])
}

pub(crate) fn check_locked_sources(lock: &vibe_core::manifest::Lockfile) -> Result<()> {
    for (key, record) in &lock.configurations {
        if record.host_path_targets().is_empty() {
            continue;
        }
        let row = lock
            .packages
            .iter()
            .find(|p| format!("{}/{}", p.group, p.name) == *key)
            .ok_or_else(unsupported)?;
        if row.kind != vibe_core::PackageKind::Flow
            || row.materialization != vibe_core::manifest::Materialization::Copy
            || row.registry.is_none()
            || row.source_kind != Some(vibe_core::manifest::SourceKind::Registry)
            || !row.source_url.as_str().starts_with("file://")
            || row.overridden
            || row.via_override.is_some()
            || row.via_redirect.is_some()
            || row.template.is_some()
        {
            return Err(unsupported());
        }
    }
    Ok(())
}

/// The existing Fresh install owner has the actual root and selected world.
/// This is not a general admission bypass for update/reinstall or arbitrary IO.
pub fn resolved_for_install(
    workspace: &Workspace,
    selected: &Path,
    manifest: &Manifest,
    packages: &[vibe_workspace::install::ResolvedDep],
    lock: Option<&vibe_core::manifest::Lockfile>,
) -> Result<()> {
    let has_host = packages.iter().any(|p| {
        p.configuration
            .as_ref()
            .is_some_and(|r| !r.host_path_targets().is_empty())
    });
    if has_host {
        let lock = lock.ok_or_else(unsupported)?;
        check_locked_sources(lock)?;
        for p in packages.iter().filter(|p| {
            p.configuration
                .as_ref()
                .is_some_and(|r| !r.host_path_targets().is_empty())
        }) {
            let row = lock.find(&p.group, &p.name).ok_or_else(unsupported)?;
            if p.version != row.version
                || p.source_hash.as_ref() != Some(&row.content_hash)
                || p.configuration.as_ref()
                    != lock.configurations.get(&format!("{}/{}", p.group, p.name))
                || !manifest
                    .requires
                    .packages
                    .first()
                    .is_some_and(|r| r.version.matches(&row.version))
            {
                return Err(unsupported());
            }
        }
    }
    resolved_checked(workspace, selected, manifest, packages)
}

pub(crate) fn resolved_checked(
    workspace: &Workspace,
    selected: &Path,
    manifest: &Manifest,
    packages: &[vibe_workspace::install::ResolvedDep],
) -> Result<()> {
    let rows = packages
        .iter()
        .filter_map(|p| {
            p.configuration
                .as_ref()
                .map(|r| (format!("{}/{}", p.group, p.name), r))
        })
        .collect::<Vec<_>>();
    for p in packages {
        if let Some(record) = &p.configuration {
            record.validate(&p.manifest)?;
            if !record.host_path_targets().is_empty()
                && (p.kind != vibe_core::PackageKind::Flow
                    || !p
                        .manifest
                        .require_package()?
                        .materialization
                        .eq(&vibe_core::manifest::Materialization::Copy))
            {
                return Err(unsupported());
            }
        } else {
            package_manifest(
                &vibe_core::PackageRef::parse(&format!("{}/{}", p.group, p.name))?,
                &p.manifest,
            )?;
        }
    }
    let checks = capture(
        workspace,
        selected,
        manifest,
        rows.iter().map(|(k, r)| (k.as_str(), *r)),
    )?;
    for check in checks {
        check.recheck()?;
    }
    Ok(())
}
