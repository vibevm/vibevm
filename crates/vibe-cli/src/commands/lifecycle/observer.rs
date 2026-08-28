//! Surface port for one lifecycle run, plus the CLI adapter.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use anyhow::Result;
use vibe_lifecycle::RunMetadata;
use vibe_lifecycle::process::StreamMode;
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;

use crate::output;

use super::world;

/// The observation policy supplied by a lifecycle surface.
///
/// Execution asks only for stream policy and typed plan/row observations. The
/// CLI adapter renders them; a hosted adapter can capture streams and retain
/// the typed command result without importing terminal behavior.
pub(crate) trait RunObserver: Send + Sync {
    fn stream_mode(&self) -> StreamMode;

    fn binary_quiet(&self) -> bool;

    /// Whether a carried executed failure owns an immediate machine document.
    fn emit_machine_failure(&self) -> bool;

    fn observe_plan(
        &self,
        plan: &world::RitualPlan,
        metadata: &RunMetadata,
        emit_empty: bool,
    ) -> Result<()>;

    fn observe_contribution(&self, report: &LifecycleContributionReport);
}

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
        plan: &world::RitualPlan,
        metadata: &RunMetadata,
        emit_empty: bool,
    ) -> Result<()> {
        super::plan::surface_cli_plan(self.ctx, plan, metadata, emit_empty)
    }

    fn observe_contribution(&self, report: &LifecycleContributionReport) {
        super::dispatch::render_cli_outcome(self.ctx, report);
    }
}

#[cfg(test)]
mod tests {
    use vibe_lifecycle::process::StreamMode;

    use super::{CliRunObserver, RunObserver};

    fn context(quiet: bool, json: bool) -> crate::output::Context {
        crate::output::Context::from_flags(quiet, json, None, false, crate::cli::AgentModeArg::Cli)
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
}
