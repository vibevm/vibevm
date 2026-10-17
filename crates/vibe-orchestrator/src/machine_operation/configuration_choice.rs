//! Explicit answers over a retained M16/B2/C1 basis. Transport never admits definitions.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003-dep-evolution#M05E4-CAPTURE"
);
use crate::document_analysis::PreparedConfigurationDiscovery;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{path::Path, sync::Mutex};
use vibe_install::{InstallRequest, InstallSource};
use vibe_resolver::resolvo_engine::{
    configuration::DefaultBinding,
    request::{ConfigurationInput, NormalizedRequest},
};
use vibe_wire::generated::consumer_configuration::e1::{
    request::{EdgePolicy, Request},
    result::OutcomeStatus,
};

#[cfg(test)]
use crate as vibe_orchestrator;
#[cfg(test)]
#[path = "configuration_choice_fixture.rs"]
mod fixture;
#[cfg(test)]
mod m_05_e_4_cases {
    use super::*;
    fn answer(host: &ConfigurationChoiceHost, defaults: bool) -> ChoiceOperation {
        let q = host.questions();
        ChoiceOperation::Prepare {
            capture: host.capture().into(),
            shown: q["shown"].as_str().unwrap().into(),
            request: serde_json::from_value(q["request"].clone()).unwrap(),
            allow_defaults: defaults,
        }
    }
    #[test]
    fn shown_defaults_prepare_real_graph_and_apply_original_intent() {
        let (_dir, host) = fixture::fixture();
        let q = host.questions();
        assert_eq!(
            q["definitions"][0]["definition"]["constraints"]["groups"]["choice"]["defaults"][0],
            "b"
        );
        let p = host.dispatch(answer(&host, true)).unwrap();
        assert_eq!(p["status"], "prepared");
        assert_eq!(p["graph"].as_array().unwrap().len(), 1);
        assert_eq!(
            p["request"]["edges"][0]["selections"]["dormant"]["members"][0],
            "later"
        );
        assert!(
            p["request"]["edges"][0]["selections"]
                .get("choice")
                .is_none()
        );
        host.dispatch(ChoiceOperation::Apply {
            capture: host.capture().into(),
            plan: p["plan"].as_str().unwrap().into(),
        })
        .unwrap();
        let lock = vibe_core::manifest::Lockfile::read(host.root().join("vibe.lock")).unwrap();
        let (requested, _) = vibe_wire::behaviour::lock_configuration::decode(
            lock.meta.configuration.as_ref().unwrap(),
        )
        .unwrap();
        assert_eq!(requested.defaults.len(), 1);
        assert!(lock.packages[0].features.contains(&"b".into()));
    }
    #[test]
    fn unattended_missing_does_not_choose_default_or_lexical_member() {
        let (_dir, host) = fixture::fixture();
        let p = host.dispatch(answer(&host, false)).unwrap();
        assert_eq!(p["status"], "needs_selection");
        assert!(!host.root().join("vibe.lock").exists());
        assert!(!host.root().join("vibedeps").exists());
    }
    #[test]
    fn unknown_and_conflicting_exact_refuse_before_writes() {
        for members in [vec!["unknown"], vec!["a", "b"]] {
            let (_dir, host) = fixture::fixture();
            let mut op = answer(&host, false);
            if let ChoiceOperation::Prepare { request, .. } = &mut op {
                request.edges[0].selections.insert(
                    "choice".into(),
                    serde_json::from_value(json!({
                    "intent":"exact","provenance":"test:explicit","members":members}))
                    .unwrap(),
                );
            }
            let result = host.dispatch(op);
            assert!(result.is_err() || result.unwrap()["status"] == "needs_selection");
            assert!(!host.root().join("vibe.lock").exists());
        }
    }
    #[test]
    fn substituted_shown_identity_cannot_authorize_defaults() {
        let (_dir, host) = fixture::fixture();
        let mut op = answer(&host, true);
        if let ChoiceOperation::Prepare { shown, .. } = &mut op {
            shown.push('x');
        }
        assert!(host.dispatch(op).is_err());
        assert!(!host.root().join("vibe.lock").exists());
    }
    #[test]
    fn conflicting_consumers_and_unknown_group_refuse_without_mutation() {
        for conflict in [false, true] {
            let (_dir, host) = fixture::fixture();
            let mut operation = answer(&host, false);
            if let ChoiceOperation::Prepare { request, .. } = &mut operation {
                request.edges[0].selections.insert(
                    if conflict { "choice" } else { "foreign" }.into(),
                    serde_json::from_value(
                        json!({"intent":"exact","provenance":"consumer:explicit","members":["a"]}),
                    )
                    .unwrap(),
                );
                if conflict {
                    // Identity changes are rejected at this answer seam, before solving.
                    let mut other = request.edges[0].clone();
                    other.consumer = "other".into();
                    other.selections.get_mut("choice").unwrap().members = Some(vec!["b".into()]);
                    request.edges.push(other);
                }
            }
            assert!(host.dispatch(operation).is_err());
            assert!(!host.root().join("vibe.lock").exists());
        }
    }
    #[test]
    fn explicit_unattended_answer_retains_effects_and_provenance() {
        let (_dir, host) = fixture::fixture();
        let mut operation = answer(&host, false);
        if let ChoiceOperation::Prepare { request, .. } = &mut operation {
            request.edges[0].selections.insert(
                "choice".into(),
                serde_json::from_value(json!({
                "intent":"exact","members":["a"],"provenance":"machine:explicit-answer"}))
                .unwrap(),
            );
        }
        let p = host.dispatch(operation).unwrap();
        assert_eq!(p["status"], "prepared");
        assert_eq!(p["effects"][0]["config"]["picked"]["bool"], true);
        assert_eq!(
            p["request"]["edges"][0]["selections"]["choice"]["provenance"],
            "machine:explicit-answer"
        );
        assert!(!host.root().join("vibe.lock").exists());
    }
    #[test]
    fn active_controls_follow_existing_guard_evaluation() {
        let (_dir, host) = fixture::fixture();
        let q = host.questions();
        let mut op = ChoiceOperation::Evaluate {
            capture: host.capture().into(),
            shown: q["shown"].as_str().unwrap().into(),
            request: serde_json::from_value(q["request"].clone()).unwrap(),
            allow_defaults: false,
        };
        let before = host.dispatch(op.clone()).unwrap();
        let extras = |v: &Value| {
            v["controls"]
                .as_array()
                .unwrap()
                .iter()
                .find(|q| q["group"] == "extras")
                .unwrap()["members"]
                .clone()
        };
        assert_eq!(extras(&before), json!([]));
        if let ChoiceOperation::Evaluate { request, .. } = &mut op {
            request.edges[0].selections.insert(
                "choice".into(),
                serde_json::from_value(json!({
                "intent":"exact","members":["a"],"provenance":"test:answer"}))
                .unwrap(),
            );
        }
        assert_eq!(extras(&host.dispatch(op).unwrap()), json!(["optional"]));
        assert!(!host.root().join("vibe.lock").exists());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChoiceOperation {
    Questions {
        capture: String,
    },
    Evaluate {
        capture: String,
        shown: String,
        request: Request,
        allow_defaults: bool,
    },
    Prepare {
        capture: String,
        shown: String,
        request: Request,
        allow_defaults: bool,
    },
    Apply {
        capture: String,
        plan: String,
    },
}

/// A trusted embedding host supplies a source and the complete root union.
/// No wire value can construct either. Application is a separate explicit operation.
pub struct ConfigurationChoiceHost {
    basis: PreparedConfigurationDiscovery,
    request: NormalizedRequest,
    source: Box<dyn InstallSource + Send + Sync>,
    install: InstallRequest,
    format: vibe_core::manifest::SpecFormat,
    policy: Option<(
        vibe_core::user_config::SlotIntegrity,
        vibe_workspace::hooks::HookPolicy,
    )>,
    pending: Mutex<Option<(String, NormalizedRequest, vibe_install::PlannedInstall)>>,
    shown: String,
    initial: vibe_wire::generated::consumer_configuration::e1::result::Result,
}
impl ConfigurationChoiceHost {
    pub fn new(
        basis: PreparedConfigurationDiscovery,
        request: NormalizedRequest,
        source: Box<dyn InstallSource + Send + Sync>,
        install: InstallRequest,
        format: vibe_core::manifest::SpecFormat,
    ) -> Result<Self, String> {
        let c = request
            .configuration
            .as_ref()
            .ok_or("missing admitted consumer request")?;
        if basis.admitted_definitions().is_empty() || !basis.query().unavailable.is_empty() {
            return Err("normalized captured authority unavailable".into());
        }
        let definitions = |ds: &[vibe_resolver::resolvo_engine::configuration::CandidateConfiguration]| -> Result<Vec<_>, String> {
            ds.iter().map(|d| Ok((d.package.to_string(), d.version.to_string(), d.identity.clone(),
                vibe_install::configuration_definition(&d.definition).map_err(|e| e.to_string())?))).collect()
        };
        if definitions(&c.definitions)? != definitions(basis.admitted_definitions())?
            || !c.defaults.is_empty()
        {
            return Err(
                "request differs from captured admission or carries unshown consent".into(),
            );
        }
        if c.request
            .edges
            .iter()
            .any(|e| e.policy != EdgePolicy::RequireSelection)
        {
            return Err("initial request must require explicit selection".into());
        }
        vibe_wire::behaviour::consumer_configuration::admit_request(&c.request)?;
        let initial = vibe_resolver::features::configuration::resolve_consumer_configuration(
            &c.request,
            &c.definitions
                .iter()
                .map(|d| d.definition.clone())
                .collect::<Vec<_>>(),
        )?
        .result;
        if !install.roots.is_empty() {
            return Err("choice install preserves workspace roots".into());
        }
        let shown =
            crate::document_analysis::choice_identity(&json!({"capture":basis.capture_id(),
            "definitions":basis.query(),"request":c.request}));
        Ok(Self {
            basis,
            request,
            source,
            install,
            format,
            policy: None,
            pending: Mutex::new(None),
            shown,
            initial,
        })
    }
    pub fn with_application(
        mut self,
        integrity: vibe_core::user_config::SlotIntegrity,
        hooks: vibe_workspace::hooks::HookPolicy,
    ) -> Self {
        self.policy = Some((integrity, hooks));
        self
    }
    pub fn root(&self) -> &Path {
        self.basis.root()
    }
    pub fn capture(&self) -> &str {
        self.basis.capture_id()
    }
    pub fn questions(&self) -> Value {
        json!({"status":"needs_selection", "root":self.root(), "capture":self.capture(), "shown":self.shown,
            "offline":self.install.offline,
            "request":self.request.configuration.as_ref().unwrap().request,
            "questions":self.initial,
            "definitions":self.basis.query().candidates})
    }
    pub fn dispatch(&self, operation: ChoiceOperation) -> Result<Value, String> {
        let evaluate_only = matches!(&operation, ChoiceOperation::Evaluate { .. });
        let capture = match &operation {
            ChoiceOperation::Questions { capture }
            | ChoiceOperation::Prepare { capture, .. }
            | ChoiceOperation::Evaluate { capture, .. }
            | ChoiceOperation::Apply { capture, .. } => capture,
        };
        if capture != self.capture() {
            return Err("foreign capture".into());
        }
        match operation {
            ChoiceOperation::Questions { .. } => Ok(self.questions()),
            ChoiceOperation::Prepare {
                shown,
                mut request,
                allow_defaults,
                ..
            }
            | ChoiceOperation::Evaluate {
                shown,
                mut request,
                allow_defaults,
                ..
            } => {
                if shown != self.shown {
                    return Err("shown definition changed; rediscovery required".into());
                }
                let original = &self.request.configuration.as_ref().unwrap().request;
                let skeleton = |r: &Request| {
                    let mut r = r.clone();
                    for e in &mut r.edges {
                        e.selections.clear();
                        e.policy = EdgePolicy::RequireSelection;
                    }
                    r
                };
                if skeleton(&request) != skeleton(original) {
                    return Err("answer changed consumer identity or non-choice intent".into());
                }
                if !allow_defaults
                    && request
                        .edges
                        .iter()
                        .any(|e| e.policy != EdgePolicy::RequireSelection)
                {
                    return Err("defaults require explicit shown-definition consent".into());
                }
                for edge in &mut request.edges {
                    edge.policy = if allow_defaults {
                        EdgePolicy::UseDeclaredDefaults
                    } else {
                        EdgePolicy::RequireSelection
                    };
                }
                let edges = vibe_wire::behaviour::consumer_configuration::admit_request(&request)?;
                let candidates = self.basis.admitted_definitions();
                let mut defaults = Vec::new();
                for (i, edge) in edges.iter().enumerate() {
                    let matching: Vec<_> = candidates
                        .iter()
                        .filter(|d| d.definition.instance == edge.instance)
                        .collect();
                    if matching.len() != 1 {
                        return Err("ambiguous or missing shown candidate".into());
                    }
                    if allow_defaults {
                        defaults.push(DefaultBinding::shown(i, &request, matching[0]));
                    }
                }
                let evaluation =
                    vibe_resolver::features::configuration::resolve_consumer_configuration(
                        &request,
                        &candidates
                            .iter()
                            .map(|d| d.definition.clone())
                            .collect::<Vec<_>>(),
                    )?;
                if evaluate_only {
                    let controls=evaluation.questions.iter().map(|q| {
                        use vibe_core::manifest::feature_constraints::Cardinality::*;
                        json!({"edge":q.representative,"group":q.group.as_str(),"members":q.eligible.iter().map(|m|m.as_str()).collect::<Vec<_>>(),
                            "defaults":q.defaults.as_ref().map(|ms|ms.iter().map(|m|m.as_str()).collect::<Vec<_>>()),
                            "cardinality":match q.cardinality {ExactlyOne=>"exactly-one",ZeroOrOne=>"zero-or-one",OneOrMore=>"one-or-more",ZeroOrMore=>"zero-or-more"}})
                    }).collect::<Vec<_>>();
                    return Ok(
                        json!({"status":"questions", "capture":self.capture(),"shown":self.shown,
                        "request":request,"questions":evaluation.result,"controls":controls,"definitions":self.basis.query().candidates}),
                    );
                }
                if evaluation
                    .result
                    .outcomes
                    .iter()
                    .any(|o| o.status != OutcomeStatus::Resolved)
                {
                    return Ok(
                        json!({"status":"needs_selection", "capture":self.capture(), "shown":self.shown,
                        "request":request,"questions":evaluation.result,"definitions":self.basis.query().candidates}),
                    );
                }
                let mut normalized = self.request.clone();
                normalized.configuration = Some(ConfigurationInput {
                    request: request.clone(),
                    definitions: candidates.to_vec(),
                    defaults,
                });
                let _lease = crate::acquire_lease(self.root()).map_err(|e| e.to_string())?;
                let source = self
                    .source
                    .begin_normalized_transaction(&normalized)
                    .map_err(|e| e.to_string())?
                    .ok_or("source cannot bind normalized transaction")?;
                let plan = vibe_install::evolution::plan_reconfiguration(
                    &*source,
                    self.root(),
                    self.install.clone(),
                    self.format,
                    &vibe_install::NullObserver,
                )
                .map_err(|e| e.to_string())?;
                let graph=json!(plan.resolution.iter().map(|d|json!({"group":d.group,"name":d.name,"version":d.version,
                    "source_hash":d.source_hash,"requires":d.requires,"admitted_by":d.admitted_by,"via_override":d.via_override})).collect::<Vec<_>>());
                let effects=json!(evaluation.expansions.iter().map(|e| e.as_ref().map(|e| json!({
                    "active_features":e.active_features,"dependencies":e.active_deps,"dep_features":e.dep_features,
                    "weak_dep_features":e.weak_dep_features,"subskills":e.active_subskills,"config":e.config_values.iter().map(|(k,v)| {
                        use vibe_core::manifest::feature_effects::ConfigValue;
                        (k.clone(),match v {ConfigValue::Bool(v)=>json!({"bool":v}),ConfigValue::Integer(v)=>json!({"integer":v}),ConfigValue::String(v)=>json!({"string":v})})
                    }).collect::<serde_json::Map<String,Value>>()
                }))).collect::<Vec<_>>());
                let record = plan
                    .configuration_record()
                    .map_err(|e| e.to_string())?
                    .ok_or("prepared configuration record unavailable")?;
                let id = crate::document_analysis::choice_identity(
                    &json!({"shown":shown,"request":request,"graph":graph,"effects":effects,"record":record}),
                );
                *self.pending.lock().map_err(|_| "choice state poisoned")? =
                    Some((id.clone(), normalized, plan));
                Ok(
                    json!({"status":"prepared","capture":self.capture(),"shown":shown,"plan":id,
                    "request":request,"graph":graph,"effects":effects,"record":record}),
                )
            }
            ChoiceOperation::Apply { plan, .. } => {
                let (integrity, hooks) = self
                    .policy
                    .as_ref()
                    .ok_or("application authority unavailable")?;
                let _lease = crate::acquire_lease(self.root()).map_err(|e| e.to_string())?;
                let mut pending = self.pending.lock().map_err(|_| "choice state poisoned")?;
                if pending.as_ref().map(|p| &p.0) != Some(&plan) {
                    return Err("missing or stale prepared plan".into());
                }
                let (_, normalized, planned) = pending.take().unwrap();
                let source = self
                    .source
                    .begin_normalized_transaction(&normalized)
                    .map_err(|e| e.to_string())?
                    .ok_or("source cannot bind normalized transaction")?;
                vibe_install::apply_with_spec_format(
                    &*source,
                    planned,
                    *integrity,
                    self.format,
                    hooks,
                )
                .map_err(|e| e.to_string())?;
                Ok(json!({"status":"completed","capture":self.capture(),"plan":plan}))
            }
        }
    }
    /// Host-supplied connection only; no listener, process or implicit preparation.
    pub fn serve_cli(
        &self,
        connection: &mut (impl std::io::Read + std::io::Write),
    ) -> Result<(), String> {
        let operation = serde_json::from_reader(std::io::Read::take(&mut *connection, 1024 * 1024))
            .map_err(|e| e.to_string())?;
        let response = match self.dispatch(operation) {
            Ok(v) => v,
            Err(e) => json!({"status":"refused","reason":e}),
        };
        serde_json::to_writer(&mut *connection, &response).map_err(|e| e.to_string())?;
        std::io::Write::flush(connection).map_err(|e| e.to_string())
    }
}
