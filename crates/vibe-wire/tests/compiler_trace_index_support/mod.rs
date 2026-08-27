//! Shared fixture machinery for the trace index's semantic reds, used
//! by `compiler_trace_index_validator.rs` (identity, timestamps, scopes,
//! events) and `compiler_trace_index_relational.rs` (snapshots, the
//! root's word, the timing table, the diagnostic cap). Not a test
//! binary of its own; it exists because the two halves would otherwise
//! exceed the 600-line file cap as one.
//!
//! Every red is one authored golden document mutated into exactly ONE
//! violation, so the family the validator names is the family the
//! mutation created — not a second, accidental one downstream.

#![allow(dead_code)]

use std::path::PathBuf;

use serde_json::Value;
use vibe_wire::behaviour::compiler_trace_index::{TraceIndexError, validate};
use vibe_wire::generated::compiler_trace_index::e1::index::CompilerTraceIndex;

/// One authored valid document from the registered corpus.
pub fn corpus(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_trace_index/e1/valid")
        .join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} readable: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name} parses: {e}"))
}

/// The complete `ok` run: two parse invocations, one whole-artifact
/// pass, three distinct snapshots, exact aggregates.
pub fn ok() -> Value {
    corpus("ok_complete.json")
}

/// The partial `failed` run: a prior success that keeps its snapshot,
/// then one `pass-failed` event that certifies nothing.
pub fn failed() -> Value {
    corpus("failed_partial.json")
}

/// The partial `running` index: a fingerprint-only skip, a pending
/// scope, and explicitly empty lists.
pub fn running() -> Value {
    corpus("running_skipped.json")
}

/// Parse a mutated document through the GENERATED reader, then run the
/// hand-written laws over it. A mutation that the reader itself refuses
/// is a reader test, not a semantic one, and fails loudly here.
pub fn check(doc: Value) -> Result<(), TraceIndexError> {
    let index: CompilerTraceIndex =
        serde_json::from_value(doc).expect("the mutated document still parses");
    validate(&index)
}

/// Delete one member, so an "absent where the law requires it" red is
/// spelled as a deletion rather than as a null.
pub fn remove(doc: &mut Value, pointer: &str, key: &str) {
    doc.pointer_mut(pointer)
        .and_then(|node| node.as_object_mut())
        .unwrap_or_else(|| panic!("pointer {pointer} names an object"))
        .remove(key);
}
