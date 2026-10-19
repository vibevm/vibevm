//! Retained exact inputs for read-only deploy inspection after API exit.
//! A fresh host grant remains mandatory; these local bytes grant no access.

use super::*;

const FORMAT: &str = "vibevm.deploy.bound-vibe-bin-input/1";
const CAPSULE_CAP: usize = 4 * C1_CAP + 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RootTokens {
    project: String,
    settings: String,
    user_home: String,
}

impl RootTokens {
    pub(super) fn capture(admitted: &AdmittedLocalDeployBinding) -> Result<Self> {
        Ok(Self {
            project: admitted
                .project()
                .root_correlation_token()
                .map_err(refused)?,
            settings: admitted
                .settings()
                .root_correlation_token()
                .map_err(refused)?,
            user_home: admitted
                .user_home()
                .root_correlation_token()
                .map_err(refused)?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputCapsule {
    format: String,
    run_id: String,
    request: String,
    plan: String,
    binding_id: String,
    roots: RootTokens,
}

fn name(run_id: &str) -> Result<String> {
    member(run_id, "input")
}

fn read_capsule(lease: &LifecycleLease, run_id: &str) -> Result<Option<InputCapsule>> {
    let Some(home) = lease.project().dir_if_present(&HOME).map_err(refused)? else {
        return Ok(None);
    };
    let Some(bytes) = lease
        .project()
        .read_file_bounded_in(&home, &name(run_id)?, CAPSULE_CAP)
        .map_err(refused)?
    else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&bytes)?))
}

fn canonical_value(bytes: &[u8]) -> Result<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    if serde_json::to_vec(&value)? != bytes {
        return Err(refused("deploy capsule holds noncanonical JSON"));
    }
    Ok(value)
}

fn validate(
    capsule: &InputCapsule,
    request: &[u8],
    bridge: &Bridge,
    binding: &EnvironmentOperationBinding,
    roots: &RootTokens,
) -> Result<()> {
    if capsule.format != FORMAT
        || capsule.run_id != bridge.run_id
        || capsule.binding_id != binding.binding_id
        || bridge.binding_id != binding.binding_id
        || capsule.roots != *roots
        || capsule.request.as_bytes() != request
        || sha(request) != bridge.request_sha256
        || sha(capsule.plan.as_bytes()) != bridge.plan_sha256
        || serde_json::to_vec(&bridge.plan)? != capsule.plan.as_bytes()
    {
        return Err(refused(
            "deploy capsule differs from bound request, plan or physical roots",
        ));
    }
    let request_value = canonical_value(request)?;
    let plan_value = canonical_value(capsule.plan.as_bytes())?;
    if request_value["binding"] != serde_json::to_value(binding)?
        || request_value["principal"] != bridge.principal
        || request_value["project"] != bridge.project
        || request_value["package"] != serde_json::to_value(&bridge.package)?
        || request_value["profile"] != bridge.profile
        || request_value["target"] != bridge.target
        || request_value["mechanism"] != "deploy:vibe-bin"
        || plan_value != bridge.plan
        || plan_value["home_id"] != bridge.home_id
        || plan_value["binding_id"] != bridge.binding_id
        || plan_value["bound_plan_hash"] != bridge.bound_plan_hash
    {
        return Err(refused("deploy capsule request or plan posture differs"));
    }
    Ok(())
}

pub(super) fn publish(
    lease: &LifecycleLease,
    run_id: &str,
    request: &[u8],
    plan: &[u8],
    binding: &EnvironmentOperationBinding,
    bridge: &Bridge,
    roots: &RootTokens,
) -> Result<()> {
    let capsule = InputCapsule {
        format: FORMAT.into(),
        run_id: run_id.into(),
        request: String::from_utf8(request.to_vec()).map_err(refused)?,
        plan: String::from_utf8(plan.to_vec()).map_err(refused)?,
        binding_id: binding.binding_id.clone(),
        roots: roots.clone(),
    };
    validate(&capsule, request, bridge, binding, roots)?;
    let bytes = serde_json::to_vec(&capsule)?;
    if bytes.len() > CAPSULE_CAP {
        return Err(refused("deploy input capsule exceeds cap"));
    }
    let home = lease.project().subproject(&HOME).map_err(refused)?;
    let dir = home.root_dir().map_err(refused)?;
    let name = name(run_id)?;
    home.publish_new_in(&dir, &name, &bytes).map_err(|error| {
        refused(format!(
            "deploy capsule publication: {:#}",
            error.into_report()
        ))
    })?;
    if home
        .read_file_bounded_in(&dir, &name, CAPSULE_CAP)
        .map_err(refused)?
        .as_deref()
        != Some(bytes.as_slice())
    {
        return Err(refused("deploy capsule changed on readback"));
    }
    Ok(())
}

pub(super) fn verify_existing(
    lease: &LifecycleLease,
    run_id: &str,
    request: &[u8],
    bridge: &Bridge,
    binding: &EnvironmentOperationBinding,
    roots: &RootTokens,
) -> Result<()> {
    let capsule =
        read_capsule(lease, run_id)?.ok_or_else(|| refused("deploy input capsule is absent"))?;
    validate(&capsule, request, bridge, binding, roots)
}

pub(super) fn verify_retained(
    lease: &LifecycleLease,
    run_id: &str,
    bridge: &Bridge,
    binding: &EnvironmentOperationBinding,
    roots: &RootTokens,
) -> Result<()> {
    let capsule = read_capsule(lease, run_id)?.ok_or_else(|| refused("retained capsule absent"))?;
    validate(&capsule, capsule.request.as_bytes(), bridge, binding, roots)
}

pub(super) fn retained_target(
    lease: &LifecycleLease,
    run_id: &str,
) -> Result<vibe_core::manifest::DeployTarget> {
    let capsule = read_capsule(lease, run_id)?.ok_or_else(|| refused("retained input absent"))?;
    let value = canonical_value(capsule.request.as_bytes())?;
    let text = |key: &str| {
        value[key]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| refused("retained target field absent"))
    };
    let config = if value["config"].is_null() {
        None
    } else {
        Some(vibe_core::manifest::ExtensionConfig::from_table(
            serde_json::from_value::<toml::Table>(value["config"].clone())?,
        ))
    };
    Ok(vibe_core::manifest::DeployTarget {
        id: text("target")?,
        artifact: text("artifact")?,
        mechanism: text("mechanism")?.parse().map_err(refused)?,
        provider: None,
        when: None,
        depends_on: Some(Vec::new()),
        config,
    })
}

/// Historical inspection requires a newly issued, held host grant. Capsule
/// paths are values only; the settings Project is never opened from them.
pub(crate) fn inspect_host_admitted_bound_vibe_bin_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    admitted: &AdmittedLocalDeployBinding,
) -> Result<Option<BoundDeployBridgeInspection>> {
    lease.ensure_root(
        admitted.project().root_path(),
        "at cold bound deploy inspect",
    )?;
    admitted.reobserve_roots().map_err(refused)?;
    let binding = admitted.binding();
    if binding.host_context_id != scope.context
        || binding.instance_ref != scope.target
        || binding.principal_ref != scope.principal
    {
        return Err(refused("cold deploy grant differs from trusted scope"));
    }
    let roots = RootTokens::capture(admitted)?;
    let namespace = SubmissionNamespace::from_host_token(namespace_token.to_owned())?;
    let Some(existing) =
        SubmissionReservations::inspect_existing_key(lease, &namespace, scope, key)?
    else {
        return Ok(None);
    };
    let run_id = existing.domain_run_id;
    let bridge = match read::<Bridge>(lease, &run_id, "bridge") {
        Ok(Some(bridge)) => bridge,
        _ => {
            return Ok(Some(BoundDeployBridgeInspection::RecoveryRequired {
                run_id,
            }));
        }
    };
    if bridge.binding_id != binding.binding_id
        || bridge.instance_ref != scope.target
        || bridge.principal != scope.principal
    {
        return Ok(Some(BoundDeployBridgeInspection::RecoveryRequired {
            run_id,
        }));
    }
    let settings = admitted.settings();
    let home = DeploymentHome::new_bound(
        &settings.root_path().join("state/deployments"),
        &bridge.project,
        bridge.package.as_deref(),
        &bridge.target,
        binding,
    )?;
    let capsule = match read_capsule(lease, &run_id) {
        Ok(Some(capsule)) => capsule,
        Ok(None) if existing.journal_admission == JournalAdmission::Ready => {
            return Ok(Some(BoundDeployBridgeInspection::Uncertain {
                partial: partial(settings, &home),
                run_id,
            }));
        }
        Ok(None) => {
            return Ok(Some(BoundDeployBridgeInspection::RecoveryRequired {
                run_id,
            }));
        }
        Err(_) => {
            return Ok(Some(BoundDeployBridgeInspection::Uncertain {
                partial: partial(settings, &home),
                run_id,
            }));
        }
    };
    if capsule.binding_id != binding.binding_id || capsule.roots != roots {
        return Err(refused(
            "cold deploy grant differs from retained binding or physical roots",
        ));
    }
    if sha(capsule.request.as_bytes()) != existing.request_sha256
        || sha(capsule.plan.as_bytes()) != existing.plan_sha256
    {
        return Ok(Some(BoundDeployBridgeInspection::Uncertain {
            partial: partial(settings, &home),
            run_id,
        }));
    }
    if validate(
        &capsule,
        capsule.request.as_bytes(),
        &bridge,
        binding,
        &roots,
    )
    .is_err()
    {
        return Ok(Some(BoundDeployBridgeInspection::Uncertain {
            partial: partial(settings, &home),
            run_id,
        }));
    }
    let result = inspect(
        lease,
        &namespace,
        scope,
        key,
        capsule.request.as_bytes(),
        run_id,
        existing.journal_admission,
        existing.reservation_ref,
        settings,
        &home,
        binding,
        &roots,
    )?;
    Ok(Some(result))
}
