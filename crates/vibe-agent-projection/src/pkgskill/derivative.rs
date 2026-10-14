//! Passive admission of a derivative into an existing skill binding. Receipt
//! reconciliation remains the sole projection writer; this is not an effect grant.
use super::{DeclaredSkillProvider, ProjectSkillBinding};
use std::collections::BTreeMap;
use vibe_core::manifest::SourceSelector;
use vibe_workspace::derivatives::BoundDerivative;
use vibe_workspace::derivatives::compatibility::Compatibility;

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
