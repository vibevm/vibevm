//! Canonical C1 intent bytes for legacy and explicitly profiled Deploy.
use super::*;

#[derive(Clone, Copy)]
pub(crate) enum RequestMode {
    Legacy,
    Profiled,
}

pub(crate) fn canonical_request(
    execution: &DeployExecution<'_>,
    binding: &EnvironmentOperationBinding,
    principal: &str,
) -> Result<Vec<u8>> {
    canonical_request_with_mode(execution, binding, principal, RequestMode::Legacy)
}

pub(crate) fn canonical_profiled_request(
    execution: &DeployExecution<'_>,
    binding: &EnvironmentOperationBinding,
    principal: &str,
) -> Result<Vec<u8>> {
    canonical_request_with_mode(execution, binding, principal, RequestMode::Profiled)
}

pub(super) fn canonical_request_with_mode(
    execution: &DeployExecution<'_>,
    binding: &EnvironmentOperationBinding,
    principal: &str,
    mode: RequestMode,
) -> Result<Vec<u8>> {
    let [target_id] = execution.selection.targets.as_slice() else {
        return Err(refused("one selected target required"));
    };
    let target = execution
        .targets
        .iter()
        .find(|row| &row.id == target_id)
        .ok_or_else(|| refused("selected target absent"))?;
    if target.mechanism.to_string() != "deploy:vibe-bin"
        || target.when.is_some()
        || target
            .depends_on
            .as_ref()
            .is_some_and(|rows| !rows.is_empty())
    {
        return Err(refused(
            "only unconditional independent vibe-bin is admitted",
        ));
    }
    let path = |value: &std::path::Path| -> Result<String> {
        if !value.is_absolute() {
            return Err(refused("root is not absolute"));
        }
        value
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| refused("root is not UTF-8"))
    };
    let mut value = serde_json::json!({
        "format": "vibevm.deploy.bound-vibe-bin-request/1",
        "binding": binding, "principal": principal,
        "project_root": path(execution.project_root)?,
        "settings_root": path(execution.settings_root)?,
        "user_home": path(execution.user_home)?,
        "state_home": path(execution.state_home)?,
        "project": execution.project, "package": execution.package,
        "profile": execution.selection.profile,
        "target": target.id, "artifact": target.artifact,
        "mechanism": target.mechanism.to_string(),
        "provider": target.provider.as_ref().map(ToString::to_string),
        "config": target.config.as_ref().map(|row| row.as_table()),
    });
    if matches!(mode, RequestMode::Profiled) {
        let Some(object) = value.as_object_mut() else {
            return Err(refused("canonical deploy request is not an object"));
        };
        object.insert(
            "requested_deploy_profile".into(),
            execution.selection.profile.clone().into(),
        );
    }
    let bytes = serde_json::to_vec(&value)?;
    if bytes.len() > C1_CAP {
        return Err(refused("request exceeds C.1 cap"));
    }
    Ok(bytes)
}
