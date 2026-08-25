//! Closed builtin-handler registry.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#H-BUILTIN");

use specmark::spec;
use vibe_wire::generated::lifecycle::e1::context::Context;
use vibe_wire::generated::lifecycle::e1::reply::{Reply, ReplyStatus};

use crate::ExtensionRegistryRow;

use super::DispatchError;

/// The explicit closed registry of builtin handlers shipped in this binary.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-BUILTIN")]
pub struct BuiltinRegistry;

impl BuiltinRegistry {
    /// Stable names accepted by this build of vibe.
    pub const NAMES: &'static [&'static str] = &["log"];

    /// Dispatch exactly one known builtin against the canonical envelope.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#H-BUILTIN")]
    pub fn dispatch(
        name: &str,
        row: &ExtensionRegistryRow,
        envelope: &Context,
    ) -> Result<Reply, DispatchError> {
        match name {
            "log" => log(row, envelope),
            unknown => Err(DispatchError::UnknownBuiltin {
                key: row.key().clone(),
                name: unknown.to_string(),
            }),
        }
    }
}

fn log(row: &ExtensionRegistryRow, envelope: &Context) -> Result<Reply, DispatchError> {
    let message = envelope
        .execution
        .config
        .get("message")
        .and_then(Option::as_ref)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| DispatchError::InvalidLogConfig {
            key: row.key().clone(),
            reason: "required field `message` must be a string".to_string(),
        })?;
    let rendered = render_message(message, envelope);
    Ok(Reply {
        artifacts: Vec::new(),
        envelope: 1,
        status: ReplyStatus::Ok,
        tasks: Vec::new(),
        message: Some(rendered),
    })
}

/// Expand only tokens present in the original template. Inserted authored
/// values are appended directly and are never recursively interpreted.
fn render_message(template: &str, envelope: &Context) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut remaining = template;
    while !remaining.is_empty() {
        if let Some(rest) = remaining.strip_prefix("{phase}") {
            rendered.push_str(&envelope.run.phase);
            remaining = rest;
        } else if let Some(rest) = remaining.strip_prefix("{project}") {
            rendered.push_str(&envelope.project.name);
            remaining = rest;
        } else if let Some(rest) = remaining.strip_prefix("{package}") {
            rendered.push_str(&envelope.execution.package);
            remaining = rest;
        } else {
            let Some(character) = remaining.chars().next() else {
                break;
            };
            rendered.push(character);
            remaining = &remaining[character.len_utf8()..];
        }
    }
    rendered
}
