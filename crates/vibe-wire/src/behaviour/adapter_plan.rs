//! Semantic admission shared by all prepared adapter surfaces.
use crate::generated::adapter_plan::e1::{error as e, request as q, result as r};

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn relative(value: &str) -> bool {
    text(value)
        && !value.contains(['\\', ':'])
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
fn text(value: &str) -> bool {
    !value.is_empty() && value.len() <= 4096 && !value.chars().any(char::is_control)
}

/// Preserve JTD optional-property typing before serde's Option collapses null.
pub fn read_request(bytes: &[u8]) -> Result<q::Request, &'static str> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "invalid request JSON")?;
    if value
        .get("command")
        .and_then(|v| v.get("capture_id"))
        .is_some_and(|v| !v.is_string())
    {
        return Err("capture must be a string when present");
    }
    let parsed = serde_json::from_value(value).map_err(|_| "invalid generated request")?;
    Ok(parsed)
}

pub fn read_result(bytes: &[u8]) -> Result<r::Result, &'static str> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "invalid result JSON")?;
    optional_strings(&value, &["capture_id", "profile", "recorded_digest"])?;
    let parsed = serde_json::from_value(value).map_err(|_| "invalid generated result")?;
    result(&parsed)?;
    Ok(parsed)
}
pub fn read_error(bytes: &[u8]) -> Result<e::Error, &'static str> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "invalid error JSON")?;
    optional_strings(&value, &["context_id", "target_id", "capture_id", "reason"])?;
    let parsed = serde_json::from_value(value).map_err(|_| "invalid generated error")?;
    error(&parsed)?;
    Ok(parsed)
}
fn optional_strings(value: &serde_json::Value, names: &[&str]) -> Result<(), &'static str> {
    match value {
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                if names.contains(&key.as_str()) && !value.is_string() {
                    return Err("wrong optional property type");
                }
                optional_strings(value, names)?;
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                optional_strings(value, names)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn request(value: &q::Request) -> Result<(), &'static str> {
    if value.schema != 1 {
        return Err("unsupported epoch");
    }
    if !text(&value.context_id) || !text(&value.target_id) {
        return Err("invalid binding");
    }
    if let q::Command::Plan(plan) = &value.command
        && plan.capture_id.as_deref().is_some_and(|id| !text(id))
    {
        return Err("invalid capture");
    }
    Ok(())
}

pub fn error(value: &e::Error) -> Result<(), &'static str> {
    if value.schema != 1 || !text(&value.message) {
        return Err("invalid error");
    }
    if (value.code == e::ErrorCode::NeedsPreparation) != value.reason.is_some() {
        return Err("reason/code mismatch");
    }
    if value.context_id.is_some() != value.target_id.is_some()
        || value.capture_id.is_some() && value.context_id.is_none()
    {
        return Err("incomplete error binding");
    }
    for id in [&value.context_id, &value.target_id, &value.capture_id]
        .into_iter()
        .flatten()
    {
        if !text(id) {
            return Err("invalid error identity");
        }
    }
    Ok(())
}

pub fn result(value: &r::Result) -> Result<(), &'static str> {
    if value.schema != 1
        || !text(&value.context_id)
        || !text(&value.target_id)
        || !text(&value.result_id)
        || value.capture_id.as_deref().is_some_and(|id| !text(id))
    {
        return Err("invalid result binding");
    }
    match &value.body {
        r::Body::Session(session) => {
            if !session.complete
                || session.profile.as_deref().is_some_and(|p| !text(p))
                || session
                    .source_evidence
                    .iter()
                    .chain(&session.client_evidence)
                    .any(|id| !text(id))
            {
                return Err("invalid session evidence");
            }
            if (session.preparation == r::BodySessionPreparation::Ready)
                != value.capture_id.is_some()
            {
                return Err("preparation/capture mismatch");
            }
            if (session.adapter_kind == r::BodySessionAdapterKind::Deploy)
                != session.profile.is_some()
            {
                return Err("profile/kind mismatch");
            }
        }
        r::Body::Plan(plan) => {
            if value.capture_id.is_none() || !plan.complete {
                return Err("incomplete plan");
            }
            for row in &plan.rows {
                match row {
                    r::Row::Skill(skill) => {
                        if !text(&skill.provider)
                            || !text(&skill.declaration)
                            || skill.files.is_empty()
                            || !relative(&skill.target)
                            || !text(&skill.agent)
                            || !skill
                                .source_digest
                                .strip_prefix("sha256:")
                                .is_some_and(digest)
                        {
                            return Err("incomplete skill");
                        }
                        let mut previous: Option<&str> = None;
                        for file in &skill.files {
                            if !relative(&file.path)
                                || previous.is_some_and(|p| p >= file.path.as_str())
                                || !digest(&file.sha256)
                                || file.bytes.parse::<u64>().ok().map(|n| n.to_string())
                                    != Some(file.bytes.clone())
                            {
                                return Err("invalid selected file");
                            }
                            previous = Some(&file.path);
                        }
                    }
                    r::Row::Deploy(deploy) => {
                        if !text(&deploy.target)
                            || !text(&deploy.provider)
                            || !digest(&deploy.config_digest)
                            || !digest(&deploy.fingerprint)
                            || deploy.client_evidence.iter().any(|id| !text(id))
                            || deploy.resources.iter().any(|r| {
                                !text(&r.resource)
                                    || !digest(&r.desired_digest)
                                    || r.recorded_digest.as_deref().is_some_and(|d| !digest(d))
                            })
                        {
                            return Err("invalid deploy row");
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Validate the publication envelope as well as its generated domain payload.
pub fn publication(
    value: &crate::generated::machine::e1::operation_envelope::OperationEnvelope,
) -> Result<(), &'static str> {
    use crate::generated::machine::e1::operation_envelope as env;
    if value.schema != 1
        || value.operation_id.as_deref() != Some("rust:vibe-orchestrator:adapter-plan")
        || value.domain_owner.as_deref() != Some("rust:vibe-orchestrator")
        || value.source_id.as_deref() != Some("prepared-adapter-plan/1")
        || value.evidence_id.is_some()
    {
        return Err("foreign envelope");
    }
    match &value.body {
        env::Body::Result(body) => {
            if body.effect_state != env::BodyResultEffectState::NoEffects
                || body.completeness != env::BodyResultCompleteness::Complete
                || body.format_id.as_deref() != Some("adapter-plan-result")
            {
                return Err("invalid query result effects/completeness");
            }
            let parsed = read_result(
                body.payload_json
                    .as_deref()
                    .ok_or("missing result")?
                    .as_bytes(),
            )?;
            result(&parsed)?;
            if parsed.capture_id != body.provenance_id {
                return Err("foreign result provenance");
            }
        }
        env::Body::Error(body) => {
            if body.effect_state != env::BodyErrorEffectState::NoEffects
                || body.completeness != env::BodyErrorCompleteness::Unavailable
                || body.format_id.as_deref() != Some("adapter-plan-error")
            {
                return Err("invalid query error effects/completeness");
            }
            let parsed = read_error(
                body.payload_json
                    .as_deref()
                    .ok_or("missing error")?
                    .as_bytes(),
            )?;
            error(&parsed)?;
            if parsed.capture_id != body.provenance_id {
                return Err("foreign error provenance");
            }
        }
        _ => return Err("not a query publication"),
    }
    Ok(())
}
pub fn result_for(request: &q::Request, result_value: &r::Result) -> Result<(), &'static str> {
    result(result_value)?;
    if request.context_id != result_value.context_id || request.target_id != result_value.target_id
    {
        return Err("foreign result context");
    }
    if let q::Command::Plan(plan) = &request.command {
        if plan.capture_id != result_value.capture_id
            || !matches!(result_value.body, r::Body::Plan(_))
        {
            return Err("foreign plan capture");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn golden_readers_reject_foreign_binding_and_effect_completeness() {
        use super::publication_cases::*;
        use crate::generated::machine::e1::operation_envelope as env;
        let request = read_request(REQUEST).unwrap();
        let plan = read_result(PLAN).unwrap();
        result_for(&request, &plan).unwrap();
        read_result(SESSION).unwrap();
        read_error(ERROR).unwrap();
        let mut foreign = plan.clone();
        foreign.context_id = "foreign".into();
        assert!(result_for(&request, &foreign).is_err());
        foreign = plan.clone();
        foreign.capture_id = Some("foreign".into());
        assert!(result_for(&request, &foreign).is_err());
        let mut envelope = env::OperationEnvelope {
            schema: 1,
            operation_id: Some("rust:vibe-orchestrator:adapter-plan".into()),
            domain_owner: Some("rust:vibe-orchestrator".into()),
            source_id: Some("prepared-adapter-plan/1".into()),
            evidence_id: None,
            body: env::Body::Result(Box::new(env::BodyResult {
                format_id: Some("adapter-plan-result".into()),
                payload_json: Some(String::from_utf8(PLAN.to_vec()).unwrap()),
                provenance_id: Some("capture".into()),
                effect_state: env::BodyResultEffectState::NoEffects,
                completeness: env::BodyResultCompleteness::Complete,
            })),
        };
        publication(&envelope).unwrap();
        let env::Body::Result(body) = &mut envelope.body else {
            unreachable!()
        };
        body.completeness = env::BodyResultCompleteness::Partial;
        assert!(publication(&envelope).is_err());
        let env::Body::Result(body) = &mut envelope.body else {
            unreachable!()
        };
        body.completeness = env::BodyResultCompleteness::Complete;
        body.effect_state = env::BodyResultEffectState::Completed;
        assert!(publication(&envelope).is_err());
        for bytes in [PLAN, SESSION] {
            let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            value["capture_id"] = json!(null);
            assert!(read_result(&serde_json::to_vec(&value).unwrap()).is_err());
        }
        let mut error: serde_json::Value = serde_json::from_slice(ERROR).unwrap();
        error["reason"] = json!(17);
        assert!(read_error(&serde_json::to_vec(&error).unwrap()).is_err());
    }
    #[test]
    fn generated_request_rejects_unknown_and_wrong_optional_types() {
        let base = json!({"schema":1,"context_id":"context","target_id":"target","command":{"action":"plan"}});
        let parsed: q::Request = serde_json::from_value(base.clone()).unwrap();
        request(&parsed).unwrap();
        for bad in [json!(true), json!(null), json!(1)] {
            let mut value = base.clone();
            value["command"]["capture_id"] = bad;
            assert!(read_request(&serde_json::to_vec(&value).unwrap()).is_err());
        }
        let mut value = base.clone();
        value["root"] = json!("/foreign");
        assert!(read_request(&serde_json::to_vec(&value).unwrap()).is_err());
        let mut value = base;
        value["schema"] = json!(2);
        assert!(request(&serde_json::from_value(value).unwrap()).is_err());
    }
    #[test]
    fn currentness_unavailable_reasons_roundtrip_and_reject_malformed_combinations() {
        let corpus: e::Error = serde_json::from_slice(include_bytes!(
            "../../../../formats/corpora/adapter-plan/e1/currentness-unavailable.json"
        ))
        .unwrap();
        error(&corpus).unwrap();
        for reason in ["client-state-unavailable", "client-executable-unavailable"] {
            let value = json!({"schema":1,"code":"needs-preparation","reason":reason,"message":"currentness unavailable"});
            let parsed: e::Error = serde_json::from_value(value.clone()).unwrap();
            error(&parsed).unwrap();
            assert_eq!(
                serde_json::from_slice::<e::Error>(&serde_json::to_vec(&parsed).unwrap()).unwrap(),
                parsed
            );
            let mut malformed = parsed;
            malformed.code = e::ErrorCode::Internal;
            assert!(error(&malformed).is_err());
            let mut unknown = value;
            unknown["reason"] = json!("guess-vendor-state");
            assert!(serde_json::from_value::<e::Error>(unknown).is_err());
        }
    }
    #[test]
    fn error_requires_reason_only_for_preparation_and_preserves_binding() {
        let mut value: e::Error = serde_json::from_value(json!({"schema":1,"code":"needs-preparation","reason":"source-capture-missing","message":"prepare"})).unwrap();
        error(&value).unwrap();
        let encoded = serde_json::to_vec(&value).unwrap();
        assert_eq!(serde_json::from_slice::<e::Error>(&encoded).unwrap(), value);
        value.code = e::ErrorCode::WrongContext;
        assert!(error(&value).is_err());
        value.reason = None;
        value.capture_id = Some("foreign".into());
        assert!(error(&value).is_err());
        value.capture_id = None;
        error(&value).unwrap();
        value.code = e::ErrorCode::NeedsPreparation;
        assert!(error(&value).is_err());
    }
    #[test]
    fn session_and_complete_ordered_plan_roundtrip() {
        let mut value = json!({"schema":1,"context_id":"context","target_id":"target","result_id":"result","capture_id":"capture","body":{"kind":"session","adapter_kind":"skills","scope":"project","preparation":"ready","source_evidence":["source"],"client_evidence":[],"complete":true}});
        let session: r::Result = serde_json::from_value(value.clone()).unwrap();
        result(&session).unwrap();
        value["body"] = json!({"kind":"plan","complete":true,"rows":[{"kind":"skill","provider":"org.example/fixture","declaration":"demo","agent":"claude","scope":"project","target":".claude/skills/demo","source_digest":format!("sha256:{}", "a".repeat(64)),"proof":"observed","ownership":"unavailable","files":[{"path":"SKILL.md","sha256":"a".repeat(64),"bytes":"4"}]}]});
        let plan: r::Result = serde_json::from_value(value.clone()).unwrap();
        result(&plan).unwrap();
        assert_eq!(
            serde_json::from_slice::<r::Result>(&serde_json::to_vec(&plan).unwrap()).unwrap(),
            plan
        );
        value["body"]["complete"] = json!(false);
        assert!(result(&serde_json::from_value(value.clone()).unwrap()).is_err());
        value["body"]["complete"] = json!(true);
        value.as_object_mut().unwrap().remove("capture_id");
        assert!(result(&serde_json::from_value(value).unwrap()).is_err());
    }
}

#[cfg(test)]
mod publication_cases {
    // Kept outside the registered family only as shared fixtures, not tests.
    pub const REQUEST: &[u8] =
        include_bytes!("../../../../formats/corpora/adapter-plan/e1/request.json");
    pub const PLAN: &[u8] = include_bytes!("../../../../formats/corpora/adapter-plan/e1/plan.json");
    pub const SESSION: &[u8] =
        include_bytes!("../../../../formats/corpora/adapter-plan/e1/session.json");
    pub const ERROR: &[u8] =
        include_bytes!("../../../../formats/corpora/adapter-plan/e1/error.json");
}
