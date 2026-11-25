//! Explicit project registration is an operator-owned post-materialization act.

use std::path::Path;

use anyhow::{Result, ensure};
use vibe_orchestrator::ports::AfterDurableWorld;

use crate::output;

pub(super) fn validate_syntax(packages: &[String]) -> Result<()> {
    for package in packages {
        let reference = vibe_core::PackageRef::parse(package)?;
        ensure!(
            reference.is_qualified() && !package.contains([':', '@']),
            "--register-mcp requires an unversioned GROUP/NAME"
        );
    }
    Ok(())
}

pub(super) struct ProjectMcpWorld<'a, T> {
    ordinary: T,
    ctx: &'a output::Context,
    packages: &'a [String],
}

impl<'a, T> ProjectMcpWorld<'a, T> {
    pub(super) fn new(ordinary: T, ctx: &'a output::Context, packages: &'a [String]) -> Self {
        Self {
            ordinary,
            ctx,
            packages,
        }
    }
}

impl<T: AfterDurableWorld> AfterDurableWorld for ProjectMcpWorld<'_, T> {
    fn after(
        &mut self,
        root: &Path,
        run: vibe_orchestrator::InstallRunContext,
        workspace: &vibe_workspace::Workspace,
    ) -> Result<vibe_orchestrator::WorldCallbackOutcome> {
        let mut outcome = self.ordinary.after(root, run, workspace)?;
        if self.packages.is_empty() {
            return Ok(outcome);
        }
        let operation = self
            .ctx
            .progress()
            .task("Registering requested project MCP packages");
        match crate::commands::mcp::register_project_packages(self.ctx, root, self.packages) {
            Ok(()) => {
                operation.finish();
                let mut seen = std::collections::BTreeSet::new();
                for package in self.packages {
                    if !seen.insert(package) {
                        continue;
                    }
                    outcome.contributions.push(vibe_wire::generated::lifecycle_report::LifecycleContributionReport {
                        key: format!("project-mcp-registration:{package}"),
                        phase: "install".into(),
                        point: "project:registration".into(),
                        handler: "addressed-mcp".into(),
                        provider: package.clone(),
                        status: "ok".into(),
                        tier: "operator".into(),
                        message: Some("explicit project-scoped MCP registration completed".into()),
                        reference: Some("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#PROJECT-INSTALL-REGISTRATION-INTENT".into()),
                        flagged: None, slot_target: None, stderr: None,
                        stderr_truncated: None, stdout: None, stdout_truncated: None, version: None,
                    });
                }
            }
            Err(error) => {
                operation.fail("project MCP registration failed");
                return Err(error);
            }
        }
        Ok(outcome)
    }
}
