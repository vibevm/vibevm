//! One graph-preserving declared-set replacement; scalar v1 stays unchanged.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#DEFAULT-CONSENT");
use super::*;
use serde::Deserialize;
use vibe_core::manifest::{ConfigurationGroupDecl, Manifest, Materialization};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Proposal {
    pub schema: u32,
    pub protocol: String,
    pub review_id: String,
    pub acknowledgement: String,
    pub package: String,
    pub consumer: String,
    pub group: String,
    pub from_version: String,
    pub to_version: String,
    pub from_content_pin: String,
    pub to_content_pin: String,
    pub from_members: Vec<String>,
    pub to_members: Vec<String>,
}
#[derive(Debug, thiserror::Error)]
#[error(
    "package member-set configuration review requires the current exact explicit acknowledgement"
)]
pub(crate) struct ReviewError {
    pub proposal: Box<Proposal>,
    pub stale: bool,
}

pub(in crate::commands::update) fn applies(
    prior: &Lockfile,
    updated: &[super::super::scoped::Resolved],
) -> bool {
    updated.iter().any(|(cached, _, _, _, next)| {
        let key = format!("{}/{}", cached.resolved.group, cached.resolved.name);
        match (prior.configurations.get(&key), next.as_ref()) {
            (Some(old), Some(new)) => {
                old.automatic_basis != new.automatic_basis
                    && old
                        .automatic_basis
                        .as_ref()
                        .is_some_and(|b| !b.consumer_choice_sets.is_empty())
            }
            _ => false,
        }
    })
}
fn denied() -> anyhow::Error {
    anyhow::anyhow!(
        "multi-member automatic default renewal is not connected for this subject; v2 requires one exact local graph-preserving declared-set replacement"
    )
}

pub(super) fn gate(
    originals: &vibe_resolver::configuration::PackageConfigurationInputs,
    subject: Subject<'_>,
    prior_bytes: &[u8],
    prior: &Lockfile,
    updated: &[super::super::scoped::Resolved],
    response: Option<&str>,
) -> Result<()> {
    let Subject {
        workspace,
        selected,
        manifest,
    } = subject;
    if workspace != selected
        || manifest.workspace.is_some()
        || !manifest.requires.remaps.is_empty()
        || manifest.requires.packages.len() != 1
        || prior.meta.root_dependencies.len() != 1
        || prior.configurations.len() != 1
        || originals.len() != 1
        || updated.len() != prior.packages.len()
    {
        return Err(denied());
    }
    let (key, old) = prior.configurations.iter().next().ok_or_else(denied)?;
    let (cached, _, _, _, next) = updated
        .iter()
        .find(|(c, ..)| format!("{}/{}", c.resolved.group, c.resolved.name) == *key)
        .ok_or_else(denied)?;
    let new = next.as_ref().ok_or_else(denied)?;
    let before = prior
        .find(&cached.resolved.group, &cached.resolved.name)
        .ok_or_else(denied)?;
    let root = &manifest.requires.packages[0];
    if root.group.as_ref() != Some(&cached.resolved.group)
        || root.name.as_str() != cached.resolved.name
        || prior.meta.root_dependencies[0].group.as_ref() != Some(&cached.resolved.group)
        || prior.meta.root_dependencies[0].name.as_str() != cached.resolved.name
        || !prior.meta.root_dependencies[0].version.is_exact_pin()
        || !prior.meta.root_dependencies[0]
            .version
            .matches(&before.version)
        || root.version.to_string() != format!("={}", cached.resolved.version)
        || cached.package_meta().kind != vibe_core::PackageKind::Flow
        || cached.resolved.version == before.version
        || old.consumers != new.consumers
        || old.consumers.len() != 1
        || old.declarations != new.declarations
        || old.active_dependencies != new.active_dependencies
        || !old.submitted_inputs.is_empty()
        || !new.submitted_inputs.is_empty()
        || !old.forwarded_inputs.is_empty()
        || !new.forwarded_inputs.is_empty()
        || !old.selected_subskill_paths().is_empty()
        || !new.selected_subskill_paths().is_empty()
        || !old.definition.parameters.is_empty()
        || !new.definition.parameters.is_empty()
        || !old.definition.profiles.is_empty()
        || !new.definition.profiles.is_empty()
        || old.definition.groups.len() != 1
        || new.definition.groups.len() != 1
        || old
            .definition
            .features
            .iter()
            .chain(&new.definition.features)
            .any(|f| {
                !f.guards.is_empty()
                    || f.effects.iter().any(|effect| {
                        matches!(
                            effect,
                            vibe_core::manifest::ConfigurationEffect::Forward { .. }
                                | vibe_core::manifest::ConfigurationEffect::Subskill { .. }
                        )
                    })
            })
    {
        return Err(denied());
    }
    let (consumer, input) = old.consumers.iter().next().ok_or_else(denied)?;
    if consumer != "./vibe.toml"
        || input.policy != vibe_core::configuration::ChoicePolicy::UseDeclaredDefaults
        || !input.selections.is_empty()
        || input.profile.is_some()
        || input.default_features
        || !input.features.is_empty()
        || !input.parameters.is_empty()
    {
        return Err(denied());
    }
    let mut normalized_old = old.definition.clone();
    let mut normalized_new = new.definition.clone();
    let ConfigurationGroupDecl::Choice {
        id: group,
        cardinality,
        defaults: Some(from),
        guards,
        ..
    } = &mut normalized_old.groups[0]
    else {
        return Err(denied());
    };
    if *cardinality != vibe_core::configuration::Cardinality::ZeroOrMore || !guards.is_empty() {
        return Err(denied());
    }
    let group = group.clone();
    let mut from = from.clone();
    from.sort();
    from.dedup();
    let ConfigurationGroupDecl::Choice {
        id,
        cardinality,
        defaults: Some(to),
        guards,
        ..
    } = &mut normalized_new.groups[0]
    else {
        return Err(denied());
    };
    if *id != group
        || *cardinality != vibe_core::configuration::Cardinality::ZeroOrMore
        || !guards.is_empty()
    {
        return Err(denied());
    }
    let mut to_members = to.clone();
    to_members.sort();
    to_members.dedup();
    if from == to_members {
        return Err(denied());
    }
    *to = from.clone();
    if normalized_old.semantic_fingerprint()? != normalized_new.semantic_fingerprint()? {
        return Err(denied());
    }
    for (record, members) in [(old, &from), (new, &to_members)] {
        let basis = record.automatic_basis.as_ref().ok_or_else(denied)?;
        if !basis.consumer_choices.is_empty()
            || !basis.consumer_dormant_choices.is_empty()
            || !basis.consumer_feature_defaults.is_empty()
            || basis.consumer_choice_sets.len() != 1
            || basis
                .consumer_choice_sets
                .get(consumer)
                .is_none_or(|g| g.len() != 1 || g.get(&group) != Some(members))
        {
            return Err(denied());
        }
    }
    // Exact package membership and edges; only this root's version/payload may move.
    for row in &prior.packages {
        let (current, deps, _, embedded, _) = updated
            .iter()
            .find(|(c, ..)| c.resolved.group == row.group && c.resolved.name == row.name.as_str())
            .ok_or_else(denied)?;
        if current.package_meta().materialization != Materialization::Copy
            || row.materialization != Materialization::Copy
            || current.overridden
            || row.overridden
            || row.via_override.is_some()
            || !current.source_uri.starts_with("file://")
            || !row.source_url.as_str().starts_with("file://")
            || current.via_redirect.is_some()
            || row.via_redirect.is_some()
            || current.template.is_some()
            || row.template.is_some()
            || current.is_git_source
            || current.is_path_source
            || current.registry_name != row.registry
            || current.source_kind() != row.source_kind
            || current.source_ref != row.source_ref
            || current.resolved_commit != row.resolved_commit
            || !embedded.is_empty()
            || !row.embedded_sources.is_empty()
        {
            return Err(denied());
        }
        let is_root = current.resolved.group == cached.resolved.group
            && current.resolved.name == cached.resolved.name;
        if is_root {
            if !vibe_registry::local_preview::same_local_version_owner(
                row.source_url.as_str(),
                &current.source_uri,
            )? {
                return Err(denied());
            }
        } else if current.resolved.version != row.version
            || current.content_hash != row.content_hash.as_str()
            || current.source_uri != row.source_url.as_str()
        {
            return Err(denied());
        }
        let mut old_edges = row
            .dependencies
            .iter()
            .map(|p| (p.group.clone(), p.name.clone()))
            .collect::<Vec<_>>();
        let mut new_edges = deps
            .iter()
            .map(|p| (p.group.clone(), p.name.clone()))
            .collect::<Vec<_>>();
        old_edges.sort();
        new_edges.sort();
        if old_edges != new_edges {
            return Err(denied());
        }
    }
    let mut hash = Sha256::new();
    hash.update(b"vibevm.package-member-set-review/v2\0");
    let mut binding = Binding(hash);
    binding.path(workspace)?;
    binding.path(selected)?;
    binding.bytes(prior_bytes);
    binding.text(&root.to_string());
    binding.bytes(&serde_json::to_vec(originals)?);
    binding.bytes(&serde_json::to_vec(old)?);
    binding.bytes(&serde_json::to_vec(new)?);
    let mut candidates = updated.iter().collect::<Vec<_>>();
    candidates.sort_by_key(|(c, ..)| (c.resolved.group.clone(), c.resolved.name.clone()));
    binding.count(candidates.len());
    for (c, deps, _, _, _) in &candidates {
        binding.text(&c.resolved.group.to_string());
        binding.text(&c.resolved.name);
        binding.text(&c.resolved.version.to_string());
        binding.text(&c.source_uri);
        binding.text(&c.content_hash);
        let mut edges = deps.iter().map(ToString::to_string).collect::<Vec<_>>();
        edges.sort();
        binding.strings(&edges);
    }
    let token = format!(
        "package-member-set-review/2:sha256:{:x}",
        binding.0.finalize()
    );
    // Verify live endpoints as well as selected cached bytes; never trust sidecars.
    let mut proofs = Vec::new();
    proofs.push(vibe_registry::local_preview::verify_local_payload_url(
        before.source_url.as_str(),
        before.content_hash.as_str(),
    )?);
    for (c, ..) in &candidates {
        let live =
            vibe_registry::local_preview::verify_local_payload_url(&c.source_uri, &c.content_hash)?;
        let held =
            vibe_registry::local_preview::verify_pinned_payload(&c.cache_dir, &c.content_hash)?;
        let parsed = Manifest::parse_str(std::str::from_utf8(held.manifest_bytes()?)?)?;
        if parsed != c.manifest {
            return Err(denied());
        }
        proofs.push(live);
        proofs.push(held);
    }
    if response == Some(token.as_str()) {
        for proof in proofs {
            proof.recheck()?;
        }
        return Ok(());
    }
    Err(ReviewError {
        stale: response.is_some(),
        proposal: Box::new(Proposal {
            schema: 2,
            protocol: "vibe-package-member-set-review/2".into(),
            review_id: token.clone(),
            acknowledgement: token,
            package: key.clone(),
            consumer: consumer.clone(),
            group,
            from_version: before.version.to_string(),
            to_version: cached.resolved.version.to_string(),
            from_content_pin: before.content_hash.to_string(),
            to_content_pin: cached.content_hash.clone(),
            from_members: from,
            to_members,
        }),
    }
    .into())
}
