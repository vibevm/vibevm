//! Host-issued target-binding syntax and carrier equality. These checks do
//! not admit roots, authenticate a principal or grant an effect.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-OPERATION-BINDING");

use std::collections::BTreeSet;

use crate::behaviour::scalars::{has_control_bytes, is_portable_token};
use crate::generated::deploy_checkpoints::DeployCheckpoints;
use crate::generated::deploy_intent::DeployIntent;
use crate::generated::deploy_inverse::DeployInverse;
use crate::generated::deploy_lock_resources::{DeployLockResources, LockBinding};
use crate::generated::deploy_receipt::DeployReceipt;
use crate::generated::environment::e1::operation_binding::OperationBinding;
use crate::generated::native::e1::deploy_request::NativeOperationBinding;
use crate::generated::shared::EnvironmentOperationBinding;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationBindingError {
    pub field: &'static str,
    pub reason: &'static str,
}
impl std::fmt::Display for OperationBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "operation binding {}: {}", self.field, self.reason)
    }
}
impl std::error::Error for OperationBindingError {}
fn invalid(field: &'static str, reason: &'static str) -> OperationBindingError {
    OperationBindingError { field, reason }
}

fn marker_binding(
    field: &'static str,
    observed: Option<&EnvironmentOperationBinding>,
    expected: Option<&EnvironmentOperationBinding>,
) -> Result<(), OperationBindingError> {
    if let Some(value) = observed {
        validate_operation_binding(value)?;
    }
    if let Some(value) = expected {
        validate_operation_binding(value)?;
    }
    if observed != expected {
        return Err(invalid(
            field,
            "durable binding differs from the host or intent",
        ));
    }
    Ok(())
}
fn text(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && !has_control_bytes(value) && value.len() <= 512
}

/// Validate one decoded strict binding without promoting its references to
/// physical authority. Every nullable member is required in the JTD form.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-OPERATION-BINDING",
    r = 3
)]
pub fn validate_operation_binding(
    binding: &EnvironmentOperationBinding,
) -> Result<(), OperationBindingError> {
    if binding.binding_epoch != 1 {
        return Err(invalid("binding_epoch", "unsupported binding epoch"));
    }
    if binding.admission_epoch == 0 {
        return Err(invalid(
            "admission_epoch",
            "admission epoch must be positive",
        ));
    }
    for (field, value) in [
        ("binding_id", &binding.binding_id),
        ("host_context_id", &binding.host_context_id),
        ("instance_ref", &binding.instance_ref),
        ("target_architecture", &binding.target_architecture),
        ("principal_ref", &binding.principal_ref),
        ("storage_binding_ref", &binding.storage_binding_ref),
    ] {
        if !text(value) {
            return Err(invalid(field, "reference is blank, unsafe or too long"));
        }
    }
    for (field, value) in [
        (
            "selected_generation_ref",
            binding.selected_generation_ref.as_deref(),
        ),
        ("target_abi", binding.target_abi.as_deref()),
        ("target_libc", binding.target_libc.as_deref()),
    ] {
        if value.is_some_and(|value| !text(value)) {
            return Err(invalid(
                field,
                "optional reference is blank, unsafe or too long",
            ));
        }
    }
    if binding.root_bindings.is_empty() {
        return Err(invalid(
            "root_bindings",
            "at least one bound root is required",
        ));
    }
    let mut roles = BTreeSet::new();
    for root in &binding.root_bindings {
        if !is_portable_token(&root.root_role) || !roles.insert(root.root_role.as_str()) {
            return Err(invalid(
                "root_bindings.root_role",
                "root role is malformed or repeated",
            ));
        }
        if !text(&root.capability_ref) {
            return Err(invalid(
                "root_bindings.capability_ref",
                "root capability reference is blank, unsafe or too long",
            ));
        }
    }
    Ok(())
}

/// The standalone registered root and the two durable records use one strict
/// generated binding type. The outer root carries its own epoch.
pub fn validate_operation_binding_root(
    root: &OperationBinding,
) -> Result<(), OperationBindingError> {
    if root.schema != 1 {
        return Err(invalid("schema", "unsupported root epoch"));
    }
    validate_operation_binding(&root.binding)
}

/// Equality before replay: a bound journal cannot settle against a legacy or
/// differently bound receipt. This checks values, not live root handles.
pub fn validate_operation_binding_pair(
    intent: &DeployIntent,
    receipt: &DeployReceipt,
) -> Result<(), OperationBindingError> {
    if let Some(value) = &intent.operation_binding {
        validate_operation_binding(value)?;
    }
    if let Some(value) = &receipt.operation_binding {
        validate_operation_binding(value)?;
    }
    if intent.operation_binding != receipt.operation_binding {
        return Err(invalid("carrier", "intent and receipt binding differ"));
    }
    Ok(())
}

/// A copied or legacy physical lock set cannot satisfy a bound intent. Both
/// slots are checked independently because an interrupted update can retain
/// committed and pending generations at once.
pub fn validate_lock_resources_binding(
    record: &DeployLockResources,
    expected: Option<&EnvironmentOperationBinding>,
) -> Result<(), OperationBindingError> {
    if record.schema != 1 {
        return Err(invalid(
            "lock_resources.schema",
            "unsupported sidecar epoch",
        ));
    }
    if expected.is_some() && record.committed.is_none() && record.pending.is_none() {
        return Err(invalid(
            "lock_resources",
            "bound sidecar has no attributed generation",
        ));
    }
    if let Some(committed) = &record.committed {
        validate_lock_binding(committed, expected, "lock_resources.committed")?;
    }
    if let Some(pending) = &record.pending {
        validate_lock_binding(pending, expected, "lock_resources.pending")?;
    }
    Ok(())
}

fn validate_lock_binding(
    record: &LockBinding,
    expected: Option<&EnvironmentOperationBinding>,
    field: &'static str,
) -> Result<(), OperationBindingError> {
    marker_binding(field, record.operation_binding.as_ref(), expected)
}

/// Checkpoint completion is attributable only to the exact bound plan and
/// host/intent binding. It is not proof that a provider replay is safe.
pub fn validate_checkpoints_binding(
    record: &DeployCheckpoints,
    expected: Option<&EnvironmentOperationBinding>,
    plan_hash: &str,
) -> Result<(), OperationBindingError> {
    if record.schema != 1 {
        return Err(invalid(
            "checkpoints.schema",
            "unsupported checkpoint epoch",
        ));
    }
    if record.plan_hash != plan_hash {
        return Err(invalid(
            "checkpoints.plan_hash",
            "checkpoint belongs to another plan",
        ));
    }
    marker_binding(
        "checkpoints.operation_binding",
        record.operation_binding.as_ref(),
        expected,
    )
}

/// Inverse markers have no plan hash in epoch one; exact binding, generation
/// and provider pin are therefore mandatory joins before any bound reversal.
pub fn validate_inverse_binding(
    record: &DeployInverse,
    expected: Option<&EnvironmentOperationBinding>,
    generation: u32,
    provider_pin: &str,
) -> Result<(), OperationBindingError> {
    if record.schema != 1 {
        return Err(invalid("inverse.schema", "unsupported inverse epoch"));
    }
    if record.generation != generation || record.provider_pin != provider_pin {
        return Err(invalid(
            "inverse.identity",
            "inverse belongs to another generation or provider",
        ));
    }
    marker_binding(
        "inverse.operation_binding",
        record.operation_binding.as_ref(),
        expected,
    )
}

/// Native requests use a permissive generated copy under their foreign-parser
/// contract. Compare every known field to the strict host-owned binding;
/// unknown native members never become authority.
pub fn validate_native_operation_binding(
    native: &NativeOperationBinding,
    host: &EnvironmentOperationBinding,
) -> Result<(), OperationBindingError> {
    validate_operation_binding(host)?;
    let native = serde_json::to_value(native)
        .map_err(|_| invalid("native", "native binding cannot be encoded"))?;
    let host = serde_json::to_value(host)
        .map_err(|_| invalid("native", "host binding cannot be encoded"))?;
    if native != host {
        return Err(invalid(
            "native",
            "native request differs from host binding",
        ));
    }
    Ok(())
}
