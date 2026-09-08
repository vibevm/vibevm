//! Pure active-target projection into exact native mechanism bindings.

use std::collections::BTreeMap;

use vibe_core::manifest::{
    ArtifactBuildTarget, ArtifactPackageTarget, DeployTarget, ExtensionHandler, MechanismKey,
    MechanismRoutes, ProviderPin,
};
use vibe_extension_registry::{MechanismRegistry, resolve_mechanism};

use super::{
    NativeArtifactError, NativeMechanismBinding, NativeMechanismPlan, PlannedMechanism,
    selection_error,
};

pub fn project_native_target_mechanisms(
    build: &[ArtifactBuildTarget],
    package: &[ArtifactPackageTarget],
    deploy: &[DeployTarget],
    registry: &MechanismRegistry,
    routes: &MechanismRoutes,
) -> Result<NativeMechanismPlan, NativeArtifactError> {
    let mut plan = NativeMechanismPlan::default();
    let mut selected_pins = BTreeMap::<MechanismKey, String>::new();
    for target in build {
        push(
            &mut plan,
            &mut selected_pins,
            &target.id,
            &target.mechanism,
            target.provider.as_ref(),
            registry,
            routes,
        )?;
    }
    for target in package {
        push(
            &mut plan,
            &mut selected_pins,
            &target.id,
            &target.mechanism,
            target.provider.as_ref(),
            registry,
            routes,
        )?;
    }
    for target in deploy {
        push(
            &mut plan,
            &mut selected_pins,
            &target.id,
            &target.mechanism,
            target.provider.as_ref(),
            registry,
            routes,
        )?;
    }
    Ok(plan)
}

pub fn project_native_mechanisms(
    package: &[ArtifactPackageTarget],
    deploy: &[DeployTarget],
    registry: &MechanismRegistry,
    routes: &MechanismRoutes,
) -> Result<NativeMechanismPlan, NativeArtifactError> {
    project_native_target_mechanisms(&[], package, deploy, registry, routes)
}

#[allow(clippy::too_many_arguments)]
fn push(
    plan: &mut NativeMechanismPlan,
    selected_pins: &mut BTreeMap<MechanismKey, String>,
    target: &str,
    key: &MechanismKey,
    provider: Option<&ProviderPin>,
    registry: &MechanismRegistry,
    routes: &MechanismRoutes,
) -> Result<(), NativeArtifactError> {
    let selected = resolve_mechanism(registry, key, provider, routes)
        .map_err(|error| selection_error(&error.to_string()))?;
    let row = selected.row();
    let pin = row.pin().to_string();
    if row.protocol() != 1 {
        return Err(selection_error(
            "selected native mechanism protocol differs from epoch 1",
        ));
    }
    if let Some(previous) = selected_pins.insert(key.clone(), pin.clone())
        && previous != pin
    {
        return Err(selection_error(
            "active targets select different exact provider pins for one logical mechanism key",
        ));
    }
    if row.is_builtin() || !matches!(row.handler(), ExtensionHandler::Native { .. }) {
        return Ok(());
    }
    let binding = NativeMechanismBinding {
        target: target.to_owned(),
        key: key.clone(),
        pin: pin.clone(),
        descriptor_id: row.declaration().id.clone(),
        protocol: row.protocol(),
        via: selected.via(),
        displaced_default: selected
            .displaced_default()
            .map(|default| default.pin().to_string()),
    };
    if let Some(existing) = plan
        .entries
        .iter_mut()
        .find(|entry| entry.row.pin().to_string() == pin)
    {
        if existing.row.key() != row.key()
            || existing.row.handler() != row.handler()
            || existing.row.declaration().id != row.declaration().id
        {
            return Err(selection_error(
                "one exact pin resolved to conflicting rows",
            ));
        }
        existing.bindings.push(binding);
    } else {
        plan.entries.push(PlannedMechanism {
            row: row.clone(),
            bindings: vec![binding],
        });
    }
    Ok(())
}
