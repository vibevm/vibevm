//! Immutable preparation vocabulary. There is deliberately no provider or writer here.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#PREVIEW-PREPARATION");
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationPlan {
    pub context: String,
    pub capture: String,
    pub cache: String,
    pub stages: Vec<PreparationStage>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationStage {
    pub kind: StageKind,
    pub input: String,
    pub effects: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageKind {
    Fetch,
    Hydration,
    Native,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageEvidence {
    pub files: BTreeMap<String, Vec<u8>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum StageOutcome {
    Captured { evidence: StageEvidence },
    Failed { reason: String },
    Unavailable { reason: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationOutcome {
    pub plan: PreparationPlan,
    /// Ordered prefix: a failed/unavailable stage ends execution.
    pub stages: Vec<StageOutcome>,
    pub terminal: bool,
}
impl PreparationOutcome {
    pub fn complete(&self) -> bool {
        self.terminal
            && self.stages.len() == self.plan.stages.len()
            && self
                .stages
                .iter()
                .all(|s| matches!(s, StageOutcome::Captured { .. }))
    }
    pub fn validate(&self, plan: &PreparationPlan) -> Result<(), String> {
        if &self.plan != plan || self.stages.len() > plan.stages.len() {
            return Err("foreign preparation evidence".into());
        }
        if self
            .stages
            .iter()
            .take(self.stages.len().saturating_sub(1))
            .any(|s| !matches!(s, StageOutcome::Captured { .. }))
        {
            return Err("evidence after failed stage".into());
        }
        if self.terminal
            && self.stages.len() < plan.stages.len()
            && !matches!(
                self.stages.last(),
                Some(StageOutcome::Failed { .. } | StageOutcome::Unavailable { .. })
            )
        {
            return Err("terminal preparation lacks stage outcome".into());
        }
        Ok(())
    }
}
