//! Retained observations for the effective display's exact selected paths.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-SHOW-READ",
    r = 1
);
use anyhow::Result;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use vibe_core::manifest::{
    LockedPackage, Lockfile, Manifest, SelectedSource, SourceInterpretation, SpecFormat,
    select_source_rows,
};
use vibe_safefs::{Project, SourceObservation};
use vibe_spec::source_policy::{
    CapturedAuthoredBoot, PackagePolicyObservation, PackageSourceMode, observe_package_policy,
};
use vibe_spec::{FileResolver, InstalledSourceExpectation, SelectedPackage, SelfCoordinate};
use vibe_workspace::{Workspace, vibedeps};

fn refused(reason: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-SHOW-READ: {reason}; fix surface: selected display path, owner policy and independent installed binding"
    )
}
struct Host {
    policy: PackagePolicyObservation,
    sources: Vec<(String, SourceObservation)>,
}
struct Installed {
    resolver: FileResolver,
    sources: Vec<String>,
    empty: bool,
}
pub(super) enum Body {
    Text(String),
    Skipped,
    Missing,
}
pub(super) struct Reads<'a> {
    root: &'a Path,
    project: Arc<Project>,
    lock: Option<SourceObservation>,
    pub(super) packages: Vec<LockedPackage>,
    format: Option<SpecFormat>,
    hosts: BTreeMap<PathBuf, Host>,
    slots: BTreeMap<PathBuf, LockedPackage>,
    members: BTreeMap<PathBuf, Manifest>,
    workspace: Option<Manifest>,
    installed: BTreeMap<PathBuf, Installed>,
    boot: Option<CapturedAuthoredBoot>,
    files: Vec<(String, SourceObservation)>,
    legacy_guards: Vec<vibe_spec::source_policy::PackagePolicyCheckpoint>,
}
impl<'a> Reads<'a> {
    pub(super) fn new(root: &'a Path, format: Option<SpecFormat>) -> Result<Self> {
        let project = Arc::new(Project::open(root)?);
        let lock = project.read_source_observation(Lockfile::FILENAME)?;
        let packages = match &lock {
            Some(bytes) => Lockfile::parse_str(std::str::from_utf8(bytes.bytes())?)?.packages,
            None => Vec::new(),
        };
        let slots = packages
            .iter()
            .map(|p| {
                let relative = if p.materialization.is_in_place() {
                    vibedeps::in_place_slot_rel_path(&p.group, &p.name)
                } else {
                    vibedeps::slot_rel_path(&p.group, &p.name, &p.version)
                };
                (root.join(relative), p.clone())
            })
            .collect();
        let policy = observe_package_policy(root)?;
        let mut members = BTreeMap::new();
        let mut workspace = None;
        if policy.manifest().is_some_and(Manifest::is_workspace_root) {
            let ws = Workspace::load(root)?;
            if !same_manifest(policy.manifest(), &ws.root_manifest) {
                return Err(refused("workspace manifest changed during selection"));
            }
            workspace = Some(ws.root_manifest);
            for member in ws.members {
                members.insert(root.join(member.rel_path.as_str()), member.manifest);
            }
        }
        let mut hosts = BTreeMap::new();
        hosts.insert(
            root.to_owned(),
            Host {
                policy,
                sources: Vec::new(),
            },
        );
        Ok(Self {
            root,
            project,
            lock,
            packages,
            format,
            hosts,
            slots,
            members,
            workspace,
            installed: BTreeMap::new(),
            boot: None,
            files: Vec::new(),
            legacy_guards: Vec::new(),
        })
    }
    /// None selects the qualified legacy directory enumeration.
    pub(super) fn own_boot(&mut self) -> Result<Option<Vec<(String, String)>>> {
        let policy = &self.hosts[self.root].policy;
        if matches!(
            policy.mode(),
            PackageSourceMode::Legacy | PackageSourceMode::MissingManifest
        ) {
            return Ok(None);
        }
        let captured = policy.capture_authored_boot()?;
        let mut entries = Vec::new();
        for resource in captured.resources() {
            let body = vibe_spec::project_declared_native(
                resource.bytes(),
                resource.media(),
                SourceInterpretation::VibeSpec,
            )
            .map_err(refused)?;
            entries.push((resource.relative().to_owned(), body));
        }
        self.boot = Some(captured);
        Ok(Some(entries))
    }
    fn generated(&mut self, relative: &Path) -> Result<Body> {
        let path = vibe_core::machine_json_path(relative);
        let Some(observation) = self.project.read_source_observation(&path)? else {
            return Ok(Body::Missing);
        };
        let text = literal(observation.bytes(), &path)?;
        self.files.push((path, observation));
        Ok(Body::Text(text))
    }
    pub(super) fn read(&mut self, relative: &Path) -> Result<Body> {
        vibe_core::manifest::declarant_path(relative).map_err(|e| refused(e.reason()))?;
        let full = self.root.join(relative);
        if let Some((owner, package)) = self
            .slots
            .iter()
            .filter(|(p, _)| full.starts_with(p))
            .max_by_key(|(p, _)| p.components().count())
        {
            let owner = owner.clone();
            let package = package.clone();
            let path = full.strip_prefix(&owner)?;
            if vibe_core::layout::is_generated_boot_output(path) {
                return self.generated(relative);
            }
            return self.installed(&owner, &package, path);
        }
        if relative.starts_with(vibe_core::layout::current_vibedeps_root())
            || relative.starts_with(vibe_core::layout::current_packages_root())
        {
            return Err(refused(format!(
                "written path `{}` has no selected package owner",
                relative.display()
            )));
        }
        let owner = self
            .members
            .keys()
            .filter(|p| full.starts_with(p))
            .max_by_key(|p| p.components().count())
            .cloned()
            .unwrap_or_else(|| self.root.to_owned());
        let path = full.strip_prefix(&owner)?;
        if vibe_core::layout::is_generated_boot_output(path) {
            return self.generated(relative);
        }
        if !self.hosts.contains_key(&owner) {
            vibe_safefs::ensure_no_follow_walk(self.project.root_path(), &full, true)?;
            let policy = observe_package_policy(&owner)?;
            if !same_manifest(policy.manifest(), &self.members[&owner]) {
                return Err(refused("registered member manifest changed"));
            }
            self.hosts.insert(
                owner.clone(),
                Host {
                    policy,
                    sources: Vec::new(),
                },
            );
        }
        let host = self
            .hosts
            .get_mut(&owner)
            .ok_or_else(|| refused("selected owner absent"))?;
        match host.policy.mode() {
            PackageSourceMode::ExplicitEmpty => Ok(Body::Skipped),
            PackageSourceMode::Legacy | PackageSourceMode::MissingManifest => self.legacy(&full),
            PackageSourceMode::Explicit => {
                if host.policy.materialized_record_present()? {
                    return Err(refused(
                        "materialized original input requires its independently selected installed binding",
                    ));
                }
                let path = vibe_core::machine_json_path(path);
                let (interpretation, media) = match select_source_rows(
                    &owner.display().to_string(),
                    host.policy
                        .source_rows()
                        .ok_or_else(|| refused("source rows absent"))?,
                    &path,
                )
                .map_err(|e| refused(format!("{path}: {e:?}")))?
                {
                    SelectedSource::Descriptor { descriptor, .. } => (
                        Some(descriptor.interpretation()),
                        descriptor.media().to_owned(),
                    ),
                    SelectedSource::Unclassified { .. } => (None, String::new()),
                    SelectedSource::NeedsInput { .. } => {
                        return Err(refused(format!("ambiguous selected source `{path}`")));
                    }
                };
                let observation = host
                    .policy
                    .read_source_observation(&path)?
                    .ok_or_else(|| refused(format!("selected source `{path}` is missing")))?;
                let body = display(
                    observation.bytes(),
                    interpretation,
                    &media,
                    &path,
                    host.policy
                        .package()
                        .is_some_and(|p| p.kind == vibe_core::PackageKind::Doc),
                )?;
                host.sources.push((path, observation));
                Ok(Body::Text(body))
            }
        }
    }
    fn installed(&mut self, owner: &Path, package: &LockedPackage, path: &Path) -> Result<Body> {
        if !self.installed.contains_key(owner) && !self.hosts.contains_key(owner) {
            let relative = vibe_core::machine_json_path(owner.strip_prefix(self.root)?);
            // Missing legacy written files keep their diagnostic even when the slot is absent.
            if !owner.exists() {
                return self.legacy(&owner.join(path));
            }
            let slot = self
                .project
                .dir(&relative.split('/').collect::<Vec<_>>(), false)?;
            let policy = observe_package_policy(slot.path())?;
            if policy.mode() == PackageSourceMode::MissingManifest {
                return Err(refused("selected installed source has no manifest"));
            }
            let declared = matches!(
                policy.mode(),
                PackageSourceMode::Explicit | PackageSourceMode::ExplicitEmpty
            );
            if declared && package.materialization.is_in_place() {
                return Err(refused("declared in-place source has no installed binding"));
            }
            let format = match (declared, self.format) {
                (true, None) => {
                    return Err(refused(
                        "declared installed input requires independently selected materialized format",
                    ));
                }
                (_, Some(format)) => format,
                (false, None) => SpecFormat::Mixed,
            };
            let binding = if package.materialization.is_in_place() {
                None
            } else {
                vibedeps::prepare_declared_query_read(
                    &self.project,
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
                )?
            };
            if let Some(binding) = binding {
                let world = self
                    .slots
                    .iter()
                    .map(|(p, entry)| {
                        (
                            (entry.group.to_string(), entry.name.to_string()),
                            SelectedPackage::new(entry.version.to_string(), p.clone()),
                        )
                    })
                    .collect();
                let node = self.hosts[self.root]
                    .policy
                    .manifest()
                    .and_then(Manifest::consumer_node);
                let coordinate = SelfCoordinate::new(
                    node.as_ref()
                        .and_then(|n| n.group.as_ref().map(ToString::to_string)),
                    node.map(|n| n.name).unwrap_or_default(),
                );
                let resolver =
                    FileResolver::with_selected_world(self.root, self.root, world, coordinate)
                        .with_installed_source_bindings(
                            self.project.clone(),
                            vec![binding],
                            None,
                        )?;
                self.installed.insert(
                    owner.to_owned(),
                    Installed {
                        resolver,
                        sources: Vec::new(),
                        empty: policy.mode() == PackageSourceMode::ExplicitEmpty,
                    },
                );
            } else {
                if declared {
                    return Err(refused("declared source lost its installed binding"));
                }
                self.hosts.insert(
                    owner.to_owned(),
                    Host {
                        policy,
                        sources: Vec::new(),
                    },
                );
            }
        }
        let Some(installed) = self.installed.get_mut(owner) else {
            return self.legacy(&owner.join(path));
        };
        let row = installed
            .resolver
            .installed_source_binding(package.group.as_str(), &package.name)?
            .outputs()
            .iter()
            .find(|r| Path::new(&r.output_path) == path)
            .ok_or_else(|| {
                refused(format!(
                    "selected output `{}` is not recorded",
                    path.display()
                ))
            })?
            .clone();
        let bytes = installed.resolver.read_installed_source(
            package.group.as_str(),
            &package.name,
            &row.source_path,
            None,
        )?;
        if installed.empty {
            installed.sources.push(row.source_path);
            return Ok(Body::Skipped);
        }
        let body = display(
            &bytes,
            row.interpretation,
            &row.media,
            &row.source_path,
            package.kind == vibe_core::PackageKind::Doc,
        )?;
        installed.sources.push(row.source_path);
        Ok(Body::Text(body))
    }
    fn legacy(&mut self, full: &Path) -> Result<Body> {
        let Some(guards) = super::authored_guards(self.root, full)? else {
            return Ok(Body::Skipped);
        };
        self.legacy_guards.extend(guards);
        if !full.is_file() {
            return Ok(Body::Missing);
        }
        let relative = vibe_core::machine_json_path(full.strip_prefix(self.root)?);
        let observation = self
            .project
            .read_source_observation(&relative)?
            .ok_or_else(|| refused("selected legacy source disappeared"))?;
        let raw = literal(observation.bytes(), &relative)?;
        let (body, _) = vibe_specdoc::project_spec_text(full, &raw)?;
        self.files.push((relative, observation));
        Ok(Body::Text(body))
    }
    pub(super) fn recheck(&self) -> Result<()> {
        if let Some(expected) = &self.workspace {
            let ws = Workspace::load(self.root)?;
            let members: BTreeMap<_, _> = ws
                .members
                .into_iter()
                .map(|m| (self.root.join(m.rel_path.as_str()), m.manifest))
                .collect();
            if ws.root_manifest != *expected || members != self.members {
                return Err(refused("workspace ownership changed"));
            }
        }
        let lock = self.project.read_source_observation(Lockfile::FILENAME)?;
        if lock.as_ref().map(|s| (s.identity(), s.bytes()))
            != self.lock.as_ref().map(|s| (s.identity(), s.bytes()))
        {
            return Err(refused("lock changed during display"));
        }
        for host in self.hosts.values() {
            host.policy.recheck_unchanged()?;
            for (path, before) in &host.sources {
                let after = host
                    .policy
                    .read_source_observation(path)?
                    .ok_or_else(|| refused("selected source disappeared"))?;
                if before.identity() != after.identity() || before.bytes() != after.bytes() {
                    return Err(refused("selected source changed"));
                }
            }
        }
        for guard in &self.legacy_guards {
            guard.recheck_unchanged()?;
        }
        if let Some(boot) = &self.boot {
            self.hosts[self.root].policy.recheck_authored_boot(boot)?;
        }
        for (path, before) in &self.files {
            let after = self
                .project
                .read_source_observation(path)?
                .ok_or_else(|| refused("selected display file disappeared"))?;
            if before.identity() != after.identity() || before.bytes() != after.bytes() {
                return Err(refused("selected display file changed"));
            }
        }
        if let Some(format) = self.format
            && let Some(manifest) = self.hosts[self.root].policy.manifest()
            && crate::commands::reinstall::effective_rederive_format(manifest)? != format
        {
            return Err(refused("selected materialized format changed"));
        }
        for (owner, installed) in &self.installed {
            let p = &self.slots[owner];
            installed
                .resolver
                .recheck_installed_package(p.group.as_str(), &p.name)?;
            for path in &installed.sources {
                installed
                    .resolver
                    .read_installed_source(p.group.as_str(), &p.name, path, None)?;
            }
        }
        Ok(())
    }
}
fn same_manifest(observed: Option<&Manifest>, resolved: &Manifest) -> bool {
    let Some(observed) = observed else {
        return false;
    };
    let mut authored = observed.clone();
    if !authored.requires.var_packages.is_empty() {
        authored.requires = resolved.requires.clone();
    }
    authored == *resolved
}
fn literal(bytes: &[u8], path: &str) -> Result<String> {
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|e| refused(format!("literal display input `{path}` is not UTF-8: {e}")))
}
fn display(
    bytes: &[u8],
    interpretation: Option<SourceInterpretation>,
    media: &str,
    path: &str,
    doc: bool,
) -> Result<String> {
    match interpretation {
        Some(SourceInterpretation::VibeSpec) => {
            vibe_spec::project_declared_native(bytes, media, SourceInterpretation::VibeSpec)
                .map_err(refused)
        }
        Some(SourceInterpretation::VibeDoc) if doc => {
            vibe_spec::project_declared_native(bytes, media, SourceInterpretation::VibeDoc)
                .map_err(refused)
        }
        Some(SourceInterpretation::VibeDoc) => {
            Err(refused("Doc vocabulary requires an actual Doc package"))
        }
        _ => literal(bytes, path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn retained_reads_refuse_source_policy_lock_artifact_and_boot_membership_drift() {
        for change in ["source", "identity", "policy", "lock", "artifact", "boot"] {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            let boot = vibe_core::layout::current_boot_dir();
            fs::create_dir_all(root.join(&boot)).unwrap();
            let manifest = format!(
                "[project]\nname='host'\nversion='1.0.0'\n[[project.sources]]\nfile='raw.xml'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n[[project.sources]]\ndirectory='{}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n",
                vibe_core::machine_json_path(&boot)
            );
            fs::write(root.join("vibe.toml"), &manifest).unwrap();
            fs::write(root.join("raw.xml"), "<broken\n#use literal\n").unwrap();
            fs::write(root.join(boot.join("one.md")), "# One\n").unwrap();
            fs::write(root.join(boot.join("STATIC.md")), "generated").unwrap();
            let mut reads = Reads::new(root, None).unwrap();
            assert_eq!(reads.own_boot().unwrap().unwrap().len(), 1);
            assert!(matches!(
                reads.read(Path::new("raw.xml")).unwrap(),
                Body::Text(_)
            ));
            assert!(matches!(
                reads.read(&boot.join("STATIC.md")).unwrap(),
                Body::Text(_)
            ));
            reads.recheck().unwrap();
            match change {
                "source" => fs::write(root.join("raw.xml"), "changed").unwrap(),
                "identity" => {
                    fs::rename(root.join("raw.xml"), root.join("old.xml")).unwrap();
                    fs::copy(root.join("old.xml"), root.join("raw.xml")).unwrap();
                }
                "policy" => {
                    fs::write(root.join("vibe.toml"), format!("{manifest}\n# changed\n")).unwrap()
                }
                "lock" => Lockfile::empty("test", "0")
                    .write(root.join("vibe.lock"))
                    .unwrap(),
                "artifact" => fs::write(root.join(boot.join("STATIC.md")), "changed").unwrap(),
                _ => fs::write(root.join(boot.join("two.md")), "# Two\n").unwrap(),
            }
            assert!(reads.recheck().is_err(), "{change}");
        }
    }

    #[test]
    fn declared_installed_read_never_guesses_missing_format_even_for_empty_policy() {
        for sources in [
            "sources=[]\n",
            "[[package.sources]]\nfile='native.payload'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n",
        ] {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            fs::write(
                root.join("vibe.toml"),
                "[project]\nname='host'\nversion='1.0.0'\n",
            )
            .unwrap();
            let slot = vibe_workspace::vibedeps::slot_rel_path(
                &"org.source".parse().unwrap(),
                "plain",
                &"1.0.0".parse().unwrap(),
            );
            fs::create_dir_all(root.join(&slot)).unwrap();
            fs::write(root.join(&slot).join("vibe.toml"), format!("[package]\nkind='flow'\ngroup='org.source'\nname='plain'\nversion='1.0.0'\n{sources}")).unwrap();
            let lock = format!(
                "[meta]\nschema_version={}\ngenerated_by='test'\ngenerated_at='2026-01-01T00:00:00Z'\n[[package]]\ngroup='org.source'\nname='plain'\nkind='flow'\nversion='1.0.0'\ncontent_hash='sha256:{}'\nsource_url='https://example.invalid/plain.git'\n",
                vibe_core::manifest::CURRENT_SCHEMA_VERSION,
                "a".repeat(64)
            );
            fs::write(root.join("vibe.lock"), lock).unwrap();
            let mut reads = Reads::new(root, None).unwrap();
            let result = reads.read(&Path::new(&slot).join("native.payload"));
            assert!(
                result
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("independently selected materialized format")
            );
        }
    }
}
