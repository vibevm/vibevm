//! Exact selected inputs retained until the tree model is ready.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-TREE-READ",
    r = 1
);

use super::*;
use std::sync::Arc;
use vibe_core::manifest::{SelectedSource, SourceInterpretation, SpecFormat, select_source_rows};
use vibe_safefs::{Project as HeldProject, SourceObservation};
use vibe_spec::source_policy::{
    PackagePolicyObservation, PackageSourceMode, observe_package_policy,
};
use vibe_spec::{FileResolver, InstalledSourceExpectation, SelectedPackage, SelfCoordinate};
use vibe_workspace::{Workspace, vibedeps};

pub(super) fn refused(reason: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-TREE-READ: {reason}; fix surface: selected boot path, owner policy and independent installed binding"
    )
}

struct Host {
    policy: PackagePolicyObservation,
    sources: Vec<(String, SourceObservation)>,
}
struct Installed {
    resolver: FileResolver,
    sources: Vec<String>,
}
pub(super) struct SelectedReads<'a> {
    root: &'a Path,
    project: Arc<HeldProject>,
    lock: Option<SourceObservation>,
    format: Option<SpecFormat>,
    slots: BTreeMap<PathBuf, &'a LockedPackage>,
    hosts: BTreeMap<PathBuf, Host>,
    members: BTreeMap<PathBuf, Manifest>,
    workspace_manifest: Option<Manifest>,
    installed: BTreeMap<PathBuf, Installed>,
}
impl<'a> SelectedReads<'a> {
    pub(super) fn new(
        root: &'a Path,
        lock: &'a Lockfile,
        format: Option<SpecFormat>,
        manifest: Option<&Manifest>,
    ) -> Result<Self> {
        let project = Arc::new(HeldProject::open(root)?);
        let observed_lock = project.read_source_observation(Lockfile::FILENAME)?;
        if let Some(observed) = &observed_lock
            && Lockfile::parse_str(std::str::from_utf8(observed.bytes())?)? != *lock
        {
            return Err(refused("lock changed during tree selection"));
        }
        if manifest.is_some()
            && observed_lock.is_none()
            && *lock != Lockfile::empty("vibe (no-lockfile)", "0")
        {
            return Err(refused("selected lock disappeared before tree capture"));
        }
        let policy = observe_package_policy(root)?;
        if manifest.is_some() && manifest != policy.manifest() {
            return Err(refused("host manifest changed during tree selection"));
        }
        let mut members = BTreeMap::new();
        let mut workspace_manifest = None;
        if policy.manifest().is_some_and(Manifest::is_workspace_root) {
            let ws = Workspace::load(root)?;
            if !same_authored_manifest(policy.manifest(), &ws.root_manifest) {
                return Err(refused("workspace root changed during owner selection"));
            }
            workspace_manifest = Some(ws.root_manifest);
            for member in ws.members {
                members.insert(root.join(member.rel_path.as_str()), member.manifest);
            }
        }
        let slots = lock
            .packages
            .iter()
            .map(|p| {
                let relative = if p.materialization.is_in_place() {
                    vibedeps::in_place_slot_rel_path(&p.group, &p.name)
                } else {
                    vibedeps::slot_rel_path(&p.group, &p.name, &p.version)
                };
                (root.join(relative), p)
            })
            .collect();
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
            lock: observed_lock,
            format,
            slots,
            hosts,
            members,
            workspace_manifest,
            installed: BTreeMap::new(),
        })
    }

    pub(super) fn read(&mut self, relative: &Path) -> Result<Option<(String, bool)>> {
        let full = self.root.join(relative);
        if let Some((owner, package)) = self
            .slots
            .iter()
            .filter(|(slot, _)| full.starts_with(slot))
            .max_by_key(|(slot, _)| slot.components().count())
        {
            let owner = owner.clone();
            let package = *package;
            let path = full.strip_prefix(&owner)?;
            if vibe_core::layout::is_generated_boot_output(path) {
                return Ok(None);
            }
            return self.installed(&owner, package, path);
        }
        if relative.starts_with(vibe_core::layout::current_vibedeps_root())
            || relative.starts_with(vibe_core::layout::current_packages_root())
        {
            return Err(refused(format!(
                "tree boot source `{}` has no selected package owner",
                relative.display()
            )));
        }
        let owner = self
            .members
            .keys()
            .filter(|path| full.starts_with(path))
            .max_by_key(|path| path.components().count())
            .cloned()
            .unwrap_or_else(|| self.root.to_owned());
        let relative = full.strip_prefix(&owner)?;
        if vibe_core::layout::is_generated_boot_output(relative) {
            return Ok(None);
        }
        if !self.hosts.contains_key(&owner) {
            vibe_safefs::ensure_no_follow_walk(self.project.root_path(), &full, true)?;
            let policy = observe_package_policy(&owner)?;
            // Workspace resolves requires placeholders; authored fields stay
            // exact, and the complete resolved registration is rechecked below.
            let registered = &self.members[&owner];
            if !same_authored_manifest(policy.manifest(), registered) {
                return Err(refused(
                    "registered member manifest changed during selection",
                ));
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
            .ok_or_else(|| refused("selected host owner is unavailable"))?;
        match host.policy.mode() {
            PackageSourceMode::ExplicitEmpty => Ok(None),
            PackageSourceMode::Legacy | PackageSourceMode::MissingManifest => Ok(legacy(&full)),
            PackageSourceMode::Explicit => {
                let path = vibe_core::machine_json_path(relative);
                // Diagnostic label only: ungrouped nodes need no package coordinate.
                let descriptor = match select_source_rows(
                    &owner.display().to_string(),
                    host.policy
                        .source_rows()
                        .ok_or_else(|| refused("selected source declarations are unavailable"))?,
                    &path,
                )
                .map_err(|e| refused(format!("{path}: {e:?}")))?
                {
                    SelectedSource::Descriptor { descriptor, .. } => descriptor,
                    SelectedSource::Unclassified { .. } => return Ok(None),
                    SelectedSource::NeedsInput { .. } => {
                        return Err(refused(format!("ambiguous selected source `{path}`")));
                    }
                };
                if descriptor.interpretation() != SourceInterpretation::VibeSpec {
                    return Ok(None);
                }
                let media = descriptor.media().to_owned();
                let observation = host.policy.read_source_observation(&path)?.ok_or_else(|| {
                    refused(format!("selected native source `{path}` is missing"))
                })?;
                let text = vibe_spec::project_declared_native(
                    observation.bytes(),
                    &media,
                    SourceInterpretation::VibeSpec,
                )
                .map_err(|error| refused(format!("{path}: {error}")))?;
                host.sources.push((path, observation));
                Ok(Some((text, true)))
            }
        }
    }

    fn installed(
        &mut self,
        owner: &Path,
        package: &LockedPackage,
        relative: &Path,
    ) -> Result<Option<(String, bool)>> {
        if !self.hosts.contains_key(owner) && !self.installed.contains_key(owner) {
            let slot_relative = vibe_core::machine_json_path(owner.strip_prefix(self.root)?);
            let slot = self
                .project
                .dir(&slot_relative.split('/').collect::<Vec<_>>(), false)?;
            let policy = observe_package_policy(slot.path())?;
            match policy.mode() {
                PackageSourceMode::MissingManifest => {
                    return Err(refused("selected tree package has no manifest"));
                }
                PackageSourceMode::Explicit | PackageSourceMode::ExplicitEmpty => {
                    if package.materialization.is_in_place() {
                        return Err(refused(
                            "declared in-place tree source has no installed binding",
                        ));
                    }
                    let format = self.format.ok_or_else(|| refused("declared installed tree source requires independently selected materialized format"))?;
                    let expected = InstalledSourceExpectation {
                        kind: package.kind,
                        group: package.group.clone(),
                        name: package.name.to_string(),
                        version: package.version.to_string(),
                        source_hash: package.content_hash.clone(),
                        materialized_format: format,
                    };
                    let binding = vibedeps::prepare_declared_query_read(
                        &self.project,
                        &slot,
                        &slot_relative,
                        &expected,
                    )?
                    .ok_or_else(|| refused("selected declared tree source lost its binding"))?;
                    let world = self
                        .slots
                        .iter()
                        .map(|(path, p)| {
                            (
                                (p.group.to_string(), p.name.to_string()),
                                SelectedPackage::new(p.version.to_string(), path.clone()),
                            )
                        })
                        .collect();
                    let root_manifest = self.hosts[self.root].policy.manifest();
                    let node = root_manifest.and_then(Manifest::consumer_node);
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
                        },
                    );
                }
                PackageSourceMode::Legacy => {
                    if !package.materialization.is_in_place() {
                        let expected = InstalledSourceExpectation {
                            kind: package.kind,
                            group: package.group.clone(),
                            name: package.name.to_string(),
                            version: package.version.to_string(),
                            source_hash: package.content_hash.clone(),
                            materialized_format: self.format.unwrap_or(SpecFormat::Mixed),
                        };
                        // Even a legacy policy must not hide an incoherent
                        // record by deleting the declarations it witnessed.
                        if vibedeps::prepare_declared_query_read(
                            &self.project,
                            &slot,
                            &slot_relative,
                            &expected,
                        )?
                        .is_some()
                        {
                            return Err(refused("source policy changed during legacy admission"));
                        }
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
        }
        let Some(installed) = self.installed.get_mut(owner) else {
            return Ok(legacy(&owner.join(relative)));
        };
        let row = installed
            .resolver
            .installed_source_binding(package.group.as_str(), &package.name)?
            .outputs()
            .iter()
            .find(|row| Path::new(&row.output_path) == relative)
            .ok_or_else(|| {
                refused(format!(
                    "selected output `{}` is not recorded",
                    relative.display()
                ))
            })?
            .clone();
        let bytes = installed.resolver.read_installed_source(
            package.group.as_str(),
            &package.name,
            &row.source_path,
            None,
        )?;
        installed.sources.push(row.source_path);
        if row.interpretation != Some(SourceInterpretation::VibeSpec) {
            return Ok(None);
        }
        let text =
            vibe_spec::project_declared_native(&bytes, &row.media, SourceInterpretation::VibeSpec)
                .map_err(|error| refused(format!("{}: {error}", relative.display())))?;
        Ok(Some((text, true)))
    }

    pub(super) fn recheck(&self) -> Result<()> {
        if let Some(expected) = &self.workspace_manifest {
            let current = Workspace::load(self.root)?;
            let members: BTreeMap<_, _> = current
                .members
                .into_iter()
                .map(|m| (self.root.join(m.rel_path.as_str()), m.manifest))
                .collect();
            if current.root_manifest != *expected || members != self.members {
                return Err(refused(
                    "registered workspace ownership changed during tree read",
                ));
            }
        }
        let lock = self.project.read_source_observation(Lockfile::FILENAME)?;
        if lock.as_ref().map(|s| (s.identity(), s.bytes()))
            != self.lock.as_ref().map(|s| (s.identity(), s.bytes()))
        {
            return Err(refused("lock changed during tree read"));
        }
        for host in self.hosts.values() {
            host.policy.recheck_unchanged()?;
            for (path, before) in &host.sources {
                let after = host
                    .policy
                    .read_source_observation(path)?
                    .ok_or_else(|| refused(format!("selected source disappeared: {path}")))?;
                if before.identity() != after.identity() || before.bytes() != after.bytes() {
                    return Err(refused(format!("selected source changed: {path}")));
                }
            }
        }
        if let Some(format) = self.format
            && let Some(manifest) = self.hosts[self.root].policy.manifest()
            && crate::commands::reinstall::effective_rederive_format(manifest)? != format
        {
            return Err(refused(
                "selected materialized format changed during tree read",
            ));
        }
        for (owner, installed) in &self.installed {
            let p = self.slots[owner];
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
fn same_authored_manifest(observed: Option<&Manifest>, resolved: &Manifest) -> bool {
    let Some(observed) = observed else {
        return false;
    };
    if observed.requires.var_packages.is_empty() {
        return observed == resolved;
    }
    let mut authored = observed.clone();
    authored.requires = resolved.requires.clone();
    authored == *resolved
}
fn legacy(path: &Path) -> Option<(String, bool)> {
    vibe_specdoc::load_spec_text(path)
        .ok()
        .map(|(text, _)| (text, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(root: &Path) {
        fs::write(root.join("vibe.toml"), "[project]\nname='host'\nversion='1.0.0'\n[[project.sources]]\nfile='native.payload'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
        fs::write(
            root.join("native.payload"),
            "#use spec://org.example/lib/rule\n",
        )
        .unwrap();
    }

    #[test]
    fn retained_host_observation_refuses_source_manifest_and_lock_drift() {
        for change in ["source", "manifest", "lock", "identity"] {
            let root = tempfile::tempdir().unwrap();
            declared(root.path());
            let lock = Lockfile::empty("test", "0");
            lock.write(root.path().join(Lockfile::FILENAME)).unwrap();
            let mut read = SelectedReads::new(root.path(), &lock, None, None).unwrap();
            assert!(read.read(Path::new("native.payload")).unwrap().is_some());
            read.recheck().unwrap();
            match change {
                "source" => fs::write(root.path().join("native.payload"), "changed").unwrap(),
                "manifest" => fs::write(
                    root.path().join("vibe.toml"),
                    "[project]\nname='host'\nversion='1.0.0'\nsources=[]\n",
                )
                .unwrap(),
                "lock" => fs::remove_file(root.path().join(Lockfile::FILENAME)).unwrap(),
                _ => {
                    fs::rename(
                        root.path().join("native.payload"),
                        root.path().join("old.payload"),
                    )
                    .unwrap();
                    fs::copy(
                        root.path().join("old.payload"),
                        root.path().join("native.payload"),
                    )
                    .unwrap();
                }
            }
            assert!(read.recheck().is_err(), "{change}");
        }
    }

    #[test]
    fn selected_ambiguous_source_refuses_before_native_parsing() {
        let root = tempfile::tempdir().unwrap();
        declared(root.path());
        let manifest = fs::read_to_string(root.path().join("vibe.toml")).unwrap();
        let rows = manifest
            .split("[[project.sources]]")
            .nth(1)
            .unwrap()
            .replace("vibe-spec", "commonmark")
            .replace("role='spec'", "role='resource'");
        fs::write(
            root.path().join("vibe.toml"),
            format!("{manifest}\n[[project.sources]]{rows}"),
        )
        .unwrap();
        let lock = Lockfile::empty("test", "0");
        let mut read = SelectedReads::new(root.path(), &lock, None, None).unwrap();
        assert!(
            read.read(Path::new("native.payload"))
                .unwrap_err()
                .to_string()
                .contains("ambiguous")
        );
    }

    #[test]
    fn selected_doc_is_not_a_managed_directive_source() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("vibe.toml"), "[project]\ngroup='org.example'\nname='guide'\nversion='1.0.0'\n[[project.sources]]\nfile='doc.payload'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n").unwrap();
        fs::write(
            root.path().join("doc.payload"),
            "<invalid\n#use spec://forged/package/rule",
        )
        .unwrap();
        let lock = Lockfile::empty("test", "0");
        let mut read = SelectedReads::new(root.path(), &lock, None, None).unwrap();
        assert!(read.read(Path::new("doc.payload")).unwrap().is_none());
        read.recheck().unwrap();
    }
}
