//! Lock-selected installed observation and retained adoption admission.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-OBSERVATION-LAW");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use std::sync::Arc;
use vibe_core::manifest::{Lockfile, Manifest, SpecFormat};
use vibe_facts::{AuthoredFact, AuthoredSourceObservation, SourceKind};
use vibe_safefs::{Pinned, Project};
use vibe_spec::source_policy::{
    PackagePolicyCheckpoint, PackageSourceMode, observe_package_policy_in,
};
use vibe_spec::{FileResolver, InstalledSourceExpectation, SelectedPackage, SelfCoordinate};
use vibe_workspace::{Workspace, vibedeps};

pub(super) struct Adoption {
    facts: Vec<AuthoredFact>,
    declared: Option<DeclaredAdmission>,
}

struct DeclaredAdmission {
    project: Arc<Project>,
    slot: Pinned,
    root_policy: PackagePolicyCheckpoint,
    policy: PackagePolicyCheckpoint,
    lock: Vec<u8>,
    record: Vec<u8>,
    documents: Vec<(String, Vec<u8>)>,
    registry_root: PathBuf,
    registry_directory: Pinned,
    registry_file: String,
    format: SpecFormat,
}

impl Adoption {
    pub(super) fn prepare(root: &Path, package: &str) -> Result<Self> {
        let workspace = Workspace::discover(root)?;
        // Admit only the caller's workspace. A lock-derived slot is always a
        // no-follow descendant, never canonicalized into a fresh authority.
        let project = Arc::new(Project::open(&workspace.root)?);
        let root_dir = project.root_dir()?;
        let root_policy = observe_package_policy_in(&project, &root_dir)?;
        let lock = project
            .read_source_bytes(Lockfile::FILENAME)?
            .context("facts adopt requires an installed lockfile")?;
        let parsed = Lockfile::parse_str(std::str::from_utf8(&lock)?)?;
        let mut matches = parsed
            .packages
            .iter()
            .filter(|row| format!("{}/{}", row.group, row.name) == package);
        let selected = matches
            .next()
            .context("package has no lock-selected installed slot")?;
        if matches.next().is_some() {
            bail!("facts adopt has ambiguous lock-selected package `{package}`");
        }
        let relative = if selected.materialization.is_in_place() {
            vibedeps::in_place_slot_rel_path(&selected.group, &selected.name)
        } else {
            vibedeps::slot_rel_path(&selected.group, &selected.name, &selected.version)
        };
        let slot = project.dir(&relative.split('/').collect::<Vec<_>>(), false)?;
        let policy = observe_package_policy_in(&project, &slot)?;
        let legacy = matches!(
            policy.mode(),
            PackageSourceMode::Legacy | PackageSourceMode::MissingManifest
        );
        if selected.materialization.is_in_place() {
            if !legacy {
                bail!("declared facts adopt does not support in-place source provenance");
            }
            return Ok(Self {
                facts: vibe_facts::scan_authored_facts(slot.path(), package, SourceKind::Package)?,
                declared: None,
            });
        }
        let format = if legacy {
            SpecFormat::Mixed
        } else {
            super::super::reinstall::effective_rederive_format(&workspace.root_manifest)?
        };
        let expected = InstalledSourceExpectation {
            kind: selected.kind,
            group: selected.group.clone(),
            name: selected.name.to_string(),
            version: selected.version.to_string(),
            source_hash: selected.content_hash.clone(),
            materialized_format: format,
        };
        let binding = vibedeps::prepare_declared_query_read(&project, &slot, &relative, &expected)?;
        let Some(binding) = binding else {
            let facts = vibe_facts::scan_authored_facts(slot.path(), package, SourceKind::Package)?;
            policy.recheck_unchanged()?;
            return Ok(Self {
                facts,
                declared: None,
            });
        };
        let record = project
            .read_source_bytes_in(&slot, vibedeps::SLOT_RECORD_FILENAME)?
            .context("declared facts requires a slot record")?;
        let mut documents = Vec::new();
        for row in binding.outputs() {
            let bytes = project
                .read_source_bytes_in(&slot, &row.output_path)?
                .with_context(|| format!("installed output `{}` is absent", row.output_path))?;
            if vibedeps::sha256_bytes(&bytes) != row.sha256 {
                bail!(
                    "installed output `{}` changed during facts capture",
                    row.output_path
                );
            }
            documents.push((row.output_path.clone(), bytes));
        }
        let world = parsed
            .packages
            .iter()
            .map(|locked| {
                let path = if locked.materialization.is_in_place() {
                    vibedeps::in_place_slot_abs_path(&workspace.root, &locked.group, &locked.name)
                } else {
                    vibedeps::slot_abs_path(
                        &workspace.root,
                        &locked.group,
                        &locked.name,
                        &locked.version,
                    )
                };
                (
                    (locked.group.to_string(), locked.name.to_string()),
                    SelectedPackage::new(locked.version.to_string(), path),
                )
            })
            .collect();
        let self_coordinate = if let Some(host) = &workspace.root_manifest.project {
            SelfCoordinate::new(
                host.group.as_ref().map(ToString::to_string),
                host.name.clone(),
            )
        } else if let Some(host) = &workspace.root_manifest.package {
            SelfCoordinate::new(Some(host.group.to_string()), host.name.to_string())
        } else {
            SelfCoordinate::new(None, String::new())
        };
        let resolver = FileResolver::with_selected_world(
            &workspace.root,
            &workspace.root,
            world,
            self_coordinate,
        )
        .with_installed_source_bindings(project.clone(), vec![binding], None)?;
        let facts = match vibe_facts::observe_installed_source(
            &resolver,
            expected.group.as_str(),
            &expected.name,
        )? {
            AuthoredSourceObservation::Available { facts, .. } => facts,
            AuthoredSourceObservation::Invalid { issue, .. } => bail!(
                "invalid declared native source `{}`: {}",
                issue.path.display(),
                issue.message
            ),
        };
        let registry_root = root.to_path_buf();
        vibe_safefs::ensure_lexically_contained(project.root_path(), &registry_root)?;
        let registry_components = registry_root
            .strip_prefix(project.root_path())?
            .components()
            .map(|part| {
                part.as_os_str()
                    .to_str()
                    .context("registry directory is not UTF-8")
            })
            .collect::<Result<Vec<_>>>()?;
        let registry_directory = project.dir(&registry_components, false)?;
        let admission = DeclaredAdmission {
            project,
            slot,
            root_policy,
            policy,
            lock,
            record,
            documents,
            format,
            registry_root,
            registry_directory,
            registry_file: format!("{}.{}.toml", selected.group, selected.name),
        };
        admission.recheck()?;
        Ok(Self {
            facts,
            declared: Some(admission),
        })
    }

    pub(super) fn facts(&self) -> &[AuthoredFact] {
        &self.facts
    }
    pub(super) fn is_declared(&self) -> bool {
        self.declared.is_some()
    }
    pub(super) fn mixed_noop(&self) -> bool {
        self.declared
            .as_ref()
            .is_some_and(|admission| admission.format == SpecFormat::Mixed)
    }
    pub(super) fn recheck(&self) -> Result<()> {
        if let Some(admission) = &self.declared {
            admission.recheck()?;
        }
        Ok(())
    }
}

impl DeclaredAdmission {
    fn recheck(&self) -> Result<()> {
        self.root_policy.recheck_unchanged()?;
        self.policy.recheck_unchanged()?;
        let components = self
            .registry_root
            .strip_prefix(self.project.root_path())?
            .components()
            .map(|part| {
                part.as_os_str()
                    .to_str()
                    .context("registry directory is not UTF-8")
            })
            .collect::<Result<Vec<_>>>()?;
        if self.project.dir(&components, false)?.identity()?
            != self.registry_directory.identity()?
        {
            bail!("facts registry destination changed before adoption");
        }
        if self
            .project
            .read_source_bytes(Lockfile::FILENAME)?
            .as_deref()
            != Some(&self.lock)
        {
            bail!("lock-selected facts source changed before adoption");
        }
        if self
            .project
            .read_source_bytes_in(&self.slot, vibedeps::SLOT_RECORD_FILENAME)?
            .as_deref()
            != Some(&self.record)
        {
            bail!("facts slot record changed before adoption");
        }
        for (path, before) in &self.documents {
            if self
                .project
                .read_source_bytes_in(&self.slot, path)?
                .as_deref()
                != Some(before)
            {
                bail!("declared source `{path}` changed before adoption");
            }
        }
        let manifest = Manifest::parse_str(std::str::from_utf8(
            self.root_policy
                .manifest_bytes()
                .context("workspace manifest is missing")?,
        )?)?;
        if super::super::reinstall::effective_rederive_format(&manifest)? != self.format {
            bail!("facts selected materialized format changed before adoption");
        }
        // The registry destination stays below this same admitted workspace.
        // Existing path components may not redirect writes through links.
        let target = self
            .registry_root
            .join(vibe_core::layout::current_vibefacts_root())
            .join(&self.registry_file);
        vibe_safefs::ensure_no_follow_walk(self.project.root_path(), &target, true)?;
        let relative = vibe_core::machine_json_path(target.strip_prefix(self.project.root_path())?);
        self.project
            .read_source_bytes(&relative)
            .context("facts registry destination is not an available regular file")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
