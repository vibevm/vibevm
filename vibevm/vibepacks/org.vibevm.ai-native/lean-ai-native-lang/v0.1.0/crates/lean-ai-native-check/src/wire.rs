use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const TOOLCHAIN: &str = "leanprover/lean4:v4.34.1";
pub const LEAN_COMMIT: &str = "5045d0056413266e57c625dcd7c365b10e377c52";
pub const WITNESS_SCHEMA: &str = "lean-ai-native/proof-witness/1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Scan {
        carrier: PathBuf,
    },
    Check {
        carrier: PathBuf,
        subject: String,
    },
    VerifyWitness {
        carrier: PathBuf,
        subject: String,
        witness: PathBuf,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
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
    pub(crate) fn serialization(error: serde_json::Error) -> Self {
        Self::new("serialization_failed", error.to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileBinding {
    pub path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpecBinding {
    pub file: String,
    pub file_sha256: String,
    pub unit_hash: String,
    pub revision: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolchainBinding {
    pub identifier: String,
    pub version: String,
    pub commit: String,
    pub bin_lib_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProofWitness {
    pub schema: String,
    pub grade: String,
    pub outcome: String,
    pub subject: String,
    pub spec: SpecBinding,
    pub module: String,
    pub theorem: String,
    pub statement: String,
    pub statement_sha256: String,
    pub carrier_sha256: String,
    pub sources: Vec<FileBinding>,
    pub compiled_modules: Vec<FileBinding>,
    pub allowed_axioms: Vec<String>,
    pub observed_axioms: Vec<String>,
    pub toolchain: ToolchainBinding,
    pub runner_version: String,
    pub checker_source_sha256: String,
    pub inspector_sha256: String,
    pub assurance: String,
}
