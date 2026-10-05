//! Strict per-tool arguments. The operation is chosen by the registered
//! tool, never accepted from an untrusted argument object.

specmark::scope!("spec://org.vibevm.ai-native/lean-ai-native-mcp/tools/lean-mcp#arguments");

use std::path::PathBuf;

use lean_ai_native_check::Request;
use serde::Deserialize;
use serde_json::Value;

use super::Operation;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scan {
    carrier: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Check {
    carrier: PathBuf,
    subject: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifyWitness {
    carrier: PathBuf,
    subject: String,
    witness: PathBuf,
}

pub(super) fn parse(operation: Operation, args: &Value) -> Result<Request, serde_json::Error> {
    if !args.is_object() {
        return Err(<serde_json::Error as serde::de::Error>::custom(
            "tool arguments must be a JSON object",
        ));
    }
    match operation {
        Operation::Scan => {
            let Scan { carrier } = serde_json::from_value(args.clone())?;
            Ok(Request::Scan { carrier })
        }
        Operation::Check => {
            let Check { carrier, subject } = serde_json::from_value(args.clone())?;
            Ok(Request::Check { carrier, subject })
        }
        Operation::VerifyWitness => {
            let VerifyWitness {
                carrier,
                subject,
                witness,
            } = serde_json::from_value(args.clone())?;
            Ok(Request::VerifyWitness {
                carrier,
                subject,
                witness,
            })
        }
    }
}
