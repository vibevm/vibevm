//! Observe required remap offers without adopting them or changing this run's Locator.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#layers-packages");
use super::*;
use std::collections::BTreeSet;
use vibe_resolver::{
    RootRequirement, SourceSolveRequest,
    source_candidates::{EdgeId, EdgeOwner},
    source_edges::{
        OriginalRequirement, RequirementActivation, RequirementAlternative, RequirementSite,
    },
};
use vibe_wire::generated::install::e2::plan::{
    RemapInputRequired, RemapPlanEntry, RemapPlanOutcome,
};

/// This uses the same remap source catalog as adoption (no embedded family),
/// but explicit storage grants writes only below this operation's scratch.
pub(crate) fn preview_project_requirements(
    manifest: &Manifest,
    locked: &Lockfile,
    operation: &vibe_package_source::SourceOperation,
    root: &Path,
    options: &vibe_package_source::PackageSourceOptions,
    offline: bool,
    progress: &vibe_core::progress::Progress,
) -> Result<Vec<RemapPlanEntry>> {
    let mut output = Vec::new();
    let mut new = Vec::new();
    let mut locked_sources = Vec::new();
    for requirement in &manifest.requires.remaps {
        if requirement
            .kind
            .is_some_and(|kind| kind != PackageKind::Remap)
        {
            bail!(
                "{} is not a remap requirement",
                requirement.qualified_name()
            );
        }
        if let Some(old) = locked
            .remaps
            .iter()
            .find(|old| old.package == requirement.qualified_name())
        {
            match old.locked_package() {
                Ok(binding) => locked_sources.push(binding),
                Err(vibe_core::manifest::LockedRemapSourceError::MissingSource { .. }) => {
                    // A legacy registry alias is not endpoint/ref authority.
                    output.push(RemapPlanEntry {
                        package: old.package.clone(),
                        version: Some(old.version.clone()),
                        outcome: RemapPlanOutcome::NeedsInput,
                        input_required: Some(RemapInputRequired::MissingLockedSource),
                        source_candidates: None,
                        source_evidence: Some(vibe_wire::generated::shared::SourceEvidence {
                            registry: old.registry.clone(),
                            source_url: None,
                            source_ref: None,
                            resolved_commit: None,
                            source_kind: None,
                            via_redirect: None,
                            fetched_from: None,
                            acquisition: None,
                            store_reused: None,
                        }),
                    });
                    continue;
                }
                Err(error) => return Err(error.into()),
            }
        }
        let mut reference = requirement.clone();
        reference.kind = Some(PackageKind::Remap);
        new.push(reference);
    }
    if new.is_empty() {
        output.sort_by(|left, right| left.package.cmp(&right.package));
        return Ok(output);
    }
    let source = vibe_package_source::build_install_resolver_with_operation(
        vibe_package_source::InstallResolverRequest {
            options,
            manifest,
            embedded_root: None,
            project_root: root,
            offline,
            locked: &locked_sources,
            progress,
        },
        operation,
    )?;
    let roots = new
        .iter()
        .enumerate()
        .map(|(index, reference)| RootRequirement {
            id: EdgeId {
                owner: EdgeOwner::Root {
                    consumer: format!("{}/vibe.toml [requires.remaps]", root.display()),
                },
                site: RequirementSite::Requires(index),
            },
            requirement: OriginalRequirement {
                site: RequirementSite::Requires(index),
                activation: RequirementActivation::Mandatory,
                alternatives: vec![RequirementAlternative {
                    reference: reference.clone(),
                    registry_pin: None,
                }],
            },
        })
        .collect::<Vec<_>>();
    progress.checkpoint()?;
    let solved = source.solve_metadata(&SourceSolveRequest {
        roots: &roots,
        held_roots: &new,
        activated: &[],
        limits: source.source_solve_limits()?,
        configuration: None,
        blocked: &BTreeSet::new(),
        authoritative_locked: &locked_sources,
    })?;
    for reference in &new {
        let group = reference
            .group
            .as_ref()
            .ok_or_else(|| anyhow!("remap requirement is unqualified"))?;
        let metadata = solved
            .selected
            .get(&(group.clone(), reference.name.to_string()))
            .ok_or_else(|| {
                anyhow!(
                    "remap metadata selection omitted {}",
                    reference.qualified_name()
                )
            })?;
        let selected = &metadata.candidate;
        if selected.manifest().require_package()?.kind != PackageKind::Remap {
            bail!("{} is not a remap package", reference.qualified_name());
        }
        // Remap validation forbids requires/conditional/hooks/boot/installer
        // tables, so the offer metadata is its entire allowed package closure.
        output.push(RemapPlanEntry {
            package: reference.qualified_name(),
            version: Some(selected.version().to_string()),
            outcome: RemapPlanOutcome::OfferOnly,
            input_required: None,
            source_evidence: Some(selected.source_evidence().clone()),
            source_candidates: Some(crate::commands::install::source_candidate_rows(
                &metadata.observations,
            )),
        });
    }
    progress.checkpoint()?;
    output.sort_by(|left, right| left.package.cmp(&right.package));
    Ok(output)
}
