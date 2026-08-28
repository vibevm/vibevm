//! The install-consent port and the CLI dialoguer adapter.

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use vibe_orchestrator::ports::ConfirmGate;

use crate::exit_code::InstallError;
use crate::output;

/// CLI confirmation over the invocation's already-resolved posture.
pub(crate) struct CliConfirmGate<'a> {
    ctx: &'a output::Context,
    assume_yes: bool,
}

impl<'a> CliConfirmGate<'a> {
    pub(crate) const fn new(ctx: &'a output::Context, assume_yes: bool) -> Self {
        Self { ctx, assume_yes }
    }
}

impl ConfirmGate for CliConfirmGate<'_> {
    fn confirm_install(&self, packages: usize) -> Result<()> {
        if self.assume_yes || self.ctx.is_unattended() || self.ctx.is_json() {
            return Ok(());
        }
        if !console::user_attended() {
            bail!(
                "no TTY available for confirmation; re-run with `--assume-yes` to apply this plan non-interactively"
            );
        }
        let approved = Confirm::new()
            .with_prompt(format!(
                "Materialise {packages} package{} into vibedeps/ and regenerate boot artifacts?",
                if packages == 1 { "" } else { "s" },
            ))
            .default(false)
            .interact()
            .context("reading user confirmation")?;
        if approved {
            Ok(())
        } else {
            Err(InstallError::UserDeclined.into())
        }
    }
}
