//! Historical inspection does not reconstruct executable current compiler state.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-EMBED-PROVENANCE-EPOCH");

use vibe_wire::generated::compiler_ir::e1::ir::Ir;

/// A structural/token/epoch refusal while inspecting a historical snapshot.
#[derive(Debug, thiserror::Error)]
pub enum HistoricalCompilerIrError {
    #[error(
        "historical compiler IR cannot be inspected: {detail}; retain the original snapshot and inspect it with its declared epoch"
    )]
    Read { detail: String },
    #[error(
        "historical compiler IR inspection requires schema 1, got {found}; use the current compiler owner for current execution"
    )]
    Epoch { found: u32 },
}

/// Read the exact historical epoch-1 typed payload through the existing strict
/// duplicate-key token walk. This validates structural shape and epoch only;
/// it does not replay old derived link/digest laws, invent generated provenance,
/// or grant current execution authority. Current execution accepts only IR2.
///
/// ```
/// let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
///     "/../../formats/corpora/compiler_ir/e1/valid/source_document.json"));
/// let historical = vibe_spec::inspect_historical_compiler_ir(bytes).unwrap();
/// assert!(matches!(historical,
///     vibe_wire::generated::compiler_ir::e1::ir::Ir::SourceDocument(_)));
/// ```
pub fn inspect_historical_compiler_ir(bytes: &[u8]) -> Result<Ir, HistoricalCompilerIrError> {
    let value: Ir =
        super::json::from_strict_slice(bytes).map_err(|error| HistoricalCompilerIrError::Read {
            detail: error.to_string(),
        })?;
    let found = match &value {
        Ir::SourceDocument(value) => value.ir_schema,
        Ir::DocumentDocument(value) => value.ir_schema,
        Ir::DocumentsArtifact(value) => value.ir_schema,
        Ir::ClosureArtifact(value) => value.ir_schema,
        Ir::LaneArtifact(value) => value.ir_schema,
        Ir::EmittedArtifact(value) => value.ir_schema,
    };
    if found != 1 {
        return Err(HistoricalCompilerIrError::Epoch { found });
    }
    Ok(value)
}

#[cfg(test)]
#[path = "tests/history.rs"]
mod tests;
