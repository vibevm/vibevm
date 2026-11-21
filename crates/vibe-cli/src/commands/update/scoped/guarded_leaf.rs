//! Exactly one guarded optional plain-copy leaf; all other graph changes refuse.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PERSISTENCE");
use super::*;
use std::collections::BTreeSet;
use vibe_core::manifest::{
    ConfigurationEffect, ConfigurationGroupDecl, LockedPackage, Materialization,
    PackageConfiguration,
};
use vibe_workspace::install::{CheckedRemovalPlan, PreparedRetirementSlot};

pub(super) struct Reconciliation {
    pub(super) base: Lockfile,
    pub(super) removal: Option<CheckedRemovalPlan>,
    // Keep independently verified original bytes/reference alive until apply.
    _reference: Option<Reference>,
}
struct Reference {
    source: vibe_registry::local_preview::PreparedRetirementSource,
    _staging: tempfile::TempDir,
    slot: std::path::PathBuf,
}
impl PreparedRetirementSlot for Reference {
    fn package(&self) -> &LockedPackage {
        self.source.package()
    }
    fn expected_slot(&self) -> &Path {
        &self.slot
    }
    fn recheck_source(&self) -> Result<(), String> {
        self.source.recheck().map_err(|error| error.to_string())
    }
}
fn denied() -> anyhow::Error {
    anyhow::anyhow!(
        "configured whole-workspace update requires explicit graph membership reconciliation before effects"
    )
}
fn guarded_target(record: &PackageConfiguration) -> Option<String> {
    let mut targets = BTreeSet::new();
    for group in &record.definition.groups {
        let ConfigurationGroupDecl::Choice {
            members, guards, ..
        } = group
        else {
            continue;
        };
        if guards.is_empty() {
            continue;
        }
        for feature in &record.definition.features {
            if !members.contains(&feature.id) {
                continue;
            }
            for effect in &feature.effects {
                if let ConfigurationEffect::Dependency { target } = effect {
                    targets.insert(target.clone());
                }
            }
        }
    }
    let mut targets = targets.into_iter();
    match (targets.next(), targets.next()) {
        (Some(only), None) => Some(only),
        _ => None,
    }
}

fn same_source(old: &LockedPackage, cached: &CachedPackage) -> bool {
    let projected = super::super::inputs::locked_package(
        cached,
        &old.dependencies,
        Some(old),
        &old.embedded_sources,
    );
    projected.registry == old.registry
        && projected.source_url == old.source_url
        && projected.source_ref == old.source_ref
        && projected.resolved_commit == old.resolved_commit
        && projected.source_kind == old.source_kind
        && projected.overridden == old.overridden
        && projected.via_override == old.via_override
        && projected.via_redirect == old.via_redirect
        && projected.materialization == old.materialization
        && projected.template == old.template
}
fn local_plain(cached: &CachedPackage) -> bool {
    cached.package_meta().materialization == Materialization::Copy
        && cached.source_uri.starts_with("file://")
        && !cached.overridden
        && cached.via_redirect.is_none()
        && cached.template.is_none()
        && cached.manifest.configuration.is_none()
        && cached.manifest.requires.is_empty()
        && cached.manifest.requires_any.is_empty()
        && cached.manifest.recommends.is_empty()
}
fn selected_guard_cause(record: &PackageConfiguration, target: &str) -> bool {
    record.dependency_causes.get(target).is_some_and(|causes|causes.iter().any(|id| {
        record.definition.groups.iter().any(|group|matches!(group,ConfigurationGroupDecl::Choice{members,guards,..} if !guards.is_empty() && members.contains(id)))
    }))
}

fn guard_activity(record: &PackageConfiguration, target: &str) -> Option<bool> {
    let (consumer, input) = record.consumers.iter().next()?;
    let normalized = record.definition.normalize_input(input, consumer).ok()?;
    let evaluation = record
        .definition
        .compile_choices()
        .ok()?
        .evaluate(&normalized.features, &[normalized.choices]);
    let mut groups=record.definition.groups.iter().filter_map(|group| {
        let ConfigurationGroupDecl::Choice{id,members,guards,..}=group else{return None};
        (!guards.is_empty() && record.definition.features.iter().any(|feature|members.contains(&feature.id) && feature.effects.iter().any(|effect|matches!(effect,ConfigurationEffect::Dependency{target:effect_target} if effect_target==target)))).then_some(id)
    });
    let id = groups.next()?;
    if groups.next().is_some() {
        return None;
    }
    evaluation
        .controls
        .iter()
        .find(|control| &control.id == id)
        .map(|control| control.active)
}

pub(super) fn prepare(
    workspace: &Workspace,
    old: &Lockfile,
    updated: &[Resolved],
    resolver: &vibe_package_source::InstallResolver,
    configured_all: bool,
    spec_format: vibe_core::manifest::SpecFormat,
) -> Result<Reconciliation> {
    let previous = old
        .packages
        .iter()
        .map(|row| format!("{}/{}", row.group, row.name))
        .collect::<BTreeSet<_>>();
    let selected = updated
        .iter()
        .map(|(cached, ..)| format!("{}/{}", cached.resolved.group, cached.resolved.name))
        .collect::<BTreeSet<_>>();
    let delta = previous
        .symmetric_difference(&selected)
        .cloned()
        .collect::<Vec<_>>();
    if delta.is_empty() {
        return Ok(Reconciliation {
            base: old.clone(),
            removal: None,
            _reference: None,
        });
    }
    if !configured_all
        || delta.len() != 1
        || !workspace.is_standalone()
        || spec_format != vibe_core::manifest::SpecFormat::Mixed
        || old.configurations.len() != 1
        || old.packages.iter().any(|row| {
            row.materialization != Materialization::Copy
                || !row.source_url.as_str().starts_with("file://")
        })
    {
        return Err(denied());
    }
    let Some((root_key, prior)) = old.configurations.iter().next() else {
        return Err(denied());
    };
    let Some((cached, _, _, _, Some(next))) = updated.iter().find(|(cached, ..)| {
        format!("{}/{}", cached.resolved.group, cached.resolved.name) == *root_key
    }) else {
        return Err(denied());
    };
    let root = old
        .packages
        .iter()
        .find(|row| format!("{}/{}", row.group, row.name) == *root_key)
        .ok_or_else(denied)?;
    if !same_source(root, cached)
        || root.version != cached.resolved.version
        || root.content_hash.as_str() != cached.content_hash
        || prior.definition != next.definition
        || prior.declarations != next.declarations
        || !prior.definition.parameters.is_empty()
        || !prior.definition.profiles.is_empty()
        || prior.automatic_basis.is_some()
        || next.automatic_basis.is_some()
        || !prior.forwarded_inputs.is_empty()
        || !next.forwarded_inputs.is_empty()
        || prior.consumers.len() != 1
        || next.consumers.len() != 1
        || guarded_target(prior).as_ref() != Some(&delta[0])
    {
        return Err(denied());
    }
    // Unrelated coordinates must keep exact version/content/source identity.
    for oldrow in &old.packages {
        let key = format!("{}/{}", oldrow.group, oldrow.name);
        if key == delta[0] || key == *root_key {
            continue;
        }
        let Some((current, ..)) = updated.iter().find(|(cached, ..)| {
            cached.resolved.group == oldrow.group && cached.resolved.name == oldrow.name.as_str()
        }) else {
            return Err(denied());
        };
        if !same_source(oldrow, current)
            || current.resolved.version != oldrow.version
            || current.content_hash != oldrow.content_hash.as_str()
        {
            return Err(denied());
        }
    }
    let mut base = old.clone();
    if previous.contains(&delta[0]) {
        let leaf = old
            .packages
            .iter()
            .find(|row| format!("{}/{}", row.group, row.name) == delta[0])
            .ok_or_else(denied)?;
        if !leaf.dependencies.is_empty()
            || old.configurations.contains_key(&delta[0])
            || prior.declarations.optional.get(&delta[0]) != Some(&true)
            || !prior.active_dependencies.contains(&delta[0])
            || next.active_dependencies.contains(&delta[0])
            || !selected_guard_cause(prior, &delta[0])
            || guard_activity(prior, &delta[0]) != Some(true)
            || guard_activity(next, &delta[0]) != Some(false)
        {
            return Err(denied());
        }
        let source = resolver.prepare_exact_retirement_source(leaf)?;
        let staging = tempfile::Builder::new()
            .prefix("vibe-retirement-reference-")
            .tempdir()?;
        source.recheck()?;
        vibedeps::materialise(
            staging.path(),
            &leaf.group,
            leaf.name.as_str(),
            &leaf.version,
            source.frozen_tree(),
            &leaf.content_hash,
        )?;
        source.recheck()?;
        let reference = Reference {
            slot: vibedeps::slot_abs_path(
                staging.path(),
                &leaf.group,
                leaf.name.as_str(),
                &leaf.version,
            ),
            source,
            _staging: staging,
        };
        let removal = CheckedRemovalPlan::capture(&workspace.root, leaf, &reference)?;
        base.packages
            .retain(|row| format!("{}/{}", row.group, row.name) != delta[0]);
        base.configurations.remove(&delta[0]);
        Ok(Reconciliation {
            base,
            removal: Some(removal),
            _reference: Some(reference),
        })
    } else {
        let Some((leaf, dependencies, ..)) = updated.iter().find(|(cached, ..)| {
            format!("{}/{}", cached.resolved.group, cached.resolved.name) == delta[0]
        }) else {
            return Err(denied());
        };
        if !dependencies.is_empty()
            || !local_plain(leaf)
            || prior.declarations.optional.get(&delta[0]) != Some(&true)
            || prior.active_dependencies.contains(&delta[0])
            || !next.active_dependencies.contains(&delta[0])
            || !selected_guard_cause(next, &delta[0])
            || guard_activity(prior, &delta[0]) != Some(false)
            || guard_activity(next, &delta[0]) != Some(true)
        {
            return Err(denied());
        }
        let slot = vibedeps::slot_abs_path(
            &workspace.root,
            &leaf.resolved.group,
            &leaf.resolved.name,
            &leaf.resolved.version,
        );
        // Adding the leaf never adopts a foreign incumbent at the new name.
        vibe_safefs::Project::pin_absent_path(&slot)?;
        Ok(Reconciliation {
            base,
            removal: None,
            _reference: None,
        })
    }
}
