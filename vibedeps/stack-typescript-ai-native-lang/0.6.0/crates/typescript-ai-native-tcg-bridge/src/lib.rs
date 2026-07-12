//! typescript-ai-native-tcg-bridge — the Rust side of the agentic type oracle
//! (TCG-PROTOCOL-v0.1 / TCG-ORACLE-v0.1): the embedded oracle source and
//! its content-addressed materialisation, the protocol's message types,
//! and a persistent correlation-id transport over a spawned node child.
//!
//! The fact/marker vocabulary is `typescript-ai-native-extract-bridge`'s VERBATIM
//! (`RawFact` / `RawMarker`) — one serde vocabulary serves both tools,
//! so `conform_facts` lowers oracle responses exactly like extractor
//! records (TCG-PROTOCOL §2).
//!
//! Transport tests replay recorded response lines with no node; the
//! spawn path is exercised by the CLI crate's node-gated end-to-end
//! test (the extract-bridge testing split, kept).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use typescript_ai_native_extract_bridge::{FileRecord, RawFact, RawMarker};

mod transport;
pub use transport::{OracleTransport, SystemOracle};

/// The oracle wire protocol this bridge speaks
/// (TCG-PROTOCOL-v0.1 §1; independent of the extractor's protocol).
pub const ORACLE_PROTOCOL: u64 = 1;

/// The oracle source, embedded at compile time from the package's
/// `tools/ts-oracle/oracle.ts` (TCG-ORACLE §1: one self-contained file).
pub const ORACLE_SOURCE: &str = include_str!("../../../tools/ts-oracle/oracle.ts");

/// Strip Windows' verbatim prefix (`\\?\`): node cannot load an entry
/// script through it, and `canonicalize()` adds it — the same lesson
/// PROP-019's `derive_self` and the junction helpers learned. A no-op
/// on non-verbatim paths and non-Windows.
pub fn verbatim_free(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(stripped) => PathBuf::from(stripped),
        None => path.to_path_buf(),
    }
}

/// Materialise the embedded oracle content-addressed under
/// `<root>/target/tcg/ts-oracle/oracle-<hash16>.ts`; idempotent, and a
/// source change lands at a new path (stale copies are inert).
pub fn materialise_oracle(project_root: &Path) -> Result<PathBuf, TcgBridgeError> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(ORACLE_SOURCE.as_bytes());
    let hash = hasher.finalize();
    let short: String = hash[..8].iter().map(|b| format!("{b:02x}")).collect();
    let dir = project_root.join("target").join("tcg").join("ts-oracle");
    let path = dir.join(format!("oracle-{short}.ts"));
    if !path.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| TcgBridgeError::Io {
            what: format!("creating {}", dir.display()),
            source: e,
        })?;
        std::fs::write(&path, ORACLE_SOURCE).map_err(|e| TcgBridgeError::Io {
            what: format!("writing {}", path.display()),
            source: e,
        })?;
    }
    Ok(path)
}

/// The five-way taxonomy of TCG-PROTOCOL §4, plus the local I/O class.
/// Every variant carries the violated REQ and a fix surface.
#[derive(Debug, thiserror::Error)]
pub enum TcgBridgeError {
    #[error(
        "node is not spawnable: {source} \
         (violates spec://org.vibevm.ai-native/typescript-ai-native-lang/mechanisms/TCG-PROTOCOL#errors; \
         fix: install node >= 22.6 and put it on PATH)"
    )]
    NodeMissing { source: std::io::Error },

    #[error(
        "the project's `typescript` install is unresolvable: {detail} \
         (violates spec://org.vibevm.ai-native/typescript-ai-native-lang/mechanisms/TCG-ORACLE#compiler; \
         fix: `npm install -D typescript` in the project root)"
    )]
    TypescriptUnresolvable { detail: String },

    #[error(
        "the oracle process died mid-session: {detail} \
         (violates spec://org.vibevm.ai-native/typescript-ai-native-lang/mechanisms/TCG-PROTOCOL#errors; \
         fix: re-init the session; if it repeats, run the op one-shot via \
         `typescript-ai-native-tcg validate ...` to see the child's stderr)"
    )]
    OracleCrashed { detail: String },

    #[error(
        "protocol violation: {detail} \
         (violates spec://org.vibevm.ai-native/typescript-ai-native-lang/mechanisms/TCG-PROTOCOL#framing; \
         fix: rebuild the slot binary so bridge and oracle share one \
         ORACLE_PROTOCOL)"
    )]
    Protocol { detail: String },

    #[error(
        "the oracle did not answer `{op}` within {budget_ms} ms \
         (violates spec://org.vibevm.ai-native/typescript-ai-native-lang/mechanisms/TCG-PROTOCOL#errors; \
         fix: raise the timeout for cold init, or check the child's stderr \
         log lines)"
    )]
    Timeout { op: String, budget_ms: u64 },

    #[error("oracle bridge i/o: {what}: {source}")]
    Io {
        what: String,
        source: std::io::Error,
    },
}

// ---------------------------------------------------------------------------
// Protocol result shapes (TCG-PROTOCOL §2)
// ---------------------------------------------------------------------------

/// A compiler diagnostic surfaced by `validate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: u64,
    pub category: String,
    pub message: String,
    pub line: u64,
    pub character: u64,
}

/// `init` result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitResult {
    pub ts_version: String,
    pub config_file: String,
    pub root_files: u64,
}

/// `validate` result — facts/markers in the extractor's vocabulary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResult {
    pub diagnostics: Vec<Diagnostic>,
    pub facts: Vec<RawFact>,
    pub markers: Vec<RawMarker>,
    pub degraded: bool,
}

/// One in-scope symbol from `scope`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub type_text: String,
}

/// A branded type found at a seam (heuristic-labelled — ORACLE §4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandedType {
    pub name: String,
    pub seam: String,
    pub heuristic: bool,
}

/// `scope` result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeResult {
    pub symbols: Vec<SymbolInfo>,
    pub cell: Option<String>,
    pub seam_file: Option<String>,
    pub branded: Vec<BrandedType>,
}

/// One completion entry (`unsafe` per the §8 ban set).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionEntry {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub type_text: String,
    #[serde(rename = "unsafe")]
    pub unsafe_: bool,
}

/// `complete` result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteResult {
    pub entries: Vec<CompletionEntry>,
}

/// `type` (quick info) result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeResult {
    pub display: String,
    pub documentation: String,
}

/// A position in the protocol's convention (1-based line, 0-based char).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub line: u64,
    pub character: u64,
}

/// The op-grain error body of an `{ok: false}` response.
#[derive(Debug, Clone, Deserialize)]
pub struct WireError {
    pub kind: String,
    pub detail: String,
    #[serde(default)]
    pub recipe: Option<String>,
}

/// One response frame, as parsed off the wire.
#[derive(Debug, Deserialize)]
pub struct ResponseFrame {
    pub proto: u64,
    pub id: Option<u64>,
    pub ok: bool,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<WireError>,
}

/// Parse one NDJSON response line into a frame, enforcing the protocol
/// version (a mismatch is the bridge's own error class — §5).
pub fn parse_response_line(line: &str) -> Result<ResponseFrame, TcgBridgeError> {
    let frame: ResponseFrame =
        serde_json::from_str(line).map_err(|e| TcgBridgeError::Protocol {
            detail: format!("unparseable response line: {e}: `{line}`"),
        })?;
    if frame.proto != ORACLE_PROTOCOL {
        return Err(TcgBridgeError::Protocol {
            detail: format!(
                "oracle speaks proto {}, bridge {}",
                frame.proto, ORACLE_PROTOCOL
            ),
        });
    }
    Ok(frame)
}

/// Map an `{ok:false}` wire error into the typed taxonomy.
pub fn error_from_wire(e: WireError) -> TcgBridgeError {
    let detail = match &e.recipe {
        Some(r) => format!("{} (oracle recipe: {r})", e.detail),
        None => e.detail.clone(),
    };
    match e.kind.as_str() {
        "typescript-unresolvable" => TcgBridgeError::TypescriptUnresolvable { detail },
        "protocol" => TcgBridgeError::Protocol { detail },
        "oracle-crashed" => TcgBridgeError::OracleCrashed { detail },
        _ => TcgBridgeError::Protocol {
            detail: format!("unknown wire error kind `{}`: {detail}", e.kind),
        },
    }
}

/// Assemble the extractor-shaped `FileRecord` for a validated file so
/// `typescript_ai_native_extract_bridge::conform_facts` lowers oracle answers exactly like
/// extractor records. `in_test` uses the extractor's file-name
/// convention (the record field is file-grain there too).
pub fn to_file_record(file: &str, v: &ValidateResult) -> FileRecord {
    let in_test = file.contains(".test.") || file.contains(".spec.") || file.contains("__tests__");
    FileRecord {
        protocol: typescript_ai_native_extract_bridge::PROTOCOL,
        file: file.to_string(),
        in_test,
        degraded: v.degraded,
        facts: v.facts.clone(),
        markers: v.markers.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALIDATE_LINE: &str = r#"{"proto":1,"id":7,"ok":true,"result":{"diagnostics":[{"code":2322,"category":"error","message":"Type 'string' is not assignable to type 'number'.","line":41,"character":6}],"facts":[{"fact":"file_metrics","lines":40},{"fact":"ts_unsafe","kind":"as_cross","line":33,"reason":null},{"fact":"item","kind":"function","symbol":"greet","line":37,"is_exported":true,"has_doc_example":false}],"markers":[{"tag":"implements","uri":"spec://fixture/PROP-001#req-greet","reason":null,"symbol":"greet","line":36}],"degraded":false}}"#;

    #[test]
    fn replay_validate_response_parses_into_typed_result() {
        let frame = parse_response_line(VALIDATE_LINE).expect("frame");
        assert!(frame.ok);
        assert_eq!(frame.id, Some(7));
        let v: ValidateResult =
            serde_json::from_value(frame.result.expect("result")).expect("shape");
        assert_eq!(v.diagnostics.len(), 1);
        assert_eq!(v.diagnostics[0].code, 2322);
        assert_eq!(v.facts.len(), 3);
        assert_eq!(v.markers.len(), 1);
        assert!(!v.degraded);
    }

    #[test]
    fn replay_error_response_maps_to_the_taxonomy() {
        let line = r#"{"proto":1,"id":3,"ok":false,"error":{"kind":"typescript-unresolvable","detail":"no typescript","recipe":"npm install -D typescript"}}"#;
        let frame = parse_response_line(line).expect("frame");
        assert!(!frame.ok);
        let err = error_from_wire(frame.error.expect("error"));
        let msg = err.to_string();
        assert!(msg.contains("npm install -D typescript"), "{msg}");
        assert!(msg.contains("TCG-ORACLE#compiler"), "{msg}");
    }

    #[test]
    fn proto_mismatch_is_a_protocol_error() {
        let line = r#"{"proto":99,"id":1,"ok":true}"#;
        let err = parse_response_line(line).expect_err("mismatch");
        assert!(matches!(err, TcgBridgeError::Protocol { .. }));
    }

    #[test]
    fn unparseable_line_is_a_protocol_error_naming_the_line() {
        let err = parse_response_line("not json").expect_err("bad line");
        assert!(err.to_string().contains("not json"));
    }

    #[test]
    fn file_record_assembly_matches_the_extractor_vocabulary() {
        let frame = parse_response_line(VALIDATE_LINE).expect("frame");
        let v: ValidateResult =
            serde_json::from_value(frame.result.expect("result")).expect("shape");
        let record = to_file_record("src/cells/greet/index.ts", &v);
        assert!(!record.in_test);
        let facts = typescript_ai_native_extract_bridge::conform_facts(&record);
        // file_metrics + as_cross + item lower to three engine facts
        assert_eq!(facts.len(), 3);
        let test_record = to_file_record("src/cells/greet/index.test.ts", &v);
        assert!(test_record.in_test);
    }

    #[test]
    fn materialise_is_idempotent_and_content_addressed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = materialise_oracle(dir.path()).expect("first");
        let second = materialise_oracle(dir.path()).expect("second");
        assert_eq!(first, second);
        let text = std::fs::read_to_string(&first).expect("read");
        assert_eq!(text, ORACLE_SOURCE);
        assert!(
            first
                .file_name()
                .and_then(|n| n.to_str())
                .expect("name")
                .starts_with("oracle-")
        );
    }
}
