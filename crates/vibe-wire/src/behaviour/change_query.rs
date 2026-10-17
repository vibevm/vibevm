//! Strict generated change reads; no file/provider/effect authority here.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-READ-SERVICE");
pub use crate::generated::change_query::e1::{
    request::{Request, RequestMode, RequestRole},
    response::*,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
pub fn refused(
    code: ResponseRefusedCode,
    message: impl Into<String>,
    candidates: Vec<String>,
) -> Response {
    Response::Refused(Box::new(ResponseRefused {
        schema: 1,
        code,
        message: message.into(),
        candidates,
    }))
}
pub fn read_request(value: &Value) -> Result<Request, Response> {
    let request: Request = serde_json::from_value(value.clone()).map_err(|_| {
        refused(
            ResponseRefusedCode::InvalidRequest,
            "change query accepts only schema/mode/id/roles with declared types",
            vec![],
        )
    })?;
    validate_request(&request)?;
    Ok(request)
}
pub fn validate_request(request: &Request) -> Result<(), Response> {
    if request.schema != 1 {
        return Err(refused(
            ResponseRefusedCode::UnknownEpoch,
            "supported change query epoch is 1",
            vec!["1".into()],
        ));
    }
    // The core change owner validates ID grammar before preparation. This
    // wire layer bounds the scalar without maintaining a competing parser.
    if request.id.is_empty() || request.id.len() > 1024 {
        return Err(refused(
            ResponseRefusedCode::InvalidId,
            "bounded change identity required",
            vec![],
        ));
    }
    let roles: Vec<_> = request
        .roles
        .iter()
        .map(|role| {
            serde_json::to_value(role)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    if roles.iter().collect::<BTreeSet<_>>().len() != roles.len() {
        return Err(refused(
            ResponseRefusedCode::InvalidRequest,
            "duplicate role filter",
            vec![],
        ));
    }
    Ok(())
}
pub fn validate_response(response: &Response) -> Result<(), &'static str> {
    let row = match response {
        Response::Refused(row) => {
            return if row.schema == 1 && !row.message.trim().is_empty() {
                Ok(())
            } else {
                Err("change refusal epoch/diagnostic")
            };
        }
        Response::Success(row) => row,
    };
    if row.schema != 1
        || row.id.is_empty()
        || row.capture.is_empty()
        || row.intent_hash.is_empty()
        || row.work_hash.is_empty()
    {
        return Err("change result identity");
    }
    if row.scope_complete
        && (!row.gaps.is_empty()
            || row.origins.iter().any(|o| o.state != OriginState::Matched)
            || row.roles.iter().any(|r| {
                matches!(
                    r.state,
                    RoleState::Unclassified
                        | RoleState::Invalid
                        | RoleState::Unavailable
                        | RoleState::Inert
                )
            }))
    {
        return Err("incomplete change scope reported complete");
    }
    let mut facts = BTreeSet::new();
    for fact in &row.facts {
        if !facts.insert(&fact.metadata.address)
            || fact.capture.is_empty()
            || !fact.numeric_position_unavailable
        {
            return Err("change fact provenance");
        }
        let legacy: crate::generated::requirements_report::RequirementRow = serde_json::from_value(
            serde_json::to_value(&fact.metadata).map_err(|_| "fact serialization")?,
        )
        .map_err(|_| "fact metadata shape")?;
        super::requirements_report::validate_fact_metadata(&legacy)
            .map_err(|_| "fact metadata invariant")?;
    }
    let addresses: BTreeSet<_> = row
        .facts
        .iter()
        .map(|f| f.metadata.address.as_str())
        .collect();
    for origin in &row.origins {
        if origin.source_strengths.values().any(|strength| {
            !matches!(
                strength.as_str(),
                "observed" | "record-consistent" | "source-proven"
            )
        }) {
            return Err("change source strength");
        }
        if origin.state == OriginState::Matched
            && (origin.capture.as_deref().is_none_or(str::is_empty)
                || origin.declared_content.as_deref().is_none_or(str::is_empty)
                || origin.declared_content != origin.observed_content
                || origin.selected_addresses != origin.observed_addresses
                || origin
                    .selected_addresses
                    .iter()
                    .any(|a| !addresses.contains(a.as_str())))
        {
            return Err("matched origin inventory mismatch");
        }
    }
    for role in &row.roles {
        if (row.mode == ResponseSuccessMode::Show) != role.raw_hex.is_some() {
            return Err("raw role mode mismatch");
        }
        if let Some(raw) = &role.raw_hex {
            if raw.len() != role.bytes as usize * 2
                || !raw
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err("raw role encoding");
            }
        }
    }
    Ok(())
}
pub fn read_response(value: &Value) -> Result<Response, String> {
    let response: Response = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    validate_response(&response).map_err(str::to_owned)?;
    Ok(response)
}
pub fn request_json_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["schema","mode","id"],"properties":{"schema":{"type":"integer","const":1},"mode":{"type":"string","enum":["check","show","coverage"]},"id":{"type":"string","minLength":1,"maxLength":1024},"roles":{"type":"array","uniqueItems":true,"items":{"type":"string","enum":["intent","refinement","acceptance","work","closure","release-impact"]}}}})
}
#[cfg(test)]
mod m_02_d_2_cases;
