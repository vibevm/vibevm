//! Passive admission of a derivative into an existing skill binding. Receipt
//! reconciliation remains the sole projection writer; this is not an effect grant.
use super::{DeclaredSkillProvider, ProjectSkillBinding};
use std::collections::BTreeMap;
use vibe_core::manifest::SourceSelector;
use vibe_workspace::derivatives::BoundDerivative;
use vibe_workspace::derivatives::compatibility::Compatibility;

/// Prepare an inert handoff from the complete checked local result. This calls
/// the existing receipt planner directly, without hydration. Reconciliation is
/// still a separate, explicitly invoked consumer operation.
pub fn prepare_materialized_binding(
    result: &vibe_workspace::derivatives::MaterializedDerivative,
    project_root: &std::path::Path,
    declaration: vibe_core::manifest::SkillDecl,
) -> anyhow::Result<ProjectSkillBinding> {
    result.verify_tree()?;
    let local = result.result().binding().local();
    if declaration.source.is_some()
        || !declaration.resources.is_empty()
        || !declaration.include.is_empty()
        || declaration.path != std::path::Path::new(local.descriptor.selector().path())
    {
        anyhow::bail!("derivative skill must select the complete local result without hydration");
    }
    let root = result.tree().root_path().to_path_buf();
    let provider = DeclaredSkillProvider::Authored {
        group: local
            .owner
            .group
            .clone()
            .ok_or_else(|| anyhow::anyhow!("missing group"))?,
        name: local.owner.name.clone(),
        version: local
            .owner
            .version
            .to_string()
            .trim_start_matches('=')
            .to_owned(),
        kind: local
            .owner
            .kind
            .ok_or_else(|| anyhow::anyhow!("missing kind"))?,
        root: root.clone(),
        embedded_sources: vec![],
    };
    let skill = super::DeclaredSkill {
        source: root.join(&declaration.path),
        source_root: root,
        origin: provider.identity(),
        provider,
        decl: declaration,
    };
    let binding = super::projection::lower_one_binding(project_root, skill)?;
    match inspect_binding(result.result(), &binding) {
        Compatibility::Compatible => Ok(binding),
        other => anyhow::bail!("derivative projection interface: {other:?}"),
    }
}

pub fn inspect_binding(result: &BoundDerivative, binding: &ProjectSkillBinding) -> Compatibility {
    let local = result.binding().local();
    let (group, name, version, kind) = match &binding.skill.provider {
        DeclaredSkillProvider::Authored {
            group,
            name,
            version,
            kind,
            ..
        }
        | DeclaredSkillProvider::Installed {
            group,
            name,
            version,
            kind,
            ..
        } => (group, name, version, kind),
    };
    let owner = format!("{}:{}/{}@={}", kind.as_str(), group, name, version);
    if owner != local.owner.to_string() {
        return Compatibility::Incompatible("projection provider is not the result owner".into());
    }
    let Some(selected) = &binding.selected_files else {
        return Compatibility::Unknown("projection source snapshot is absent".into());
    };
    let expected: BTreeMap<String, Vec<u8>> = local
        .files
        .iter()
        .map(|file| {
            let relative = match local.descriptor.selector() {
                SourceSelector::Directory(root) => {
                    file.path.strip_prefix(&format!("{root}/")).unwrap()
                }
                SourceSelector::File(_) => file.path.rsplit('/').next().unwrap(),
            };
            (relative.to_owned(), file.bytes.clone())
        })
        .collect();
    if selected != &expected {
        return Compatibility::Incompatible(
            "projection snapshot differs from actual result inventory".into(),
        );
    }
    if let Err(error) = super::receipt::judge_selection(selected.keys().map(String::as_str)) {
        return Compatibility::Incompatible(error.to_string());
    }
    if !selected.contains_key("SKILL.md") {
        return Compatibility::Incompatible("result has no SKILL.md interface".into());
    }
    if let Err(error) =
        super::validate_skill_frontmatter(&binding.skill.decl.name, &binding.skill.source, selected)
    {
        return Compatibility::Incompatible(error.to_string());
    }
    if binding.targets.is_empty() {
        return Compatibility::Unknown("no projection target selected".into());
    }
    Compatibility::Compatible
}

#[cfg(test)]
mod m_10_b_2_cases;

#[cfg(test)]
mod m_10_c_1_cases;

#[cfg(test)]
mod m_10_d_2_cases;
