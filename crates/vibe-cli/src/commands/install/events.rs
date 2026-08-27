//! The orchestrator's typed plan events, rendered in the CLI's voice.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use vibe_install::{PlanEvent, PlanObserver};

use crate::output;

/// Renders the orchestrator's typed plan events in the CLI's voice.
pub(super) struct CtxObserver<'a>(pub(super) &'a output::Context);

impl PlanObserver for CtxObserver<'_> {
    fn on(&self, event: PlanEvent) {
        let ctx = self.0;
        match event {
            PlanEvent::MigratingRequires { entries } => ctx.step(&format!(
                "Migrating [requires] from `vibe.lock` meta.root_dependencies ({} entr{})",
                entries,
                if entries == 1 { "y" } else { "ies" },
            )),
            PlanEvent::Reresolving { reason } => ctx.step(&format!("re-resolving — {reason}")),
            PlanEvent::HeldPinsConflicted { error } => ctx.step(&format!(
                "held pins conflicted with the change ({error}); re-resolving freely"
            )),
            PlanEvent::ResolvingRoots { roots } => ctx.heading(&format!(
                "Resolving {} root package{}…",
                roots,
                if roots == 1 { "" } else { "s" },
            )),
            PlanEvent::GraphSolved { roots, total } => ctx.step(&format!(
                "{} root, {} transitive — {} package{} total",
                roots,
                total - roots,
                total,
                if total == 1 { "" } else { "s" },
            )),
            PlanEvent::ConditionalIteration { iteration, extras } => ctx.step(&format!(
                "Conditional dependencies (iter {}): {} extra root{}",
                iteration,
                extras,
                if extras == 1 { "" } else { "s" },
            )),
            PlanEvent::FeaturesUnmatched { features } => ctx.step(&format!(
                "warning: requested feature{} {} not declared on any root package — silently ignored",
                if features.len() == 1 { "" } else { "s" },
                features
                    .iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }
}
