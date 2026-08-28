//! Adapter from authenticated lifecycle-world inputs to package-skill preset data.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_agent_projection::pkgskill::{
    DeclaredSkillProvider, PROJECT_SKILL_RECONCILE_KEY, PROJECT_SKILL_RECOVER_KEY,
    ProjectSkillBinding, ProjectSkillProviderInput, lower_project_skill_bindings,
    project_skill_receipt_exists,
};
use vibe_core::lifecycle::{ExtensionPoint, Phase, PhasePoint};
use vibe_core::manifest::{
    ExtensionConfig, ExtensionDecl, ExtensionHandler, ExtensionKey, SkillDecl,
};
use vibe_lifecycle::{
    DependencyProvider, DependencyProviderId, ExtensionProvider, HostIdentity, HostProvider,
    SyntheticPresetSource,
};

use super::LoadedDependency;

pub(crate) const RECONCILE_KEY: &str = PROJECT_SKILL_RECONCILE_KEY;
pub(crate) const RECOVER_KEY: &str = PROJECT_SKILL_RECOVER_KEY;
const INTERNAL_BUILTIN: &str = "package-skill-project";
type PresetPlan = (
    Vec<SyntheticPresetSource>,
    BTreeMap<String, ProjectSkillBinding>,
    BTreeSet<String>,
);

pub(super) fn presets(
    selected: &Path,
    host: &HostProvider,
    host_skills: &[SkillDecl],
    installed: &[LoadedDependency],
) -> Result<PresetPlan> {
    let mut inputs = Vec::new();
    if !host_skills.is_empty() {
        inputs.push(ProjectSkillProviderInput {
            provider: authored_provider(host)?,
            declarations: host_skills.to_vec(),
        });
    }
    inputs.extend(
        installed
            .iter()
            .filter(|dependency| !dependency.skills.is_empty())
            .map(|dependency| ProjectSkillProviderInput {
                provider: installed_provider(&dependency.source.provider),
                declarations: dependency.skills.clone(),
            }),
    );

    let bindings = lower_project_skill_bindings(selected, inputs)
        .context("lowering authenticated project-scope package skill bindings")?;
    let desired = bindings
        .iter()
        .map(ProjectSkillBinding::identity)
        .collect::<BTreeSet<_>>();
    let mut presets = Vec::with_capacity(bindings.len() + 2);
    if project_skill_receipt_exists(selected)? {
        // Engine rows: recovery of a durable applying transaction first, then
        // the vanished-binding sweep — both ahead of every ordinary binding
        // and both beyond host disable control.
        presets.push(SyntheticPresetSource {
            key: ExtensionKey::authored(RECOVER_KEY),
            provider: ExtensionProvider::Host(host.clone()),
            declaration: recovery_declaration(),
        });
        presets.push(SyntheticPresetSource {
            key: ExtensionKey::authored(RECONCILE_KEY),
            provider: ExtensionProvider::Host(host.clone()),
            declaration: reconcile_declaration(&desired),
        });
    }
    let mut by_key = BTreeMap::new();
    for binding in bindings {
        let key = binding.identity();
        presets.push(SyntheticPresetSource {
            key: ExtensionKey::authored(key.clone()),
            provider: provider(&binding),
            declaration: declaration(&binding),
        });
        if by_key.insert(key.clone(), binding).is_some() {
            bail!("duplicate package skill binding identity `{key}`");
        }
    }
    Ok((presets, by_key, desired))
}

fn authored_provider(host: &HostProvider) -> Result<DeclaredSkillProvider> {
    let HostIdentity::Coordinate(id) = &host.identity else {
        bail!("a selected host with [[skill]] must have package coordinates");
    };
    let kind = host
        .kind
        .context("a selected host with [[skill]] must have package kind")?;
    Ok(DeclaredSkillProvider::Authored {
        group: id.group().clone(),
        name: id.name().clone(),
        version: host.version.clone(),
        kind,
        root: host.root.clone(),
    })
}

fn installed_provider(provider: &DependencyProvider) -> DeclaredSkillProvider {
    DeclaredSkillProvider::Installed {
        group: provider.id.group().clone(),
        name: provider.id.name().clone(),
        version: provider.version.clone(),
        kind: provider.kind,
        root: provider.root.clone(),
        content_hash: provider.content_hash.clone(),
    }
}

fn provider(binding: &ProjectSkillBinding) -> ExtensionProvider {
    match &binding.skill.provider {
        DeclaredSkillProvider::Authored {
            group,
            name,
            version,
            kind,
            root,
        } => ExtensionProvider::Host(HostProvider {
            identity: HostIdentity::coordinate(DependencyProviderId::new(
                group.clone(),
                name.clone(),
            )),
            root: root.clone(),
            version: version.clone(),
            kind: Some(*kind),
            content_hash: None,
        }),
        DeclaredSkillProvider::Installed {
            group,
            name,
            version,
            kind,
            root,
            content_hash,
        } => ExtensionProvider::Dependency(DependencyProvider {
            id: DependencyProviderId::new(group.clone(), name.clone()),
            root: root.clone(),
            version: version.clone(),
            kind: *kind,
            content_hash: content_hash.clone(),
        }),
    }
}

fn declaration(binding: &ProjectSkillBinding) -> ExtensionDecl {
    let mut config = toml::Table::new();
    config.insert(
        "provider".into(),
        toml::Value::String(binding.skill.provider.identity()),
    );
    config.insert(
        "skill".into(),
        toml::Value::String(binding.skill.decl.name.clone()),
    );
    config.insert(
        "source_snapshot".into(),
        toml::Value::String(binding.source_snapshot.clone()),
    );
    config.insert(
        "include".into(),
        strings(binding.skill.decl.include.iter().cloned()),
    );
    config.insert(
        "target_agents".into(),
        strings(
            binding
                .targets
                .iter()
                .map(|target| target.agent.as_str().to_string()),
        ),
    );
    config.insert(
        "target_paths".into(),
        strings(
            binding
                .targets
                .iter()
                .map(|target| vibe_core::machine_json_path(&target.path)),
        ),
    );
    internal_declaration(format!("package-skill-{}", binding.skill.decl.name), config)
}

fn recovery_declaration() -> ExtensionDecl {
    internal_declaration("package-skill-recover".into(), toml::Table::new())
}

fn reconcile_declaration(desired: &BTreeSet<String>) -> ExtensionDecl {
    let mut config = toml::Table::new();
    config.insert("desired".into(), strings(desired.iter().cloned()));
    internal_declaration("package-skill-reconcile".into(), config)
}

fn internal_declaration(id: String, config: toml::Table) -> ExtensionDecl {
    ExtensionDecl {
        id,
        point: ExtensionPoint::Phase(PhasePoint::Default(Phase::Package)),
        handler: ExtensionHandler::Builtin {
            name: INTERNAL_BUILTIN.into(),
        },
        config: Some(ExtensionConfig::from_table(config)),
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: None,
        pass: None,
        when: None,
    }
}

fn strings(values: impl IntoIterator<Item = String>) -> toml::Value {
    toml::Value::Array(values.into_iter().map(toml::Value::String).collect())
}
