//! Authored operation-schema reference only; selected-world registry and source
//! authentication remain separate from this pure manifest grammar.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#EXTENSION-OPERATION-SCHEMA");

use specmark::spec;

const LAW: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#EXTENSION-OPERATION-SCHEMA";

/// One optional strict authored table beneath a `[[extension]]` declaration.
/// This value carries no provider, source hash, availability or write grant.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EXTENSION-OPERATION-SCHEMA",
    r = 1
)]
pub struct MachineOperationSchemaDecl {
    pub schema_epoch: u32,
    pub operation_id: Option<String>,
    pub model_id: Option<String>,
    pub input_format_id: Option<String>,
    pub result_format_id: Option<String>,
    pub error_format_id: Option<String>,
    pub event_format_id: Option<String>,
}

fn name_is_namespaced(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !value.chars().any(|c| c.is_whitespace() || c.is_control())
        && (value.contains(':') || value.contains('.'))
}
fn reference_is_nonblank(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !value.chars().any(|c| c.is_whitespace() || c.is_control())
}
impl MachineOperationSchemaDecl {
    /// Local syntax only. The selected-world owner must check registered
    /// FormatId/epoch, provider/source joins and cross-provider collisions.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_epoch != 1 {
            return Err(format!(
                "field `machine_operation_schema.schema_epoch` must be exactly 1 ({LAW})"
            ));
        }
        if self.operation_id.is_none() && self.model_id.is_none() {
            return Err(format!(
                "fields `machine_operation_schema.operation_id` and `model_id` are both absent; at least one is required ({LAW})"
            ));
        }
        for (name, value) in [
            ("operation_id", self.operation_id.as_deref()),
            ("model_id", self.model_id.as_deref()),
        ] {
            if value.is_some_and(|v| !name_is_namespaced(v)) {
                return Err(format!(
                    "field `machine_operation_schema.{name}` must be a nonblank namespaced identity without whitespace ({LAW})"
                ));
            }
        }
        let formats = [
            ("input_format_id", self.input_format_id.as_deref()),
            ("result_format_id", self.result_format_id.as_deref()),
            ("error_format_id", self.error_format_id.as_deref()),
            ("event_format_id", self.event_format_id.as_deref()),
        ];
        if formats.iter().all(|(_, value)| value.is_none()) {
            return Err(format!(
                "machine_operation_schema requires at least one format ID reference ({LAW})"
            ));
        }
        for (name, value) in formats {
            if value.is_some_and(|v| !reference_is_nonblank(v)) {
                return Err(format!(
                    "field `machine_operation_schema.{name}` must be a nonblank format ID without whitespace ({LAW})"
                ));
            }
        }
        Ok(())
    }
    /// No FormatId lookup belongs here: a syntactically valid string is not
    /// evidence that a host-owned registered generated reader exists.
    pub fn format_ids(&self) -> impl Iterator<Item = &str> {
        [
            self.input_format_id.as_deref(),
            self.result_format_id.as_deref(),
            self.error_format_id.as_deref(),
            self.event_format_id.as_deref(),
        ]
        .into_iter()
        .flatten()
    }
}
