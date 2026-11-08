//! What a finished run hands back to the operation that drove it
//! (`PROP-065#execution`): the per-step outcomes the index keeps and the
//! environment entries the `env` steps applied.
//!
//! Projections over the recorded state, not part of running anything —
//! which is why they sit beside the runner rather than inside it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#execution");

use crate::index::{EnvEntry, StepRecord};

use super::Run;

impl Run<'_> {
    /// Inherited legacy prefixes cannot turn a current default into a historical receipt.
    pub fn secret_origins(
        &self,
    ) -> std::collections::BTreeMap<
        String,
        vibe_wire::generated::shared::ConfigurationParameterOrigin,
    > {
        use vibe_wire::generated::shared as w;
        let mut origins = self.plan.secret_origins.clone();
        if self.state.operation.as_ref().is_some_and(|o| {
            o.provenance == w::ApplicationPendingOperationProvenance::LegacyEffectiveOnly
        }) {
            for value in origins.values_mut() {
                if !matches!(value, w::ConfigurationParameterOrigin::Explicit(_)) {
                    *value = w::ConfigurationParameterOrigin::LegacyEffective(Box::new(
                        w::ConfigurationParameterOriginLegacyEffective {},
                    ));
                }
            }
        }
        origins
    }

    /// The step outcomes the index keeps.
    pub fn records(&self) -> Vec<StepRecord> {
        self.plan
            .steps
            .iter()
            .filter_map(|planned| Some((planned, self.state.get(&planned.step.id)?)))
            .map(|(planned, state)| StepRecord {
                id: state.id.clone(),
                kind: state.kind.clone(),
                phase: planned.step.phase,
                outcome: state.outcome,
                uninstall: state.uninstall.clone(),
            })
            .collect()
    }

    /// The environment entries the env steps applied, in step order
    /// (`PROP-065#env-step`).
    pub fn env_entries(&self) -> Vec<EnvEntry> {
        self.plan
            .steps
            .iter()
            .filter_map(|planned| self.state.get(&planned.step.id))
            .flat_map(|state| state.env.iter().cloned())
            .collect()
    }
}

use super::StepKind;
/// The task label of one step (`PROP-065#display`): its id, its kind and
/// its title, the same row a step panel and the event stream carry. A
/// renderer collapses the column gap along with every other run of
/// whitespace it sanitises away.
pub(crate) fn step_label(step: &vibe_core::manifest::StepDecl) -> String {
    format!("{} ({})  {}", step.id, step.kind.name(), step.title)
}

/// The step kinds that run a child of their own, and therefore write the
/// output log a failure's tail comes from (`PROP-065#display`).
pub(super) fn writes_log(kind: &StepKind) -> bool {
    matches!(
        kind,
        StepKind::Recipe { .. }
            | StepKind::Shell { .. }
            | StepKind::Native { .. }
            | StepKind::Agent { .. }
    )
}
