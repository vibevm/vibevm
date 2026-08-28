//! The post-durability stage `vibe install` runs, and its failure family.
//!
//! The STAGE itself — world planning, plan surfacing, phase dispatch — is the
//! shared application service. What stays here is the one thing a library may
//! not decide: which registered report family this command's failure belongs
//! to. `vibe install` reports a slot failure in a `cli-install-report`, and the
//! failure of THIS stage in a `cli-lifecycle-report` with no install root at
//! all — characterised behaviour a hosting agent parses.
//!
//! It is a NAMED port implementation, not a closure. The closure it replaced
//! captured the whole `output::Context`, which is exactly the surface state the
//! typed-input law exists to keep out of the lower call.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use vibe_lifecycle::AgentBackend;
use vibe_orchestrator::ports::{AfterDurableWorld, RunObserver};

use crate::commands::compile_trace;
use crate::commands::install::{InstallRunContext, WorldCallbackOutcome};

/// `vibe install`'s own post-durability stage.
///
/// It holds exactly what the stage needs — the OUTER observation policy and the
/// command's ONE agent backend — and the root its failure family is named
/// against. No rendering context, no arguments, no config.
pub(crate) struct DirectInstallWorld<'a> {
    observer: &'a dyn RunObserver,
    agent: &'a Arc<dyn AgentBackend>,
}

impl<'a> DirectInstallWorld<'a> {
    pub(crate) const fn new(
        observer: &'a dyn RunObserver,
        agent: &'a Arc<dyn AgentBackend>,
    ) -> Self {
        Self { observer, agent }
    }
}

impl AfterDurableWorld for DirectInstallWorld<'_> {
    fn after(
        &mut self,
        path: &Path,
        run: InstallRunContext,
        workspace: &vibe_workspace::Workspace,
    ) -> Result<WorldCallbackOutcome> {
        vibe_orchestrator::after_durable_world_stage(
            self.observer,
            path,
            run,
            workspace,
            self.agent,
        )
        .map_err(|error| match vibe_orchestrator::failure::take(error) {
            // The measurement travels neutral; the family is chosen HERE, and
            // the emission bit the failing site froze crosses unchanged.
            Ok(failure) => compile_trace::carry(
                super::registered_family(path, failure.evidence),
                failure.original,
                failure.emit_machine_failure,
            ),
            Err(error) => error,
        })
    }
}
