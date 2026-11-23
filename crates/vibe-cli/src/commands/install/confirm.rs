//! The install-consent port and the CLI dialoguer adapter.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#INSTALL-DESTINATION");

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use vibe_orchestrator::ports::ConfirmGate;

use crate::exit_code::InstallError;
use crate::output;

/// Only the global MCP wrapper can select automatic user-project installation.
#[derive(Clone, Copy)]
pub(crate) enum ConfirmationScope<'a> {
    Project,
    /// Selected only by the dispatcher that actually materializes project apps.
    ProjectApplications,
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
    ordinary_explicit: bool,
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
            ordinary_explicit: false,
        }
    }
    pub(crate) fn with_ordinary_explicit(mut self, enabled: bool) -> Self {
        self.ordinary_explicit = enabled;
        self
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
        matches!(
            self.scope,
            ConfirmationScope::Project | ConfirmationScope::ProjectApplications
        ) && !self.assume_yes
            && !self.ctx.is_unattended()
            && !self.ctx.is_json()
    }
}

impl ConfirmGate for CliConfirmGate<'_> {
    fn admit_selected_packages(
        &self,
        root: &Path,
        packages: &[vibe_core::manifest::LockedPackage],
        roots: &[vibe_core::PackageRef],
        selected: &[vibe_workspace::install::ResolvedDep],
    ) -> Result<()> {
        if matches!(self.scope, ConfirmationScope::ProjectApplications) {
            crate::commands::app::project::admit_selected(root, packages, roots, selected)?;
        }
        Ok(())
    }

    fn slot_judgment(&self) -> Result<vibe_install::SlotJudgmentContext> {
        super::trust::judgment(self.ctx, self.assume_yes)
    }

    fn confirm_install(&self, packages: usize) -> Result<()> {
        if let ConfirmationScope::UserProject { notice, action } = self.scope {
            notice.step(&self.user_notice(packages, action));
            return Ok(());
        }
        if self.ordinary_explicit && !self.assume_yes {
            if self.ctx.is_json() || self.ctx.is_unattended() || !console::user_attended() {
                bail!(
                    "ordinary submitted install requires explicit --assume-yes or attended apply confirmation"
                );
            }
            return match self.ask_apply(packages) {
                Ok(true) => Ok(()),
                // Escape or Ctrl-C on the ordinary path declines, as it did.
                Ok(false) => Err(InstallError::UserDeclined.into()),
                Err(dialoguer::Error::IO(error))
                    if error.kind() == std::io::ErrorKind::Interrupted =>
                {
                    Err(InstallError::UserDeclined.into())
                }
                Err(error) => Err(anyhow::Error::new(error).context("reading user confirmation")),
            };
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
            .ask_apply(packages)
            .context("reading user confirmation")?;
        if approved {
            Ok(())
        } else {
            Err(InstallError::UserDeclined.into())
        }
    }
}

impl CliConfirmGate<'_> {
    /// The one install question of both attended paths: default **yes**,
    /// asked with the progress panel's key reader stopped so the keys
    /// reach the question.
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
    fn ask_apply(&self, packages: usize) -> dialoguer::Result<bool> {
        self.ask_apply_with(packages, |prompt| {
            crate::output::prompts::confirm(prompt, true)
        })
    }

    /// [`Self::ask_apply`] over an injected question, the seam its test uses.
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
    fn ask_apply_with(
        &self,
        packages: usize,
        ask: impl FnOnce(&str) -> dialoguer::Result<bool>,
    ) -> dialoguer::Result<bool> {
        let prompt = self.project_prompt(packages);
        self.ctx.with_terminal_released(|| ask(&prompt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::AgentModeArg;
    use rust_ai_native_env_audit::EnvGuard;

    #[test]
    fn personal_project_scope_does_not_enable_application_admission() {
        let root = tempfile::tempdir().unwrap();
        let ctx = output::Context::from_flags(false, true, None, true, AgentModeArg::Cli);
        // Project scope alone is also used by personal installation. If this
        // accidentally dispatches app policy, the absent selected world fails.
        let gate = CliConfirmGate::new(&ctx, true, root.path());
        let row: vibe_core::manifest::LockedPackage = toml::from_str("kind='app'\nname='no-options'\ngroup='org.example'\nversion='1.0.0'\nsource_url='file:///synthetic'\ncontent_hash='sha256:abc'\n").unwrap();
        gate.admit_selected_packages(root.path(), std::slice::from_ref(&row), &[], &[])
            .unwrap();
        let active = CliConfirmGate::with_scope(
            &ctx,
            true,
            root.path(),
            ConfirmationScope::ProjectApplications,
        );
        assert!(
            active
                .admit_selected_packages(root.path(), &[row], &[], &[])
                .is_err()
        );
        assert!(!root.path().join("vibevm/vibeopt").exists());
    }

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

    #[test]
    #[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-067#question")]
    fn install_question_releases_the_progress_panel() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        #[derive(Default)]
        struct Panel {
            released: AtomicUsize,
            reclaimed: AtomicUsize,
        }
        impl output::TerminalPanel for Panel {
            fn release(&self) {
                self.released.fetch_add(1, Ordering::SeqCst);
            }
            fn reclaim(&self) {
                self.reclaimed.fetch_add(1, Ordering::SeqCst);
            }
        }
        let root = tempfile::tempdir().unwrap();
        let ctx = output::Context::from_flags(false, false, None, false, AgentModeArg::Cli);
        let panel = Arc::new(Panel::default());
        ctx.register_terminal_panel(panel.clone());
        let gate = CliConfirmGate::new(&ctx, false, root.path());
        let asked = gate
            .ask_apply_with(2, |prompt| {
                assert_eq!(prompt, gate.project_prompt(2));
                assert_eq!(panel.released.load(Ordering::SeqCst), 1);
                assert_eq!(panel.reclaimed.load(Ordering::SeqCst), 0);
                Ok(true)
            })
            .unwrap();
        assert!(asked);
        assert_eq!(panel.released.load(Ordering::SeqCst), 1);
        assert_eq!(panel.reclaimed.load(Ordering::SeqCst), 1);
    }
}
