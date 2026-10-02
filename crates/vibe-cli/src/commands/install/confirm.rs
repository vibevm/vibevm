//! The install-consent port and the CLI dialoguer adapter.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#INSTALL-DESTINATION");

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use std::path::{Path, PathBuf};
use vibe_orchestrator::ports::ConfirmGate;

use crate::exit_code::InstallError;
use crate::output;

/// Only the global MCP wrapper can select automatic user-project installation.
#[derive(Clone, Copy)]
pub(crate) enum ConfirmationScope<'a> {
    Project,
    UserProject {
        notice: &'a output::Context,
        action: UserProjectAction,
    },
}

#[derive(Clone, Copy)]
pub(crate) enum UserProjectAction {
    Install,
    Update,
}

/// CLI confirmation over the invocation's already-resolved posture.
pub(crate) struct CliConfirmGate<'a> {
    ctx: &'a output::Context,
    assume_yes: bool,
    destination: PathBuf,
    scope: ConfirmationScope<'a>,
}

impl<'a> CliConfirmGate<'a> {
    pub(crate) fn new(ctx: &'a output::Context, assume_yes: bool, root: &Path) -> Self {
        Self::with_scope(ctx, assume_yes, root, ConfirmationScope::Project)
    }

    pub(crate) fn with_scope(
        ctx: &'a output::Context,
        assume_yes: bool,
        root: &Path,
        scope: ConfirmationScope<'a>,
    ) -> Self {
        Self {
            ctx,
            assume_yes,
            destination: crate::commands::init::strip_unc_public(
                root.join(vibe_core::layout::current_vibedeps_root()),
            ),
            scope,
        }
    }

    fn project_prompt(&self, packages: usize) -> String {
        format!(
            "Install {packages} package{} into {}?",
            if packages == 1 { "" } else { "s" },
            self.destination.display(),
        )
    }

    fn user_notice(&self, packages: usize, action: UserProjectAction) -> String {
        let verb = match action {
            UserProjectAction::Install => "Installing",
            UserProjectAction::Update => "Updating",
        };
        format!(
            "{verb} {packages} package{} for the current user into {}",
            if packages == 1 { "" } else { "s" },
            self.destination.display(),
        )
    }

    fn requires_confirmation(&self) -> bool {
        matches!(self.scope, ConfirmationScope::Project)
            && !self.assume_yes
            && !self.ctx.is_unattended()
            && !self.ctx.is_json()
    }
}

impl ConfirmGate for CliConfirmGate<'_> {
    fn confirm_install(&self, packages: usize) -> Result<()> {
        if let ConfirmationScope::UserProject { notice, action } = self.scope {
            notice.step(&self.user_notice(packages, action));
            return Ok(());
        }
        if !self.requires_confirmation() {
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

#[cfg(test)]
mod service_tests {
    use super::*;
    use crate::cli::AgentModeArg;
    use rust_ai_native_env_audit::EnvGuard;

    #[test]
    #[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-060#INSTALL-DESTINATION")]
    fn canonical_selected_destination_is_named_for_projects_and_custom_user_settings() {
        let mut env = EnvGuard::lock();
        env.unset("VIBE_UNATTENDED");
        let scratch = tempfile::tempdir().unwrap();
        let settings = scratch.path().join("custom settings");
        env.set("VIBE_SETTINGS", settings.to_str().unwrap());
        let root = super::super::user_project_root().unwrap();
        std::fs::create_dir_all(&root).unwrap();
        let canonical = root.canonicalize().unwrap();
        let ctx = output::Context::from_flags(false, false, None, false, AgentModeArg::Cli);
        let gate = CliConfirmGate::new(&ctx, false, &canonical);
        let destination = crate::commands::init::strip_unc_public(
            canonical.join(vibe_core::layout::current_vibedeps_root()),
        )
        .display()
        .to_string();
        assert_eq!(
            gate.project_prompt(1),
            format!("Install 1 package into {destination}?")
        );
        assert_eq!(
            gate.project_prompt(3),
            format!("Install 3 packages into {destination}?")
        );
        assert_eq!(
            gate.user_notice(1, UserProjectAction::Install),
            format!("Installing 1 package for the current user into {destination}")
        );
        assert_eq!(
            gate.user_notice(3, UserProjectAction::Update),
            format!("Updating 3 packages for the current user into {destination}")
        );
        assert!(!destination.starts_with(r"\\?\"));
        assert!(
            gate.requires_confirmation(),
            "a path resembling the service project grants no exemption"
        );
    }

    #[test]
    #[specmark::verifies(
        "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#REG-USER-PROJECT-MATERIALISATION"
    )]
    fn typed_user_scope_is_automatic_without_widening_invocation_posture() {
        let mut env = EnvGuard::lock();
        env.unset("VIBE_UNATTENDED");
        let root = tempfile::tempdir().unwrap();
        for (quiet, json, unattended) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            let ctx = output::Context::from_flags(quiet, json, None, unattended, AgentModeArg::Cli);
            let child = ctx.progress_child();
            for action in [UserProjectAction::Install, UserProjectAction::Update] {
                let gate = CliConfirmGate::with_scope(
                    &child,
                    false,
                    root.path(),
                    ConfirmationScope::UserProject {
                        notice: &ctx,
                        action,
                    },
                );
                assert!(!gate.requires_confirmation());
                assert!(!gate.assume_yes);
                gate.confirm_install(2).unwrap();
            }
            assert_eq!(
                CliConfirmGate::new(&ctx, false, root.path()).requires_confirmation(),
                !json && !unattended
            );
            assert!(!CliConfirmGate::new(&ctx, true, root.path()).requires_confirmation());
        }
    }
}
