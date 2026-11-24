//! The orchestrator's typed plan events, rendered in the CLI's voice.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use vibe_install::{PlanEvent, PlanObserver};

use crate::output;

/// Renders the orchestrator's typed plan events in the CLI's voice.
pub(super) struct CtxObserver<'a> {
    ctx: &'a output::Context,
    progress: vibe_core::progress::Progress,
}

impl<'a> CtxObserver<'a> {
    pub(super) fn new(ctx: &'a output::Context) -> Self {
        Self {
            ctx,
            progress: vibe_core::progress::Progress::default(),
        }
    }

    pub(super) fn with_progress(mut self, progress: vibe_core::progress::Progress) -> Self {
        self.progress = progress;
        self
    }

    pub(super) fn progress_handle(&self) -> vibe_core::progress::Progress {
        self.progress.clone()
    }
}

impl PlanObserver for CtxObserver<'_> {
    fn on(&self, event: PlanEvent) {
        let ctx = self.ctx;
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
            PlanEvent::RecommendationSkipped {
                package,
                by,
                reason,
            } => ctx.diagnostic(&recommendation_skipped(&package, &by, &reason)),
            PlanEvent::FeaturesUnmatched { features } => ctx.diagnostic(&format!(
                "requested feature{} {} not declared on any root package — silently ignored",
                if features.len() == 1 { "" } else { "s" },
                features
                    .iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }

    fn progress(&self) -> vibe_core::progress::Progress {
        self.progress_handle()
    }
}

/// The warning for a recommendation the install did not follow (PROP-077
/// `#PROJECT-RECOMMENDS`): the package first, then who recommended it and why.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-077#PROJECT-RECOMMENDS")]
fn recommendation_skipped(package: &str, by: &str, reason: &str) -> String {
    format!(
        "{} not installed · recommended by {} · {}",
        package.replace('`', ""),
        by.replace('`', ""),
        reason.replace('`', "")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommendation_skip_names_the_package_first() {
        assert_eq!(
            recommendation_skipped(
                "org.vibevm.fixture/companion",
                "org.vibevm.fixture/recommender",
                "declined by --no-recommends"
            ),
            "org.vibevm.fixture/companion not installed · recommended by \
             org.vibevm.fixture/recommender · declined by --no-recommends"
        );
        assert_eq!(
            recommendation_skipped("`a`", "`b`", "`c`"),
            "a not installed · recommended by b · c"
        );
    }
}
