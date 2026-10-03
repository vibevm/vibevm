//! Registered JSON-RPC adapter for one explicitly granted fixed Cargo owner.
//! The request carries only action data; root, scope and mechanism plane come
//! exclusively from the private ServerContext route.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-CARGO-BUILD");

use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};
use serde::Serialize;
use serde_json::{Value, json};
use std::io::Write;
use vibe_orchestrator::machine_operation::fixed_cargo::{
    DOMAIN_OWNER, FixedCargoRoute, FixedCargoRouteError, OPERATION_ID, PUBLICATION_CAP, REQUEST_CAP,
};
use vibe_wire::generated::{
    format_id::FormatId,
    machine::e1::{
        fixed_cargo_action_error::FixedCargoActionError,
        fixed_cargo_action_reply::FixedCargoActionReply, operation_envelope as e,
    },
};

pub struct FixedCargoOperationMcpTool;

fn bounded<T: Serialize>(value: &T, cap: usize) -> Result<String, String> {
    struct Writer {
        bytes: Vec<u8>,
        cap: usize,
    }
    impl Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let size = self
                .bytes
                .len()
                .checked_add(bytes.len())
                .filter(|size| *size <= self.cap)
                .ok_or_else(|| std::io::Error::other("serialized request exceeds cap"))?;
            self.bytes.reserve(size - self.bytes.len());
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Writer {
        bytes: Vec::new(),
        cap,
    };
    serde_json::to_writer(&mut writer, value)
        .map_err(|_| "serialized request exceeds cap".to_owned())?;
    String::from_utf8(writer.bytes).map_err(|error| error.to_string())
}

fn preflight(refusal: FixedCargoRouteError) -> Result<ToolOutput, ToolError> {
    let report = serde_json::to_value(&refusal.error).map_err(|error| {
        ToolError::Internal(format!(
            "fixed Cargo preflight serialization: {error}; original={:?}",
            refusal.error
        ))
    })?;
    Err(ToolError::StructuredPreflight {
        report,
        message: refusal.error.message,
    })
}

fn diagnostic(refusal: FixedCargoRouteError) -> Result<ToolOutput, ToolError> {
    // The owning route refused a normal wire publication. Its original typed
    // evidence remains in this labelled diagnostic, without inventing a
    // completed result or asserting NoEffects from an Err.
    Ok(ToolOutput::executed_failure_text(format!(
        "UNVALIDATED fixed Cargo owner publication; may_effect={}; error={:?}; observation={:?}",
        refusal.effects_may_have_begun, refusal.error, refusal.owner_observation,
    )))
}

fn published(envelope: e::OperationEnvelope) -> Result<ToolOutput, ToolError> {
    match envelope.body {
        e::Body::Result(row) => {
            let reply: FixedCargoActionReply =
                serde_json::from_str(row.payload_json.as_deref().ok_or_else(|| {
                    ToolError::Internal("validated Cargo result lacks payload".into())
                })?)
                .map_err(|error| {
                    ToolError::Internal(format!("validated Cargo reply reader: {error}"))
                })?;
            let kind = match &reply.reply {
                vibe_wire::generated::machine::e1::fixed_cargo_action_reply::Reply::Completed(_) => "completed",
                vibe_wire::generated::machine::e1::fixed_cargo_action_reply::Reply::Uncertain(_) => "uncertain",
                vibe_wire::generated::machine::e1::fixed_cargo_action_reply::Reply::RecoveryRequired(_) => "recovery_required",
                vibe_wire::generated::machine::e1::fixed_cargo_action_reply::Reply::Retired(_) => "retired",
                vibe_wire::generated::machine::e1::fixed_cargo_action_reply::Reply::Absent(_) => "absent",
            };
            let report = serde_json::to_value(&reply).map_err(|error| {
                ToolError::Internal(format!(
                    "fixed Cargo reply serialization: {error}; original={reply:?}"
                ))
            })?;
            Ok(ToolOutput::executed(
                report,
                format!(
                    "Fixed Cargo owner observation: {kind}; inspect the typed reply for run and effect truth."
                ),
            ))
        }
        e::Body::Error(row) => {
            let error: FixedCargoActionError =
                serde_json::from_str(row.payload_json.as_deref().ok_or_else(|| {
                    ToolError::Internal("validated Cargo error lacks payload".into())
                })?)
                .map_err(|reason| {
                    ToolError::Internal(format!("validated Cargo error reader: {reason}"))
                })?;
            let report = serde_json::to_value(&error).map_err(|reason| {
                ToolError::Internal(format!(
                    "fixed Cargo error serialization: {reason}; original={error:?}"
                ))
            })?;
            let text = format!("Fixed Cargo action refused: {}", error.message);
            if row.effect_state == e::BodyErrorEffectState::NoEffects {
                Err(ToolError::StructuredPreflight {
                    report,
                    message: text,
                })
            } else {
                Ok(ToolOutput::executed_failure(report, text))
            }
        }
        _ => Err(ToolError::Internal(
            "fixed Cargo route published a non-result stage".into(),
        )),
    }
}

impl McpTool for FixedCargoOperationMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "fixed_cargo_operation".into(),
            description: "Submit or inspect only the explicitly host-granted fixed offline Cargo fixture. The request cannot select a root, namespace, provider, route, target or toolchain. Same-key calls attach to the incumbent owner result; completion may warn that ambient rustup is unproven.".into(),
            input_schema: json!({
                "type":"object", "properties":{
                    "schema":{"type":"integer","const":1},
                    "action":{"oneOf":[
                        {"type":"object","properties":{"kind":{"type":"string","const":"submit"},"key":{"type":"string"}},"required":["kind","key"],"additionalProperties":false},
                        {"type":"object","properties":{"kind":{"type":"string","const":"inspect"},"key":{"type":"string"}},"required":["kind","key"],"additionalProperties":false}
                    ]}
                },"required":["schema","action"],"additionalProperties":false
            }),
        }
    }

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let grant = ctx.fixed_cargo_bridge().map(|route| route.as_ref());
        let payload = match bounded(args, REQUEST_CAP) {
            Ok(payload) => payload,
            Err(reason) => return preflight(FixedCargoRouteError::invalid_request(&reason, grant)),
        };
        // Compile-time identities select the sole owner; caller bytes remain
        // only nested action data and are decoded again by that owner route.
        let input = json!({"schema":1,"operation_id":OPERATION_ID,"domain_owner":DOMAIN_OWNER,
            "source_id":null,"evidence_id":null,"body":{"stage":"input",
                "format_id":FormatId::MachineFixedCargoActionRequest.id(),"payload_json":payload}});
        let input = match bounded(&input, PUBLICATION_CAP) {
            Ok(input) => input,
            Err(reason) => return preflight(FixedCargoRouteError::invalid_request(&reason, grant)),
        };
        match FixedCargoRoute::exchange_local(grant, input.as_bytes(), &ctx.project_root) {
            Ok(envelope) => published(envelope),
            Err(refusal) => diagnostic(refusal),
        }
    }
}

#[cfg(test)]
mod tests;
