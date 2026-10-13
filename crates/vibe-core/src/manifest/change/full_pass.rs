//! Immutable selected composition; no effect authority or runtime carriage.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-FULL-PASS-PLAN");
use super::{
    ChangeError, ChangeErrorKind, ChangeManifest, IntentIdentity, NormativeInput, WorkGraph,
    WorkIdentity, invalid,
};
use sha2::{Digest, Sha256};

pub const PRODUCTION_REQUEST_RECIPE: &str = "sha256:vibe-change-production-request-v1";
/// Recipe-qualified host observation, never authentication or an effect grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionObservation {
    recipe: String,
    value: String,
}
impl ProductionObservation {
    pub fn new(recipe: &str, value: &str) -> Result<Self, ChangeError> {
        if [recipe, value]
            .iter()
            .any(|s| s.trim().is_empty() || s.len() > 4096 || s.chars().any(char::is_control))
        {
            return Err(invalid(
                ChangeErrorKind::Input,
                "production.observation",
                "bounded nonblank recipe/value required",
            ));
        }
        Ok(Self {
            recipe: recipe.into(),
            value: value.into(),
        })
    }
    pub fn recipe(&self) -> &str {
        &self.recipe
    }
    pub fn value(&self) -> &str {
        &self.value
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionBindings {
    pub resolution: ProductionObservation,
    pub declaration: ProductionObservation,
    pub target: ProductionObservation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionRequestIdentity(String);
impl ProductionRequestIdentity {
    pub fn recipe(&self) -> &'static str {
        PRODUCTION_REQUEST_RECIPE
    }
    pub fn digest(&self) -> &str {
        &self.0
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeOperationRole {
    Production,
    CurrentSourceVerification,
}
/// Fixed selection template. No execution graph, status or write grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedFullPass {
    intent: IntentIdentity,
    contribution: WorkIdentity,
    work: String,
    consumer: String,
    selected: String,
}
impl SelectedFullPass {
    pub fn select(
        change: &ChangeManifest,
        inputs: &[NormativeInput<'_>],
        graph: &WorkGraph,
        work: &str,
    ) -> Result<Self, ChangeError> {
        let intent = change.intent_identity(inputs)?;
        let contribution = graph.contribution_identity(change, &[work])?;
        Ok(Self {
            intent,
            contribution,
            work: work.into(),
            consumer: change.coordinate().into(),
            selected: change.selected_node().into(),
        })
    }
    pub fn intent(&self) -> &IntentIdentity {
        &self.intent
    }
    pub fn contribution(&self) -> &WorkIdentity {
        &self.contribution
    }
    pub fn work(&self) -> &str {
        &self.work
    }
    pub fn consumer(&self) -> &str {
        &self.consumer
    }
    pub fn selected_node(&self) -> &str {
        &self.selected
    }
    pub fn operations(&self) -> [ChangeOperationRole; 2] {
        [
            ChangeOperationRole::Production,
            ChangeOperationRole::CurrentSourceVerification,
        ]
    }
    /// Exact observed execution bindings are required; no invocation label,
    /// run occurrence or clock enters this proposed request recipe.
    pub fn bind(&self, bindings: &ProductionBindings) -> ProductionRequestIdentity {
        let mut h = Sha256::new();
        h.update(PRODUCTION_REQUEST_RECIPE);
        h.update(b"\0epoch=1\0");
        fn field(h: &mut Sha256, label: &str, value: &str) {
            h.update((label.len() as u64).to_be_bytes());
            h.update(label);
            h.update((value.len() as u64).to_be_bytes());
            h.update(value);
        }
        for (label, value) in [
            ("intent.recipe", self.intent.recipe()),
            ("intent.digest", self.intent.digest()),
            ("work.selected", self.work()),
            ("work.recipe", self.contribution.recipe()),
            ("work.digest", self.contribution.digest()),
            ("consumer", self.consumer()),
            ("selected-node", self.selected_node()),
            ("resolution.recipe", bindings.resolution.recipe()),
            ("resolution.value", bindings.resolution.value()),
            ("declaration.recipe", bindings.declaration.recipe()),
            ("declaration.value", bindings.declaration.value()),
            ("target.recipe", bindings.target.recipe()),
            ("target.value", bindings.target.value()),
        ] {
            field(&mut h, label, value);
        }
        ProductionRequestIdentity(format!("sha256:{:x}", h.finalize()))
    }
}
