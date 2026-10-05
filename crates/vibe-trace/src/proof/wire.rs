use lean_ai_native_check::{FileBinding, ProofWitness, SpecBinding};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SCHEMA: &str = "vibevm/selected-proof-join/1";

/// Trusted startup context, never supplied by an MCP request.
#[derive(Clone, Debug)]
pub struct Context {
    pub project_root: PathBuf,
    pub lean_toolchain_dir: Option<PathBuf>,
    pub rust_toolchain_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Check { subject: String, witness: PathBuf },
    Verify { subject: String, witness: PathBuf },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
}
impl Diagnostic {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
pub(super) type Result<T> = std::result::Result<T, Diagnostic>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ProofResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Diagnostic>,
}
impl Reply {
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            result: None,
            error: Some(Diagnostic::new("invalid_request", message)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProofResult {
    pub status: String,
    pub witness_sha256: String,
    pub witness: JoinedWitness,
}

/// Every field is checked against a fresh execution. Execution IDs identify
/// selected contributions, not authenticated historical process records.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JoinedWitness {
    pub schema: String,
    pub subject: String,
    pub grade: String,
    pub correspondence: String,
    pub spec: SpecBinding,
    pub consumer: FileBinding,
    pub selection: FileBinding,
    pub policy: FileBinding,
    pub map_sha256: String,
    pub checker_source_sha256: String,
    pub rust: RustWitness,
    pub lean: LeanWitness,
    pub assumptions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RustWitness {
    pub execution_id: String,
    pub checker: String,
    pub outcome: String,
    pub symbol: String,
    pub inputs: Vec<FileBinding>,
    pub dependency_policy: String,
    pub target: String,
    pub tests: Vec<TestOutcome>,
    pub toolchain: RustToolchain,
    pub executable_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TestOutcome {
    pub name: String,
    pub outcome: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RustToolchain {
    pub rustc_version: String,
    pub cargo_version: String,
    pub bin_lib_sha256: String,
    pub host_linker_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LeanWitness {
    pub execution_id: String,
    pub checker: String,
    pub outcome: String,
    pub proof: ProofWitness,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Policy {
    pub schema: u32,
    pub subject: String,
    pub grade: String,
    pub rust: RustPolicy,
    pub lean: LeanPolicy,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RustPolicy {
    pub execution_id: String,
    pub manifest: String,
    pub sources: Vec<String>,
    pub tests: Vec<String>,
    pub target: String,
    pub rustc_release: String,
    pub rustc_commit: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LeanPolicy {
    pub execution_id: String,
}
