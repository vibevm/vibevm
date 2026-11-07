//! Finite progress-owned observations; scope remains owned by progress-core.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-PROGRESS-READ",
    r = 1
);

use anyhow::Result;
use progress_core::scope::{self, ScopeConfig};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use vibe_core::manifest::{
    Manifest, SelectedSource, SourceInterpretation, SourceSelector, select_source_rows,
};
use vibe_safefs::{Project, SourceObservation};
use vibe_spec::source_policy::{
    PackagePolicyObservation, PackageSourceMode, observe_package_policy,
};

mod installed;
use installed::Installed;

pub(super) fn refused(reason: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-PROGRESS-READ: {reason}; fix surface: selected progress scope, source policy, bytes and independent installed binding"
    )
}

pub(super) struct Text {
    pub path: String,
    pub physical: String,
    pub text: String,
    pub xml: bool,
}

struct Host {
    policy: PackagePolicyObservation,
    sources: Vec<(String, SourceObservation)>,
}

pub(in crate::commands::progress) struct Sources {
    root: PathBuf,
    project: Project,
    configs: Vec<(&'static str, Option<SourceObservation>)>,
    cfg: ScopeConfig,
    files: Vec<PathBuf>,
    hosts: BTreeMap<PathBuf, Host>,
    members: BTreeMap<PathBuf, Manifest>,
    workspace: Option<Manifest>,
    legacy: super::VocabularyByPackage,
    legacy_sources: Vec<(String, SourceObservation)>,
    installed: Option<Installed>,
}

impl Sources {
    pub(super) fn new(root: &Path, cfg: ScopeConfig, files: Vec<PathBuf>) -> Result<Self> {
        let project = Project::open(root)?;
        let facts = project.read_source_observation("facts.toml")?;
        let mut configs = Vec::new();
        if facts.is_none() {
            configs.push((
                "progress.toml",
                project.read_source_observation("progress.toml")?,
            ));
        }
        configs.push(("facts.toml", facts));
        let current = scope::load_config(root)?;
        if current.include != cfg.include
            || current.exclude != cfg.exclude
            || current.schema != cfg.schema
            || current.judging.exempt != cfg.judging.exempt
            || current.progress.cache_dir != cfg.progress.cache_dir
        {
            return Err(refused("scope configuration changed during capture"));
        }
        let policy = observe_package_policy(root)?;
        let installed = Installed::select(root, &policy)?;
        if installed.is_none()
            && matches!(
                policy.mode(),
                PackageSourceMode::Explicit | PackageSourceMode::ExplicitEmpty
            )
            && policy.materialized_record_present()?
        {
            return Err(refused(
                "materialized input has no independently selected installed binding",
            ));
        }
        let mut members = BTreeMap::new();
        let mut workspace = None;
        if policy.manifest().is_some_and(Manifest::is_workspace_root) {
            let ws = vibe_workspace::Workspace::load(root)?;
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
        let mut reads = Self {
            root: root.to_owned(),
            project,
            configs,
            cfg,
            files,
            hosts,
            members,
            workspace,
            legacy: Default::default(),
            legacy_sources: Vec::new(),
            installed,
        };
        // Exact declarations selected by the scope are requests even if glob
        // expansion could not find their files. Unselected rows remain inert.
        if let Some(installed) = &reads.installed {
            installed.check_selected(&reads.cfg)?;
        } else {
            reads.check_declared_paths(root)?;
            // Registered owners can contain an exact selected declaration whose
            // file is absent, so no enumerated file will call read for it.
            let members = reads.members.clone();
            for (owner, manifest) in members {
                let selected = manifest.package.as_ref().and_then(|p| p.sources.as_deref()).or_else(|| manifest.project.as_ref().and_then(|p| p.sources.as_deref())).unwrap_or_default().iter().any(|row| {
                    matches!(row.selector(), SourceSelector::File(path) if scope::includes_path(root, owner.join(path).strip_prefix(root).unwrap_or(Path::new(path)), &reads.cfg).unwrap_or(true))
                });
                if selected {
                    let policy = observe_package_policy(&owner)?;
                    if policy.manifest() != Some(&manifest) {
                        return Err(refused("registered member policy changed"));
                    }
                    reads.hosts.insert(
                        owner.clone(),
                        Host {
                            policy,
                            sources: Vec::new(),
                        },
                    );
                    reads.check_declared_paths(&owner)?;
                }
            }
        }
        Ok(reads)
    }

    fn check_declared_paths(&mut self, owner: &Path) -> Result<()> {
        let policy = &self.hosts[owner].policy;
        for row in policy.source_rows().unwrap_or_default() {
            if let SourceSelector::File(path) = row.selector() {
                let full = owner.join(path);
                let rel = full.strip_prefix(&self.root)?;
                if scope::includes_path(&self.root, rel, &self.cfg)? {
                    // Select again: a shadowed row never decides interpretation.
                    let selected = classify(policy, path)?;
                    if selected.is_some() && policy.read_source_observation(path)?.is_none() {
                        return Err(refused(format!(
                            "selected source `{}` is missing",
                            full.display()
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn read(&mut self, relative: &Path) -> Result<Option<Text>> {
        if let Some(installed) = &mut self.installed
            && installed.is_declared()
        {
            return installed.read(relative);
        }
        let full = self.root.join(relative);
        let mut owner = self
            .members
            .keys()
            .filter(|p| full.starts_with(p))
            .max_by_key(|p| p.components().count())
            .cloned()
            .unwrap_or_else(|| self.root.clone());
        if owner == self.root
            && matches!(
                self.hosts[&owner].policy.mode(),
                PackageSourceMode::Legacy | PackageSourceMode::MissingManifest
            )
        {
            for ancestor in full
                .parent()
                .into_iter()
                .flat_map(Path::ancestors)
                .take_while(|p| *p != self.root)
            {
                let policy = observe_package_policy(ancestor)?;
                match policy.mode() {
                    PackageSourceMode::Explicit | PackageSourceMode::ExplicitEmpty => {
                        owner = ancestor.to_owned();
                        self.hosts.entry(owner.clone()).or_insert(Host {
                            policy,
                            sources: Vec::new(),
                        });
                        self.check_declared_paths(&owner)?;
                        break;
                    }
                    PackageSourceMode::Legacy => break,
                    PackageSourceMode::MissingManifest => {}
                }
            }
        }
        if !self.hosts.contains_key(&owner) {
            vibe_safefs::ensure_no_follow_walk(self.project.root_path(), &full, true)?;
            let policy = observe_package_policy(&owner)?;
            if policy.manifest() != self.members.get(&owner) {
                return Err(refused("registered member policy changed during selection"));
            }
            self.hosts.insert(
                owner.clone(),
                Host {
                    policy,
                    sources: Vec::new(),
                },
            );
            self.check_declared_paths(&owner)?;
        }
        let host = self
            .hosts
            .get_mut(&owner)
            .ok_or_else(|| refused("selected owner absent"))?;
        match host.policy.mode() {
            PackageSourceMode::ExplicitEmpty => Ok(None),
            PackageSourceMode::Explicit => {
                if host.policy.materialized_record_present()? {
                    return Err(refused(
                        "materialized input has no independently selected installed binding",
                    ));
                }
                let path = vibe_core::machine_json_path(full.strip_prefix(&owner)?);
                let Some((media, interpretation)) = classify(&host.policy, &path)? else {
                    return Ok(None);
                };
                let observation = host
                    .policy
                    .read_source_observation(&path)?
                    .ok_or_else(|| refused(format!("selected source `{path}` is missing")))?;
                let text = project(observation.bytes(), &media, interpretation)?;
                host.sources.push((path, observation));
                Ok(Some(Text {
                    path: scope::rel_str(relative),
                    physical: scope::rel_str(relative),
                    text,
                    xml: media == "application/xml",
                }))
            }
            PackageSourceMode::Legacy | PackageSourceMode::MissingManifest => {
                let Some(vocab) = self.legacy.of(&self.root, &full)? else {
                    return Ok(None);
                };
                let path = scope::rel_str(relative);
                let observation =
                    self.project
                        .read_source_observation(&path)?
                        .ok_or_else(|| {
                            refused(format!("selected legacy source `{path}` disappeared"))
                        })?;
                let raw = std::str::from_utf8(observation.bytes())?;
                let (text, kind) = vibe_specdoc::project_spec_text_with(&full, raw, vocab)?;
                self.legacy_sources.push((path.clone(), observation));
                Ok(Some(Text {
                    physical: path.clone(),
                    path,
                    text,
                    xml: kind == vibe_specdoc::SourceKind::XmlProjected,
                }))
            }
        }
    }

    pub(super) fn recheck(&self) -> Result<()> {
        for (name, before) in &self.configs {
            let after = self.project.read_source_observation(name)?;
            if after.as_ref().map(|s| (s.identity(), s.bytes()))
                != before.as_ref().map(|s| (s.identity(), s.bytes()))
            {
                return Err(refused(format!("scope configuration `{name}` changed")));
            }
        }
        if scope::observed_files(&self.root, &self.cfg)? != self.files {
            return Err(refused("selected corpus membership changed"));
        }
        if let Some(expected) = &self.workspace {
            let ws = vibe_workspace::Workspace::load(&self.root)?;
            let members = ws
                .members
                .into_iter()
                .map(|m| (self.root.join(m.rel_path.as_str()), m.manifest))
                .collect::<BTreeMap<_, _>>();
            if ws.root_manifest != *expected || members != self.members {
                return Err(refused("registered source ownership changed"));
            }
        }
        self.legacy.recheck()?;
        for host in self.hosts.values() {
            host.policy.recheck_unchanged()?;
            for (path, before) in &host.sources {
                compare(before, host.policy.read_source_observation(path)?, path)?;
            }
        }
        for (path, before) in &self.legacy_sources {
            compare(before, self.project.read_source_observation(path)?, path)?;
        }
        if let Some(installed) = &self.installed {
            installed.recheck()?;
        }
        Ok(())
    }
}

fn compare(before: &SourceObservation, after: Option<SourceObservation>, path: &str) -> Result<()> {
    if after.as_ref().map(|s| (s.identity(), s.bytes()))
        != Some((before.identity(), before.bytes()))
    {
        return Err(refused(format!(
            "selected source `{path}` changed or disappeared"
        )));
    }
    Ok(())
}

fn classify(
    policy: &PackagePolicyObservation,
    path: &str,
) -> Result<Option<(String, SourceInterpretation)>> {
    let Some(rows) = policy.source_rows() else {
        return Ok(None);
    };
    match select_source_rows("progress selected owner", rows, path)
        .map_err(|e| refused(format!("{path}: {e:?}")))?
    {
        SelectedSource::Descriptor { descriptor, .. } => {
            let interpretation = descriptor.interpretation();
            if interpretation == SourceInterpretation::VibeDoc
                && !policy
                    .package()
                    .is_some_and(|p| p.kind == vibe_core::PackageKind::Doc)
            {
                return Err(refused(format!(
                    "{path}: Doc vocabulary requires an actual Doc package"
                )));
            }
            Ok(matches!(
                interpretation,
                SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc
            )
            .then(|| (descriptor.media().into(), interpretation)))
        }
        SelectedSource::Unclassified { .. } => Ok(None),
        SelectedSource::NeedsInput { .. } => {
            Err(refused(format!("ambiguous selected source `{path}`")))
        }
    }
}

pub(super) fn project(
    bytes: &[u8],
    media: &str,
    interpretation: SourceInterpretation,
) -> Result<String> {
    vibe_spec::project_declared_native(bytes, media, interpretation).map_err(refused)
}
