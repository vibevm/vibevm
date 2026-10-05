//! Thin CLI projection of the shared uncached proof engine.
//! Output is the engine Reply itself in every output mode; quiet selects
//! compact JSON. Neither lifecycle freshness nor log parsing supplies proof.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#VERIFY-EVIDENCE-IDENTITY");

use anyhow::{Result, bail};
use vibe_trace::proof::{Context as ProofContext, Request, execute};

use crate::cli::ProofArgs;
use crate::output::Context;

/// The CLI command chooses the operation; the engine owns its semantics.
#[derive(Clone, Copy)]
pub enum Operation {
    Check,
    Verify,
}

/// Execute once and retain the exact typed Reply on stdout, including failure.
pub fn run(context: &Context, args: ProofArgs, operation: Operation) -> Result<()> {
    let engine = ProofContext {
        project_root: args.path,
        lean_toolchain_dir: args.lean_toolchain_dir,
        rust_toolchain_dir: args.rust_toolchain_dir,
    };
    let request = match operation {
        Operation::Check => Request::Check {
            subject: args.subject,
            witness: args.witness,
        },
        Operation::Verify => Request::Verify {
            subject: args.subject,
            witness: args.witness,
        },
    };
    let reply = execute(&engine, request);
    let output = if context.is_quiet() {
        serde_json::to_string(&reply)?
    } else {
        serde_json::to_string_pretty(&reply)?
    };
    println!("{output}");
    if !reply.ok {
        if let Some(error) = reply.error {
            bail!("{}: {}", error.code, error.message);
        }
        bail!("selected proof operation failed without a diagnostic");
    }
    Ok(())
}

#[cfg(test)]
#[path = "proof_tests.rs"]
mod tests;
