//! The one semantic validator for the build/package/deploy plane.
//!
//! [`validate_plane`] is the *only* place the mechanism / artifact / deploy
//! laws are decided, and every path into a `Manifest` runs it: parsing calls
//! it through [`Manifest::validate`](super::Manifest::validate), and
//! serialisation calls it through `TryFrom<Manifest> for ManifestWire`. A
//! document that cannot be read back can therefore never be written — the
//! two directions cannot drift because there is only one law to drift from.
//!
//! What lives here is exactly what needs the *whole document*: the role
//! matrix, cross-section identity, and the local-pin cross-check. Row shape
//! stays on the row types next door.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::collections::{BTreeMap, BTreeSet};

use super::artifact::ArtifactsSection;
use super::document::Manifest;
use super::mechanism::{
    MechanismDecl, MechanismKey, ProviderOwner, ProviderPin, validate_mechanism_declarations,
};

const ONE_MACHINE: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE";
const ARTIFACT_REGISTRY: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY";
const DEPLOY_TARGETS: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS";

/// Validate the whole build/package/deploy plane of one document.
///
/// Order is the diagnostic contract: declarations, then the section role
/// law, then local pin routing, then the artifact graph, then the deploy
/// graph against the artifacts it consumes.
pub(crate) fn validate_plane(manifest: &Manifest) -> Result<(), String> {
    let has_project = manifest.project.is_some();
    let has_package = manifest.package.is_some();

    // The self-coordinate law runs for *every* manifest, not only one that
    // happens to declare a mechanism: a grouped node that cannot form its own
    // coordinate is broken whether or not anything points at it yet. Running
    // it first is also what makes the owner total below.
    let owner = local_provider_owner(manifest)?;
    validate_mechanism_declarations(&manifest.mechanism_decls, has_project, has_package)?;
    validate_section_roles(manifest, has_project, has_package)?;
    validate_local_pins(manifest, owner)?;

    let producers = manifest
        .artifacts
        .as_ref()
        .map(ArtifactsSection::validate)
        .transpose()?;

    if let Some(deploy) = &manifest.deploy {
        let artifact_ids = producers
            .as_ref()
            .map(|index| index.keys().cloned().collect::<BTreeSet<String>>())
            .unwrap_or_default();
        deploy.validate(&artifact_ids)?;
    }
    Ok(())
}

/// Desired targets need a `[project]` or `[package]` role. A pure virtual
/// `[workspace]` may still *route* `[mechanisms]` — routing is a host
/// control, mirroring `[extensions]`; declaring and desiring are not.
fn validate_section_roles(
    manifest: &Manifest,
    has_project: bool,
    has_package: bool,
) -> Result<(), String> {
    if has_project || has_package {
        return Ok(());
    }
    if manifest.artifacts.is_some() {
        return Err(format!(
            "[artifacts] desired targets require a `[project]` or `[package]` role; a pure \
             virtual `[workspace]` may route `[mechanisms]` but declares no build or package \
             targets (violates {ARTIFACT_REGISTRY}; fix: move the targets to a project or \
             package manifest)"
        ));
    }
    if manifest.deploy.is_some() {
        return Err(format!(
            "[deploy] desired targets require a `[project]` or `[package]` role; a pure virtual \
             `[workspace]` may route `[mechanisms]` but declares no deploy targets (violates \
             {DEPLOY_TARGETS}; fix: move the deploy section to a project or package manifest)"
        ));
    }
    Ok(())
}

/// This manifest's own provider-owner identity, if it can declare providers.
///
/// This is the already-landed R2 `HostIdentity` law, reused rather than
/// reinterpreted (`vibe-extension-registry`'s `HostIdentity`, built in
/// `vibe-cli` `lifecycle::world::host_source`):
///
/// - `[package]` → the package coordinate `<group>/<package-name>`;
/// - a **grouped** `[project]` → its real self-coordinate
///   `<group>/<project-name>` — `HostIdentity::Coordinate`, not the host token;
/// - an **ungrouped** `[project]` → `__host__/<project-name>`, the reserved
///   opaque spelling of SPEC-DEBT-LIFECYCLE §8.3, because there is no group to
///   form a coordinate from;
/// - a pure virtual `[workspace]` → nothing: it declares no provider.
///
/// The function is **total** over these four cases: `Ok(None)` means "this
/// node declares no provider", never "the coordinate did not come out". A
/// coordinate that cannot be built is a defect in the manifest and is
/// returned as an error, so parser and writer both refuse rather than
/// silently skipping the cross-check.
pub(crate) fn local_provider_owner(manifest: &Manifest) -> Result<Option<ProviderOwner>, String> {
    if let Some(package) = &manifest.package {
        let name = coordinate_name(&package.name, "[package].name", &package.group)?;
        return Ok(Some(ProviderOwner::Package {
            group: package.group.clone(),
            package: name,
        }));
    }
    let Some(project) = &manifest.project else {
        // A pure virtual `[workspace]`: an explicit "declares nothing" branch,
        // not a fallback.
        return Ok(None);
    };
    match &project.group {
        // Grouped: the group makes this a real self-coordinate, so the name
        // has to be a real package name.
        Some(group) => Ok(Some(ProviderOwner::Package {
            group: group.clone(),
            package: coordinate_name(&project.name, "[project].name", group)?,
        })),
        // Ungrouped: no group, no coordinate — the opaque host owner, whose
        // name stays an arbitrary String behind the shared percent codec.
        None => Ok(Some(ProviderOwner::Host {
            project: project.name.clone(),
        })),
    }
}

/// The name half of a `<group>/<name>` self-coordinate. A grouped node claims
/// a coordinate every consumer can address, so its name answers to the one
/// package-name grammar — the same one `<group>/<package>` uses everywhere
/// else. An ungrouped project claims no coordinate and is never asked.
fn coordinate_name(
    name: &str,
    field: &str,
    group: &crate::Group,
) -> Result<crate::PackageName, String> {
    crate::PackageName::parse(name).map_err(|_| {
        format!(
            "{field} value `{name}` is not a valid package name, but `group = \"{group}\"` \
             declares the self-coordinate `{group}/{name}`; a coordinate name is one or more \
             lowercase alphanumeric segments joined by single hyphens ({ONE_MACHINE}; fix: \
             rename to a kebab-case name, or drop `group` — an ungrouped project keeps an \
             arbitrary name and is addressed as `__host__/<name>`)"
        )
    })
}

/// Cross-check every pin that names *this* manifest's own owner — package
/// coordinate or project host alike — against the `[[mechanism]]` it must
/// resolve to. A pin into an installed foreign package, or into another
/// project's host, cannot be judged without the installed world and stays
/// runtime debt; a manifest pinning itself is decidable now, and a self-pin
/// naming a missing id or the wrong capability is a defect the author can fix
/// while reading their own file.
fn validate_local_pins(manifest: &Manifest, owner: Option<ProviderOwner>) -> Result<(), String> {
    // `None` here means exactly one thing — a pure virtual workspace owns no
    // provider identity. A coordinate that failed to build already errored.
    let Some(owner) = owner else {
        return Ok(());
    };
    let declared: BTreeMap<&str, &MechanismDecl> = manifest
        .mechanism_decls
        .iter()
        .map(|decl| (decl.id.as_str(), decl))
        .collect();

    for (key, pin) in manifest.mechanism_routes.iter() {
        check_local_pin(
            &owner,
            &declared,
            key,
            pin,
            &format!("[mechanisms] route `{key}`"),
        )?;
    }
    if let Some(artifacts) = &manifest.artifacts {
        for (role, target) in artifacts.all_targets() {
            let Some(pin) = &target.provider else {
                continue;
            };
            check_local_pin(
                &owner,
                &declared,
                &target.mechanism,
                pin,
                &format!("[[artifacts.{role}]] `{}` field `provider`", target.id),
            )?;
        }
    }
    if let Some(deploy) = &manifest.deploy {
        for target in &deploy.targets {
            let Some(pin) = &target.provider else {
                continue;
            };
            check_local_pin(
                &owner,
                &declared,
                &target.mechanism,
                pin,
                &format!("[[deploy.target]] `{}` field `provider`", target.id),
            )?;
        }
    }
    Ok(())
}

fn check_local_pin(
    owner: &ProviderOwner,
    declared: &BTreeMap<&str, &MechanismDecl>,
    key: &MechanismKey,
    pin: &ProviderPin,
    site: &str,
) -> Result<(), String> {
    if pin.owner() != owner {
        // A foreign package coordinate or a foreign project host: resolution
        // against the installed world is runtime debt, not manifest grammar.
        return Ok(());
    }
    let whose = match owner {
        ProviderOwner::Package { .. } => "this package's own coordinate",
        ProviderOwner::Host { .. } => "this project's own host coordinate",
    };
    let Some(decl) = declared.get(pin.id()) else {
        return Err(format!(
            "{site} pins `{pin}`, which names {whose}, but no `[[mechanism]]` with id `{}` is \
             declared in this manifest ({ONE_MACHINE}; fix: declare it here, or pin the \
             manifest that does)",
            pin.id(),
        ));
    };
    if decl.role != key.role() || decl.name != key.name() {
        return Err(format!(
            "{site} pins `{pin}`, declared in this manifest as `{}:{}`, but the logical key is \
             `{key}`; a pin must provide the capability it is selected for ({ONE_MACHINE}; fix: \
             align the key with the provider's `role`/`name`, or pin a different provider)",
            decl.role, decl.name,
        ));
    }
    Ok(())
}

/// Iterative three-colour DFS over `consumer -> producer` edges.
///
/// Deterministic: roots are visited in the map's sorted key order and
/// neighbours in authored order, so the reported cycle is the same on every
/// run and on every platform. Iterative rather than recursive so a deep
/// dependency chain in an installed package's manifest reports a diagnostic
/// instead of overflowing the stack.
pub(crate) fn assert_acyclic(
    edges: &BTreeMap<&str, Vec<&str>>,
    subject: &str,
    anchor: &str,
    fix: &str,
) -> Result<(), String> {
    #[derive(Clone, Copy, PartialEq)]
    enum Colour {
        Grey,
        Black,
    }
    let mut colours: BTreeMap<&str, Colour> = BTreeMap::new();

    for root in edges.keys().copied() {
        if colours.contains_key(root) {
            continue;
        }
        colours.insert(root, Colour::Grey);
        // `frames` holds (node, index of the next neighbour to walk);
        // `path` is the grey stack the cycle text is read off.
        let mut frames: Vec<(&str, usize)> = vec![(root, 0)];
        let mut path: Vec<&str> = vec![root];

        while let Some(&(node, cursor)) = frames.last() {
            let neighbours = edges.get(node).map(Vec::as_slice).unwrap_or(&[]);
            let Some(&next) = neighbours.get(cursor) else {
                colours.insert(node, Colour::Black);
                frames.pop();
                path.pop();
                continue;
            };
            if let Some(frame) = frames.last_mut() {
                frame.1 = cursor + 1;
            }
            match colours.get(next) {
                Some(Colour::Black) => {}
                Some(Colour::Grey) => {
                    let from = path
                        .iter()
                        .position(|seen| *seen == next)
                        .unwrap_or_default();
                    let mut cycle = path[from..].join(" -> ");
                    cycle.push_str(" -> ");
                    cycle.push_str(next);
                    return Err(format!(
                        "{subject} is cyclic: {cycle} (violates {anchor}; fix: {fix})"
                    ));
                }
                None => {
                    colours.insert(next, Colour::Grey);
                    frames.push((next, 0));
                    path.push(next);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "plane/tests.rs"]
mod tests;
