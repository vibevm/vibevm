//! Host-injected finite local source capture, separate from retained preview.
use super::*;

#[derive(Debug)]
pub enum ScenarioFailure {
    Incompatible(String),
    Incomplete(String),
}
pub trait ScenarioProducer: Send + Sync {
    fn capture_with_derivatives(
        &self,
        host: hypothetical::HostCapture,
        draft: &scenario::Draft,
        target: &str,
        _derivatives: &vibe_workspace::source_query::derivatives::DerivativeCatalog,
    ) -> Result<hypothetical::PreparedScenario, ScenarioFailure> {
        self.capture(host, draft, target)
    }

    fn capture(
        &self,
        host: hypothetical::HostCapture,
        draft: &scenario::Draft,
        target: &str,
    ) -> Result<hypothetical::PreparedScenario, ScenarioFailure>;
}
impl BootDebuggerRoute {
    pub fn with_scenario_producer(mut self, producer: Arc<dyn ScenarioProducer>) -> Self {
        self.scenario_producer = Some(producer);
        self
    }
    pub(super) fn scenario(
        &self,
        state: &mut State,
        command: &q::CommandScenario,
    ) -> Answer<r::Result> {
        let mut body = r::BodyPreview {
            produced_snapshot_id: None,
            scenario_id: String::new(),
            baseline_world: String::new(),
            candidate_world: String::new(),
            candidate_catalog: String::new(),
            target: command.target.clone(),
            status: r::BodyPreviewStatus::Incomplete,
            missing: vec![],
            placements: vec![],
            artifact: None,
            compiler_carrier_json: None,
            measurement_json: None,
            comparison_json: None,
        };
        let produce = || {
            let draft: scenario::Draft = serde_json::from_str(&command.draft_json)
                .map_err(|e| ScenarioFailure::Incompatible(e.to_string()))?;
            let host = Self::captured_host(state, &command.snapshot_id)
                .map_err(ScenarioFailure::Incomplete)?;
            self.scenario_producer
                .as_ref()
                .ok_or_else(|| {
                    ScenarioFailure::Incomplete("local scenario producer not configured".into())
                })?
                .capture_with_derivatives(host, &draft, &command.target, &state.derivatives)
        };
        match produce() {
            Ok(scenario) => {
                let request = q::CommandPreview {
                    scenario_id: scenario.identity().into(),
                    expected_baseline: scenario.baseline().identity().into(),
                    expected_candidate: scenario.candidate().identity().into(),
                    expected_catalog: scenario.candidate().catalog().into(),
                    loading_overrides: BTreeMap::new(),
                };
                state.scenarios.insert(scenario.identity().into(), scenario);
                self.preview(state, &request)
            }
            Err(error) => {
                match error {
                    ScenarioFailure::Incompatible(message) => {
                        body.status = r::BodyPreviewStatus::Incompatible;
                        body.missing.push(message);
                    }
                    ScenarioFailure::Incomplete(message) => body.missing.push(message),
                }
                self.result(None, r::Body::Preview(Box::new(body)))
            }
        }
    }
}
impl From<String> for ScenarioFailure {
    fn from(s: String) -> Self {
        Self::Incomplete(s)
    }
}
impl From<&str> for ScenarioFailure {
    fn from(s: &str) -> Self {
        Self::Incomplete(s.into())
    }
}
