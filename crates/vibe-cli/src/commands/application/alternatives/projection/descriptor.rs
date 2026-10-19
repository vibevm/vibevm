//! Resolve one authored public command into a static proxy descriptor.

use super::super::super::launch::plan_public_launcher;
use super::*;

pub(super) fn descriptor_for(
    record: &ApplicationRecord,
    name: &str,
    settings: &Path,
    key: &str,
) -> Result<Value> {
    if record.status != ApplicationStatus::Ready {
        return Ok(
            json!({"protocol":PROXY_PROTOCOL,"mode":"unavailable","reason":"installation is not deployed"}),
        );
    }
    let Some(binding) = record
        .public_commands
        .iter()
        .find(|command| command.name == name)
    else {
        return Ok(
            json!({"protocol":PROXY_PROTOCOL,"mode":"unavailable","reason":"command is not provided by this installation"}),
        );
    };
    let plan = match plan_public_launcher(record, &binding.launcher, settings) {
        Ok(plan) => plan,
        Err(error) if error.to_string() == "Windows executable cannot run on this platform" => {
            return Ok(json!({
                "protocol": PROXY_PROTOCOL,
                "mode": "unavailable",
                "reason": "Windows executable cannot run on this platform",
            }));
        }
        Err(error) => {
            return Err(error).with_context(|| {
                format!("public command `{name}` has an unavailable declared launcher")
            });
        }
    };
    let lease = super::super::super::lease::path(settings, key)?;
    Ok(json!({
                "protocol": PROXY_PROTOCOL,
                "mode": "direct",
                "target": plan.target,
                "args": plan.args,
                "cwd": plan.cwd,
                "lease": lease,
    }))
}
