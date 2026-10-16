//! Closed client deployment evidence. These values never construct authority.
use crate::generated::{
    client_deploy::e1::{
        reply::{Reply, ReplyStatus, StepStatus},
        request::Request,
    },
    deploy_client_state::DeployClientState,
};
fn text(v: &str) -> bool {
    !v.is_empty() && v.len() <= 4096 && !v.chars().any(char::is_control)
}
pub fn read_request(bytes: &[u8]) -> Result<Request, &'static str> {
    if bytes.len() > 65536 {
        return Err("request too large");
    }
    let q: Request = serde_json::from_slice(bytes).map_err(|_| "request grammar")?;
    if q.schema != 1
        || !text(&q.plan_id)
        || !text(&q.key)
        || q.run_id.as_deref().is_some_and(|s| !text(s))
    {
        return Err("request identity");
    }
    Ok(q)
}
pub fn validate_reply(r: &Reply) -> Result<(), &'static str> {
    if r.schema != 1 || !text(&r.plan_id) || r.resources.len() > 128 || r.steps.len() > 128 {
        return Err("reply identity or size");
    }
    let mut seen = std::collections::BTreeSet::new();
    for row in &r.resources {
        if !text(&row.resource) || !seen.insert(&row.resource) {
            return Err("duplicate resource");
        }
        for value in [&row.prior, &row.desired, &row.observed] {
            if value != "absent"
                && !(value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
            {
                return Err("resource digest spelling");
            }
        }
    }
    let mut steps = std::collections::BTreeSet::new();
    if r.steps
        .iter()
        .any(|s| !seen.contains(&s.resource) || !text(&s.subject) || !steps.insert(&s.resource))
    {
        return Err("step resource join");
    }
    if r.status == ReplyStatus::Completed
        && (r.run_id.is_empty()
            || r.steps.iter().any(|s| s.status != StepStatus::Completed)
            || r.resources.iter().any(|row| {
                row.observed != row.desired
                    || row.state
                        != crate::generated::client_deploy::e1::reply::ResourceState::Desired
            })
            || r.authority != crate::generated::client_deploy::e1::reply::ReplyAuthority::Admitted)
    {
        return Err("forged complete prefix");
    }
    if !r.steps.is_empty()
        && r.run_id.is_empty()
        && r.steps.iter().any(|s| s.status != StepStatus::Pending)
    {
        return Err("effect lacks run");
    }
    Ok(())
}
pub fn read_reply(bytes: &[u8]) -> Result<Reply, &'static str> {
    if bytes.len() > 1024 * 1024 {
        return Err("reply too large");
    }
    let r = serde_json::from_slice(bytes).map_err(|_| "reply grammar")?;
    validate_reply(&r)?;
    Ok(r)
}
pub fn read_state(bytes: &[u8]) -> Result<DeployClientState, &'static str> {
    if bytes.len() > 1024 * 1024 {
        return Err("state too large");
    }
    let r: DeployClientState = serde_json::from_slice(bytes).map_err(|_| "state grammar")?;
    if r.schema != 1
        || !text(&r.deployment)
        || !text(&r.scope)
        || !text(&r.subject)
        || !text(&r.run_id)
        || r.plan_id != r.reply.plan_id
        || r.run_id != r.reply.run_id
    {
        return Err("retained identity join");
    }
    use sha2::{Digest, Sha256};
    let mut commands = std::collections::BTreeSet::new();
    for command in &r.commands {
        let plan = crate::behaviour::client_command::read_plan(
            &serde_json::to_vec(&command.plan).map_err(|_| "command encoding")?,
        )?;
        let subject = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&plan).map_err(|_| "command encoding")?)
        );
        if !commands.insert(&command.resource)
            || !r
                .reply
                .steps
                .iter()
                .any(|step| step.resource == command.resource && step.subject == subject)
        {
            return Err("retained command plan join");
        }
    }
    if r.file_target.is_none() && r.commands.len() != r.reply.steps.len() {
        return Err("missing retained commands");
    }
    if r.file_target.is_some() != r.file_subject.is_some() {
        return Err("partial file operation join");
    }
    if r.finalization_digest.is_some() != r.finalization_generation.is_some()
        || r.finalization_digest
            .as_ref()
            .is_some_and(|d| d.len() != 64 || !d.bytes().all(|b| b.is_ascii_hexdigit()))
        || r.finalization_generation == Some(0)
        || (r.finalization_digest.is_some() && r.published)
    {
        return Err("invalid finalization basis");
    }
    read_reply(&serde_json::to_vec(&r.reply).map_err(|_| "reply encoding")?)?;
    Ok(r)
}
#[cfg(test)]
mod m_08_d_2_cases;
