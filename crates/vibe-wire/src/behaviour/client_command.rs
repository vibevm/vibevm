//! Inert closed command artifacts; serialization never grants execution.
use crate::generated::client_command::e1::{plan as p, request as q};
fn text(s: &str) -> bool {
    !s.is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control)
}
pub fn request(v: &q::Request) -> Result<(), &'static str> {
    if v.schema != 1 || v.contract_version != 1 || !text(&v.version) {
        return Err("unknown command contract");
    }
    use q::RequestOperation::*;
    match v.operation {
        Version | List if v.coordinate.is_none() && v.path.is_none() => {}
        Install | Remove
            if v.path.is_none()
                && v.coordinate
                    .as_deref()
                    .is_some_and(|s| text(s) && !s.starts_with('-')) => {}
        MarketplaceAdd
            if v.coordinate.is_none()
                && v.path.as_deref().is_some_and(|s| {
                    text(s)
                        && std::path::Path::new(s).is_absolute()
                        && !std::path::Path::new(s)
                            .components()
                            .any(|c| matches!(c, std::path::Component::ParentDir))
                }) => {}
        _ => return Err("invalid typed parameters"),
    }
    Ok(())
}
pub fn read_request(bytes: &[u8]) -> Result<q::Request, &'static str> {
    let v = serde_json::from_slice(bytes).map_err(|_| "invalid request grammar")?;
    request(&v)?;
    Ok(v)
}
pub fn read_plan(bytes: &[u8]) -> Result<p::Plan, &'static str> {
    let v: p::Plan = serde_json::from_slice(bytes).map_err(|_| "invalid plan grammar")?;
    // The nested generated request has the same closed schema, but its Rust
    // type is local to the plan family.
    read_request(&serde_json::to_vec(&v.request).map_err(|_| "request encoding")?)?;
    if v.schema != 1
        || [&v.executable, &v.resolved, &v.cwd]
            .iter()
            .any(|s| !text(s) || !std::path::Path::new(s).is_absolute())
        || [
            &v.environment_ref,
            &v.principal,
            &v.policy,
            &v.evidence,
            &v.executable_identity,
            &v.version_reply,
        ]
        .iter()
        .any(|s| !text(s))
        || v.argv.is_empty()
        || v.roots.is_empty()
        || v.effects.is_empty()
        || v.executable_digest.len() != 64
        || !v.executable_digest.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("incomplete command evidence");
    }
    Ok(v)
}
#[cfg(test)]
mod m_08_d_1_cases;
