//! Shared visibility projection used by ordinary and configured solver routes.
use std::collections::BTreeSet;
use vibe_core::{
    PackageRef,
    manifest::{Manifest, Requires},
};

/// Apply the existing visibility edge mask to every ordinary dependency surface.
/// The caller supplies a derived manifest; source definitions stay unchanged.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-050#effective-set")]
pub fn mask_manifest_edges(manifest: &mut Manifest, targets: &BTreeSet<String>) {
    prune_requires(&mut manifest.requires, targets);
    manifest
        .recommends
        .packages
        .retain(|dependency| !targets.contains(&dependency.qualified_name()));
    for disjunction in &mut manifest.requires_any {
        disjunction
            .one_of
            .retain(|dependency| !targets.contains(&dependency.qualified_name()));
    }
    manifest
        .requires_any
        .retain(|disjunction| !disjunction.one_of.is_empty());
    for conditional in manifest.conditional_deps.values_mut() {
        prune_requires(&mut conditional.dependencies, targets);
    }
    let blocked = |requirement: &vibe_core::manifest::RequirementDecl| {
        PackageRef::parse(requirement.package())
            .is_ok_and(|reference| targets.contains(&reference.qualified_name()))
    };
    for option in &mut manifest.installer_options {
        option.requires.retain(|requirement| !blocked(requirement));
        for step in &mut option.steps {
            step.requires.retain(|requirement| !blocked(requirement));
        }
    }
}

fn prune_requires(requires: &mut Requires, targets: &BTreeSet<String>) {
    requires
        .packages
        .retain(|dependency| !targets.contains(&dependency.qualified_name()));
    requires.git_packages.retain(|dependency| {
        !targets.contains(&format!("{}/{}", dependency.group, dependency.name))
    });
    requires.path_packages.retain(|dependency| {
        !targets.contains(&format!("{}/{}", dependency.group, dependency.name))
    });
    requires.var_packages.retain(|dependency| {
        !targets.contains(&format!("{}/{}", dependency.group, dependency.name))
    });
    requires.links.retain(|target, _| !targets.contains(target));
    requires
        .optional
        .retain(|target, _| !targets.contains(target));
    requires
        .configurations
        .retain(|target, _| !targets.contains(target));
    requires
        .accesses
        .retain(|target, _| !targets.contains(target));
    requires
        .friend_flags
        .retain(|target, _| !targets.contains(target));
    requires
        .excludes
        .retain(|target, _| !targets.contains(target));
}
