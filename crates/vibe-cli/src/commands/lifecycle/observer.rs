//! The OUTER phase observation policy, and the CLI rendering behind it.
//!
//! Deliberately NOT the install observer. This one's stream formula nulls on
//! `--quiet`; the child install formula nulls on a SUPPRESSED context, and its
//! machine-failure emission bit is the child's answer. The two are separate
//! objects built from separate contexts, and merging them silently flips the
//! emission bit of a composed run's slot failure.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use anyhow::Result;
use vibe_lifecycle::RunMetadata;
use vibe_lifecycle::process::StreamMode;
use vibe_orchestrator::RitualPlan;
use vibe_orchestrator::ports::RunObserver;
use vibe_wire::generated::lifecycle_report::{
    LifecycleContributionReport, LifecycleReport, LifecycleStepReport,
};

use crate::output;

/// The existing CLI rendering and stream policy, behind [`RunObserver`].
pub(crate) struct CliRunObserver<'a> {
    ctx: &'a output::Context,
}

impl<'a> CliRunObserver<'a> {
    pub(crate) const fn new(ctx: &'a output::Context) -> Self {
        Self { ctx }
    }
}

impl RunObserver for CliRunObserver<'_> {
    fn stream_mode(&self) -> StreamMode {
        if self.ctx.is_json() {
            StreamMode::Capture
        } else if self.ctx.is_quiet() {
            StreamMode::Null
        } else {
            StreamMode::Inherit
        }
    }

    fn binary_quiet(&self) -> bool {
        self.ctx.is_json() || self.ctx.is_quiet()
    }

    fn emit_machine_failure(&self) -> bool {
        self.ctx.is_json() && !self.ctx.suppresses_output()
    }

    fn observe_plan(
        &self,
        plan: &RitualPlan,
        metadata: &RunMetadata,
        emit_empty: bool,
    ) -> Result<()> {
        super::plan::surface_cli_plan(self.ctx, plan, metadata, emit_empty)
    }

    fn observe_contribution(&self, report: &LifecycleContributionReport) {
        render_cli_outcome(self.ctx, report);
    }

    /// The UNTRACKED clean epoch's failure document, unchanged.
    ///
    /// The clean lifecycle keeps no state record and its wipe destroys the tree
    /// a trace would live in, so it never opens a session and has no outer
    /// funnel to hand a draft to. This is the one command that deliberately has
    /// no boundary, so it emits its own document here.
    fn observe_untracked_failure(
        &self,
        metadata: &RunMetadata,
        phase: &str,
        contributions: &[LifecycleContributionReport],
    ) -> Result<()> {
        if !self.ctx.is_json() {
            return Ok(());
        }
        // A failing run still shows the plan it was executing: the deferral only
        // ever holds documents back until the outcome is known, and a failure is
        // an outcome.
        self.ctx.flush_json_plans()?;
        self.ctx.emit_json(&LifecycleReport {
            chain: metadata.chain.clone(),
            command: "lifecycle".into(),
            contributions: contributions.to_vec(),
            notices: Vec::new(),
            ok: false,
            requested: metadata.requested.clone(),
            steps: vec![LifecycleStepReport {
                phase: phase.into(),
                status: "fail".into(),
            }],
            delegation: None,
            trace: None,
            // A fatal-outcome report is written before verify could
            // reconcile anything, so it carries no evidence member.
            verification: None,
        })
    }
}

pub(crate) fn render_cli_outcome(ctx: &output::Context, report: &LifecycleContributionReport) {
    if ctx.is_json() || ctx.is_quiet() {
        return;
    }
    if report.status == "fresh" {
        ctx.step(&format!(
            "fresh `{}` — provider={}",
            report.key, report.provider
        ));
    } else if let Some(message) = &report.message {
        if report.key.starts_with("@vibe/package/skill/") {
            ctx.step(&format!("package binding [{}]: {message}", report.provider));
        } else {
            ctx.step(&format!("log [{}]: {message}", report.provider));
        }
    }
}

#[cfg(test)]
mod tests {
    use vibe_lifecycle::process::StreamMode;
    use vibe_orchestrator::ports::RunObserver;

    use super::CliRunObserver;

    fn context(quiet: bool, json: bool) -> crate::output::Context {
        crate::output::Context::from_flags(quiet, json, None, false, crate::cli::AgentModeArg::Cli)
    }

    fn install_context(
        quiet: bool,
        json: bool,
    ) -> (crate::output::Context, crate::output::Context) {
        let outer = context(quiet, json);
        let child = outer.quiet_child();
        (outer, child)
    }

    #[test]
    fn stream_and_binary_policy_is_the_existing_cli_matrix() {
        let human_ctx = context(false, false);
        let human = CliRunObserver::new(&human_ctx);
        assert_eq!(human.stream_mode(), StreamMode::Inherit);
        assert!(!human.binary_quiet());
        assert!(!human.emit_machine_failure());

        let quiet_ctx = context(true, false);
        let quiet = CliRunObserver::new(&quiet_ctx);
        assert_eq!(quiet.stream_mode(), StreamMode::Null);
        assert!(quiet.binary_quiet());
        assert!(!quiet.emit_machine_failure());

        let json_ctx = context(false, true);
        let json = CliRunObserver::new(&json_ctx);
        assert_eq!(json.stream_mode(), StreamMode::Capture);
        assert!(json.binary_quiet());
        assert!(json.emit_machine_failure());
    }

    /// The two policies are NOT the same function, and this is the pair that
    /// proves it. Under a composed `--json` phase verb the OUTER observer emits
    /// its machine failure and the CHILD install observer does not; merging
    /// them would make a prerequisite install's slot failure narrate a second
    /// document that no golden has ever contained.
    #[test]
    fn the_child_install_policy_is_not_the_outer_phase_policy() {
        use vibe_orchestrator::ports::InstallObserver;

        let (outer, child) = install_context(false, true);
        let phase = CliRunObserver::new(&outer);
        let install = crate::commands::install::CliInstallObserver::new(&child, Some(&outer));

        assert!(
            phase.emit_machine_failure(),
            "the outer json context owns its document",
        );
        assert!(
            !install.emit_machine_failure(),
            "the suppressed child does not emit a second one",
        );

        // …and the stream formulas differ on their own axis. `--quiet` is not
        // the same predicate as "suppressed": a direct `vibe install --quiet`
        // still INHERITS its handler streams, while a phase verb's own
        // contributions are nulled. Merging the formulas would silence a
        // direct install's handler output, which no golden has ever shown.
        let quiet = context(true, false);
        assert_eq!(
            CliRunObserver::new(&quiet).stream_mode(),
            StreamMode::Null,
            "the PHASE formula nulls on --quiet",
        );
        assert_eq!(
            crate::commands::install::CliInstallObserver::new(&quiet, None).stream_mode(),
            StreamMode::Inherit,
            "the CHILD formula nulls only on a SUPPRESSED context",
        );
        // A phase verb's child context IS suppressed, and there the install
        // streams do null — same function, different input.
        assert_eq!(
            crate::commands::install::CliInstallObserver::new(&quiet.quiet_child(), None)
                .stream_mode(),
            StreamMode::Null,
        );
    }
}
