//! Host-issued target-binding syntax and carrier equality. These checks do
//! not admit roots, authenticate a principal or grant an effect.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-OPERATION-BINDING");

use std::collections::BTreeSet;

use crate::behaviour::scalars::{has_control_bytes, is_portable_token};
use crate::generated::deploy_intent::DeployIntent;
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
