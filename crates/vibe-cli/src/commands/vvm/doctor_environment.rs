//! Explicit consent and read-only observation use the same persister target as repair.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3);
use super::super::{VvmEnv, env, error::VvmError, make_persister};
use crate::output;
use std::{io::IsTerminal, path::Path};

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
pub(super) fn observe(environment: &VvmEnv, shim_dir: &Path) -> env::EnvironmentObservation {
    let shell = env::Shell::detect(environment.shell.as_deref());
    match make_persister(environment, shell) {
        Ok(persister) => persister.inspect_path(shim_dir),
        Err(error) => env::EnvironmentObservation::unavailable(error.to_string()),
    }
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
pub(super) fn confirm_fix(
    ctx: &output::Context,
    yes: bool,
    target: Option<&str>,
) -> Result<bool, VvmError> {
    if yes {
        return Ok(true);
    }
    if ctx.is_unattended() || !std::io::stdin().is_terminal() {
        return Err(VvmError::NoTty { detail: "durable environment repair requires explicit --yes or interactive consent; unattended mode alone is not consent".into() });
    }
    let prompt = format!(
        "Write shims and update PATH at {}?",
        target.unwrap_or("the unavailable durable target")
    );
    Ok(crate::output::prompts::confirm(&prompt, false).unwrap_or(false))
}
