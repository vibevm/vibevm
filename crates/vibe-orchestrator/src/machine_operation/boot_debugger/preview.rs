use super::*;

impl BootDebuggerRoute {
    pub(super) fn preview(&self, state: &State, request: &q::CommandPreview) -> Answer<r::Result> {
        let scenario = state.scenarios.get(&request.scenario_id);
        let mut body = r::BodyPreview {
            scenario_id: request.scenario_id.clone(),
            baseline_world: request.expected_baseline.clone(),
            candidate_world: request.expected_candidate.clone(),
            candidate_catalog: request.expected_catalog.clone(),
            target: scenario.map(|s| s.target().to_owned()).unwrap_or_default(),
            status: r::BodyPreviewStatus::Incomplete,
            missing: vec![],
            placements: vec![],
            artifact: None,
            compiler_carrier_json: None,
            measurement_json: None,
            comparison_json: None,
        };
        match scenario {
            None => body.missing.push(format!(
                "prepared scenario capture: {}",
                request.scenario_id
            )),
            Some(scenario) => match scenario.evaluate(
                &request.scenario_id,
                &request.expected_baseline,
                &request.expected_candidate,
                &request.expected_catalog,
                &request.loading_overrides,
            ) {
                Err(error) => {
                    body.status = if error.starts_with("stale") {
                        r::BodyPreviewStatus::Stale
                    } else {
                        r::BodyPreviewStatus::Incompatible
                    };
                    body.missing.push(error);
                }
                Ok(preview) => {
                    body.status = if preview.status == "complete" {
                        r::BodyPreviewStatus::Complete
                    } else {
                        r::BodyPreviewStatus::Incomplete
                    };
                    body.missing = preview.missing;
                    body.placements = preview
                        .placements
                        .into_iter()
                        .map(|p| r::Placement {
                            origin: p.origin,
                            path: p.path,
                            loading: p.loading,
                            when: p.when,
                        })
                        .collect();
                    body.artifact = preview.artifact;
                    body.compiler_carrier_json = preview.carrier.as_ref().map(json).transpose()?;
                    body.measurement_json = preview.measurement.as_ref().map(json).transpose()?;
                    body.comparison_json = preview.comparison.as_ref().map(json).transpose()?;
                }
            },
        }
        self.result(None, r::Body::Preview(Box::new(body)))
    }
}
