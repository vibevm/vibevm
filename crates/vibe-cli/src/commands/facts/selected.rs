//! Lock-selected, original-byte admission for declared facts adoption.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-OBSERVATION-LAW");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{Lockfile, Manifest, SpecFormat};
use vibe_facts::{AuthoredFact, CapturedDeclaredDocument, SourceKind};
use vibe_safefs::{Pinned, Project};
use vibe_spec::source_policy::{
    PackagePolicyCheckpoint, PackageSourceMode, observe_package_policy_in,
};
use vibe_workspace::{Workspace, vibedeps};

pub(super) struct Adoption {
    facts: Vec<AuthoredFact>,
    declared: Option<DeclaredAdmission>,
}

struct DeclaredAdmission {
    project: Project,
    slot: Pinned,
    root_policy: PackagePolicyCheckpoint,
    policy: PackagePolicyCheckpoint,
    lock: Vec<u8>,
    record: Vec<u8>,
    documents: Vec<(String, Vec<u8>)>,
    registry_root: PathBuf,
    registry_directory: Pinned,
    registry_file: String,
}

impl Adoption {
    pub(super) fn prepare(root: &Path, package: &str) -> Result<Self> {
        let workspace = Workspace::discover(root)?;
        // Admit only the caller's workspace. A lock-derived slot is always a
        // no-follow descendant, never canonicalized into a fresh authority.
        let project = Project::open(&workspace.root)?;
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
        if matches!(
            policy.mode(),
            PackageSourceMode::Legacy | PackageSourceMode::MissingManifest
        ) {
            // Removing a declaration from a recorded package must not turn a
            // changed manifest into legacy authority. Unrecorded old slots
            // retain their historical compatibility path.
            if let Some(bytes) =
                project.read_source_bytes_in(&slot, vibedeps::SLOT_RECORD_FILENAME)?
            {
                let record = vibedeps::read_slot_record_bytes(
                    &slot.path().join(vibedeps::SLOT_RECORD_FILENAME),
                    &bytes,
                )
                .map_err(anyhow::Error::msg)?;
                verify_recorded(
                    &record,
                    Manifest::FILENAME,
                    policy
                        .manifest_bytes()
                        .context("recorded package manifest is missing")?,
                )?;
            }
            // No-descriptor compatibility stays the incumbent scanner/rederive.
            let facts = vibe_facts::scan_authored_facts(slot.path(), package, SourceKind::Package)?;
            policy.recheck_unchanged()?;
            return Ok(Self {
                facts,
                declared: None,
            });
        }
        if selected.materialization.is_in_place() {
            bail!("declared facts adopt does not support in-place source provenance");
        }
        let meta = policy
            .package()
            .context("declared package metadata is unavailable")?;
        if meta.group != selected.group
            || meta.name != selected.name
            || meta.version != selected.version
            || meta.kind != selected.kind
        {
            bail!(
                "declared facts package identity differs from lock-selected kind/group/name/version"
            );
        }
        let record = project
            .read_source_bytes_in(&slot, vibedeps::SLOT_RECORD_FILENAME)?
            .context("declared facts adopt requires an original slot record")?;
        let parsed_record = vibedeps::read_slot_record_bytes(
            &slot.path().join(vibedeps::SLOT_RECORD_FILENAME),
            &record,
        )
        .map_err(anyhow::Error::msg)?;
        if parsed_record.spec_format != SpecFormat::Mixed {
            bail!("declared facts adopt requires an unchanged Mixed slot record");
        }
        if parsed_record.source_hash != selected.content_hash {
            bail!("facts slot source hash differs from lock-selected source hash");
        }
        verify_recorded(
            &parsed_record,
            Manifest::FILENAME,
            policy
                .manifest_bytes()
                .context("declared manifest disappeared")?,
        )?;
        let paths = vibe_facts::exact_declared_files(meta)
            .map_err(|reason| anyhow::anyhow!("declared facts census refused: {reason:?}"))?;
        let mut documents = Vec::with_capacity(paths.len());
        for path in paths {
            let bytes = project
                .read_source_bytes_in(&slot, &path)?
                .with_context(|| format!("declared source `{path}` is missing"))?;
            verify_recorded(&parsed_record, &path, &bytes)?;
            documents.push((path, bytes));
        }
        let captured = documents
            .iter()
            .map(|(path, bytes)| CapturedDeclaredDocument {
                path,
                bytes: Some(bytes),
            })
            .collect::<Vec<_>>();
        let observed = vibe_facts::observe_declared_source(meta, &captured)
            .map_err(|reason| anyhow::anyhow!("declared source walk refused: {reason:?}"))?;
        if !observed.complete {
            let failures = observed
                .documents
                .iter()
                .filter(|(_, row)| {
                    !matches!(
                        row,
                        vibe_facts::DeclaredDocumentObservation::Facts { .. }
                            | vibe_facts::DeclaredDocumentObservation::Raw { .. }
                            | vibe_facts::DeclaredDocumentObservation::Unclassified { .. }
                    )
                })
                .map(|(path, _)| path.as_str())
                .collect::<Vec<_>>();
            bail!(
                "declared source observation incomplete at {failures:?}; facts adopt made no registry change"
            );
        }
        let facts = observed.facts;
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
    pub(super) fn mixed_noop(&self) -> bool {
        self.declared.is_some()
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
        if super::super::reinstall::effective_rederive_format(&manifest)? != SpecFormat::Mixed {
            bail!(
                "declared facts adopt requires effective Mixed rederive; configuration selects a transformed target"
            );
        }
        // The registry destination stays below this same admitted workspace.
        // Existing path components may not redirect writes through links.
        let target = self
            .registry_root
            .join(vibe_core::layout::current_vibefacts_root())
            .join(&self.registry_file);
        vibe_safefs::ensure_no_follow_walk(self.project.root_path(), &target, true)?;
        Ok(())
    }
}

fn verify_recorded(record: &vibedeps::SlotRecord, path: &str, bytes: &[u8]) -> Result<()> {
    let row = record
        .files
        .iter()
        .find(|row| row.path == path)
        .with_context(|| format!("slot record has no declared source `{path}`"))?;
    if vibe_workspace::vibedeps::sha256_bytes(bytes) != row.sha256 {
        bail!("declared source `{path}` differs from recorded slot payload");
    }
    // Editable hashes establish consistency with the selected record only;
    // they do not authenticate an original publisher or hostile replacements.
    Ok(())
}

#[cfg(test)]
mod tests;
