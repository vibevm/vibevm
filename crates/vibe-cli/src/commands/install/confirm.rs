//! The install-consent port and the CLI dialoguer adapter.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#INSTALL-DESTINATION");

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use std::path::{Path, PathBuf};
use vibe_orchestrator::ports::ConfirmGate;

use crate::exit_code::InstallError;
use crate::output;

/// CLI confirmation over the invocation's already-resolved posture.
pub(crate) struct CliConfirmGate<'a> {
    ctx: &'a output::Context,
    assume_yes: bool,
    destination: PathBuf,
}

impl<'a> CliConfirmGate<'a> {
    pub(crate) fn new(ctx: &'a output::Context, assume_yes: bool, root: &Path) -> Self {
        Self {
            ctx,
            assume_yes,
            destination: crate::commands::init::strip_unc_public(
                root.join(vibe_core::layout::current_vibedeps_root()),
            ),
        }
    }

    fn project_prompt(&self, packages: usize) -> String {
        format!(
            "Install {packages} package{} into {}?",
            if packages == 1 { "" } else { "s" },
            self.destination.display(),
        )
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
        let approved = self
            .ctx
            .suspend_progress(|| {
                Confirm::new()
                    .with_prompt(self.project_prompt(packages))
                    .default(false)
                    .interact()
            })
            .context("reading user confirmation")?;
        if approved {
            Ok(())
        } else {
            Err(InstallError::UserDeclined.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::AgentModeArg;

    fn context(quiet: bool, json: bool, unattended: bool) -> output::Context {
        output::Context::from_flags(quiet, json, None, unattended, AgentModeArg::Cli)
    }

    #[test]
    fn prompt_names_prepared_canonical_destination_and_package_count() {
        let project = tempfile::tempdir().unwrap();
        let root = project.path().canonicalize().unwrap();
        let ctx = context(false, false, false);
        let gate = CliConfirmGate::new(&ctx, false, &root);
        let destination = crate::commands::init::strip_unc_public(
            root.join(vibe_core::layout::current_vibedeps_root()),
        );
        assert!(destination.is_absolute());
        assert_eq!(
            gate.project_prompt(1),
            format!("Install 1 package into {}?", destination.display())
        );
        assert_eq!(
            gate.project_prompt(2),
            format!("Install 2 packages into {}?", destination.display())
        );
    }

    #[test]
    fn existing_explicit_bypasses_allow_confirmation_without_tty() {
        let project = tempfile::tempdir().unwrap();
        for (assume_yes, json, unattended) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            let ctx = context(false, json, unattended);
            CliConfirmGate::new(&ctx, assume_yes, project.path())
                .confirm_install(1)
                .unwrap();
        }
    }

    #[test]
    fn no_tty_refuses_even_when_quiet_or_destination_looks_like_a_service() {
        // This focused gate runs with redirected streams; do not accept an
        // unattended environment as evidence of the refusal path.
        assert!(!console::user_attended(), "run this gate without a TTY");
        let project = tempfile::tempdir().unwrap();
        let root = project.path().join("services").join("mcp");
        for quiet in [false, true] {
            let ctx = context(quiet, false, false);
            assert!(!ctx.is_unattended(), "run with VIBE_UNATTENDED unset");
            let err = CliConfirmGate::new(&ctx, false, &root)
                .confirm_install(1)
                .unwrap_err();
            assert!(
                err.to_string()
                    .contains("no TTY available for confirmation")
            );
            assert!(err.to_string().contains("--assume-yes"));
        }
    }
}
