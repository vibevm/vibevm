//! Independently selected slot-root observations for progress.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-PROGRESS-READ",
    r = 1
);
use super::{Text, project, refused};
use anyhow::Result;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use vibe_core::manifest::{LockedPackage, Lockfile, Manifest, SourceInterpretation, SpecFormat};
use vibe_safefs::{Project, SourceObservation};
use vibe_spec::source_policy::{
    PackagePolicyObservation, PackageSourceMode, observe_package_policy,
};
use vibe_spec::{
    FileResolver, InstalledSourceExpectation, PreparedOutputRow, SelectedPackage, SelfCoordinate,
};

pub(super) struct Installed {
    root: PathBuf,
    workspace: Arc<Project>,
    policy: PackagePolicyObservation,
    lock: SourceObservation,
    package: LockedPackage,
    format: SpecFormat,
    resolver: Option<FileResolver>,
    record: Option<SourceObservation>,
    relative: String,
    outputs: Vec<PreparedOutputRow>,
    sources: Vec<String>,
}
impl Installed {
    pub(super) fn select(
        root: &Path,
        root_policy: &PackagePolicyObservation,
    ) -> Result<Option<Self>> {
        let recorded = root_policy.materialized_record_present()?;
        let declared = matches!(
            root_policy.mode(),
            PackageSourceMode::Explicit | PackageSourceMode::ExplicitEmpty
        );
        if !recorded && !declared {
            return Ok(None);
        }
        let Some(owner) = root
            .ancestors()
            .skip(1)
            .find(|p| root.starts_with(p.join(vibe_core::layout::current_vibedeps_root())))
        else {
            return if recorded {
                Err(refused(
                    "recorded corpus has no independent installed context",
                ))
            } else {
                Ok(None)
            };
        };
        let workspace = Arc::new(Project::open(owner)?);
        let policy = observe_package_policy(owner)?;
        let manifest = policy
            .manifest()
            .ok_or_else(|| refused("installed corpus has no containing workspace manifest"))?;
        let format = crate::commands::reinstall::effective_rederive_format(manifest)?;
        let lock = workspace
            .read_source_observation(Lockfile::FILENAME)?
            .ok_or_else(|| refused("installed corpus has no independent lock"))?;
        let parsed = Lockfile::parse_str(std::str::from_utf8(lock.bytes())?)?;
        let matching = parsed
            .packages
            .iter()
            .filter(|p| {
                owner.join(vibe_workspace::vibedeps::slot_rel_path(
                    &p.group, &p.name, &p.version,
                )) == root
            })
            .collect::<Vec<_>>();
        let [package] = matching.as_slice() else {
            return Err(refused(
                "installed corpus has missing or ambiguous locked owner",
            ));
        };
        let package = (*package).clone();
        if package.materialization.is_in_place() {
            return Err(refused("declared in-place corpus has no binding"));
        }
        let relative = vibe_core::machine_json_path(root.strip_prefix(owner)?);
        let slot = workspace.dir(&relative.split('/').collect::<Vec<_>>(), false)?;
        let binding = vibe_workspace::vibedeps::prepare_declared_query_read(
            &workspace,
            &slot,
            &relative,
            &InstalledSourceExpectation {
                kind: package.kind,
                group: package.group.clone(),
                name: package.name.to_string(),
                version: package.version.to_string(),
                source_hash: package.content_hash.clone(),
                materialized_format: format,
            },
        )?;
        let record = workspace.read_source_observation(&format!(
            "{relative}/{}",
            vibe_workspace::vibedeps::SLOT_RECORD_FILENAME
        ))?;
        let outputs = binding
            .as_ref()
            .map(|b| b.outputs().to_vec())
            .unwrap_or_default();
        let mut selected = BTreeMap::new();
        selected.insert(
            (package.group.to_string(), package.name.to_string()),
            SelectedPackage::new(package.version.to_string(), root.to_owned()),
        );
        let node = manifest.consumer_node();
        let coordinate = SelfCoordinate::new(
            node.as_ref()
                .and_then(|n| n.group.as_ref().map(ToString::to_string)),
            node.map(|n| n.name).unwrap_or_default(),
        );
        let resolver = binding
            .map(|binding| {
                FileResolver::with_selected_world(owner, owner, selected, coordinate)
                    .with_installed_source_bindings(workspace.clone(), vec![binding], None)
            })
            .transpose()?;
        Ok(Some(Self {
            root: root.to_owned(),
            workspace,
            policy,
            lock,
            package,
            format,
            resolver,
            record,
            relative,
            outputs,
            sources: Vec::new(),
        }))
    }

    pub(super) fn is_declared(&self) -> bool {
        self.resolver.is_some()
    }

    pub(super) fn check_selected(&self, cfg: &progress_core::scope::ScopeConfig) -> Result<()> {
        let Some(resolver) = &self.resolver else {
            return Ok(());
        };
        for row in &self.outputs {
            if progress_core::scope::includes_path(&self.root, Path::new(&row.output_path), cfg)?
                && native(row.interpretation)
            {
                resolver.read_installed_source(
                    self.package.group.as_str(),
                    &self.package.name,
                    &row.source_path,
                    None,
                )?;
            }
        }
        Ok(())
    }
    pub(super) fn read(&mut self, relative: &Path) -> Result<Option<Text>> {
        let resolver = self
            .resolver
            .as_ref()
            .ok_or_else(|| refused("legacy input requires its qualified reader"))?;
        let row = self
            .outputs
            .iter()
            .find(|r| Path::new(&r.output_path) == relative)
            .ok_or_else(|| {
                refused(format!(
                    "selected output `{}` is not recorded",
                    relative.display()
                ))
            })?;
        if !native(row.interpretation) {
            return Ok(None);
        }
        let interpretation = row
            .interpretation
            .ok_or_else(|| refused("selected native interpretation absent"))?;
        if interpretation == SourceInterpretation::VibeDoc
            && self.package.kind != vibe_core::PackageKind::Doc
        {
            return Err(refused("Doc vocabulary requires an actual Doc package"));
        }
        let bytes = resolver.read_installed_source(
            self.package.group.as_str(),
            &self.package.name,
            &row.source_path,
            None,
        )?;
        let text = project(&bytes, &row.media, interpretation)?;
        self.sources.push(row.source_path.clone());
        Ok(Some(Text {
            path: row.source_path.clone(),
            physical: row.output_path.clone(),
            text,
            xml: row.media == "application/xml",
        }))
    }
    pub(super) fn recheck(&self) -> Result<()> {
        self.policy.recheck_unchanged()?;
        let now = self.workspace.read_source_observation(Lockfile::FILENAME)?;
        if now.as_ref().map(|s| (s.identity(), s.bytes()))
            != Some((self.lock.identity(), self.lock.bytes()))
        {
            return Err(refused("independent installed lock changed"));
        }
        let manifest: &Manifest = self
            .policy
            .manifest()
            .ok_or_else(|| refused("workspace manifest disappeared"))?;
        if crate::commands::reinstall::effective_rederive_format(manifest)? != self.format {
            return Err(refused("selected installed format changed"));
        }
        let record = self.workspace.read_source_observation(&format!(
            "{}/{}",
            self.relative,
            vibe_workspace::vibedeps::SLOT_RECORD_FILENAME
        ))?;
        if record.as_ref().map(|s| (s.identity(), s.bytes()))
            != self.record.as_ref().map(|s| (s.identity(), s.bytes()))
        {
            return Err(refused("installed record changed"));
        }
        if let Some(resolver) = &self.resolver {
            resolver.recheck_installed_package(self.package.group.as_str(), &self.package.name)?;
            for path in &self.sources {
                resolver.read_installed_source(
                    self.package.group.as_str(),
                    &self.package.name,
                    path,
                    None,
                )?;
            }
        } else {
            let slot = self
                .workspace
                .dir(&self.relative.split('/').collect::<Vec<_>>(), false)?;
            let rebound = vibe_workspace::vibedeps::prepare_declared_query_read(
                &self.workspace,
                &slot,
                &self.relative,
                &InstalledSourceExpectation {
                    kind: self.package.kind,
                    group: self.package.group.clone(),
                    name: self.package.name.to_string(),
                    version: self.package.version.to_string(),
                    source_hash: self.package.content_hash.clone(),
                    materialized_format: self.format,
                },
            )?;
            if rebound.is_some() {
                return Err(refused("legacy installed policy changed"));
            }
        }
        Ok(())
    }
}
fn native(interpretation: Option<SourceInterpretation>) -> bool {
    matches!(
        interpretation,
        Some(SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc)
    )
}
