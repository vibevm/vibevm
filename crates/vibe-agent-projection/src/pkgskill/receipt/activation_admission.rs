//! Shared physical-alias intent admission, independent from filesystem mutation.
use crate::{
    activation::{ActivationIntent, desired_enabled},
    agents::{Agent, Scope},
};
use anyhow::{Result, bail};
use std::path::Path;

pub(super) fn check_shared_intent(
    root: &Path,
    row: &vibe_wire::generated::package_skill::e3::receipt::PackageSkillBinding,
    selected: &[Agent],
    intent: &ActivationIntent,
) -> Result<()> {
    let mut aliases: std::collections::BTreeSet<String> = row.activation.keys().cloned().collect();
    aliases.extend(
        row.target
            .iter()
            .flat_map(|target| target.agents.iter().cloned()),
    );
    for selected_agent in selected {
        let Some(selected_path) = selected_agent.skills_root(Scope::Project, Some(root))? else {
            continue;
        };
        for alias in &aliases {
            let parsed = Agent::parse_filter(alias)?;
            if parsed.len() != 1 {
                bail!("shared skill alias is not canonical");
            }
            if selected.contains(&parsed[0]) {
                continue;
            }
            if parsed[0].skills_root(Scope::Project, Some(root))? == Some(selected_path.clone())
                && desired_enabled(row.activation.get(alias)) != intent.enabled
            {
                bail!(
                    "shared skill target also serves [{}]; select the whole shared group before changing activation",
                    aliases.iter().cloned().collect::<Vec<_>>().join(", ")
                );
            }
        }
    }
    Ok(())
}

pub(super) fn validate_intent_owner(
    receipt: &vibe_wire::generated::package_skill::e3::receipt::PackageSkillReceipt,
    key: &str,
    agent: Agent,
) -> Result<usize> {
    if receipt.applying.is_some() {
        bail!("recover active project skill transaction before activation");
    }
    let (index, row) = receipt
        .binding
        .iter()
        .enumerate()
        .find(|(_, r)| r.key == key)
        .ok_or_else(|| {
            anyhow::anyhow!("project skill owner is unknown; explicitly install it first")
        })?;
    if !row.activation.contains_key(agent.as_str())
        && !row
            .target
            .iter()
            .any(|t| t.agents.iter().any(|alias| alias == agent.as_str()))
    {
        bail!("selected project skill agent is not managed");
    }
    Ok(index)
}
