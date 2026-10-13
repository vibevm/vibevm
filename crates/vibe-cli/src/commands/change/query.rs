//! Thin read-only adapter to the same M16 change service used by MCP.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-READ-SERVICE");
use clap::Args;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};
use vibe_wire::behaviour::change_query as wire;
#[derive(Debug, Args)]
pub struct ReadArgs {
    pub id: Option<String>,
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
    #[arg(long)]
    pub spec_root: Option<PathBuf>,
    #[arg(long)]
    pub role: Vec<String>,
    #[arg(long)]
    pub request_json: Option<String>,
}
pub fn execute(root: &Path, spec_root: Option<&Path>, request: &Value) -> wire::Response {
    vibe_orchestrator::machine_operation::MachineOperationService::read_change_json(
        root, spec_root, request,
    )
}
pub fn run(args: &ReadArgs, mode: &str, json_output: bool) -> ExitCode {
    let value = if let Some(raw) = &args.request_json {
        if args.id.is_some() || !args.role.is_empty() {
            return publish(
                wire::refused(
                    wire::ResponseRefusedCode::InvalidRequest,
                    "--request-json cannot mix id/role arguments",
                    vec![],
                ),
                json_output,
            );
        }
        match serde_json::from_str::<Value>(raw) {
            Ok(v) => v,
            Err(_) => {
                return publish(
                    wire::refused(
                        wire::ResponseRefusedCode::InvalidRequest,
                        "request JSON syntax",
                        vec![],
                    ),
                    json_output,
                );
            }
        }
    } else {
        json!({"schema":1,"mode":mode,"id":args.id.as_deref().unwrap_or(""),"roles":args.role})
    };
    let request = match wire::read_request(&value) {
        Ok(r) => r,
        Err(r) => return publish(r, json_output),
    };
    if serde_json::to_value(&request.mode).unwrap().as_str() != Some(mode) {
        return publish(
            wire::refused(
                wire::ResponseRefusedCode::InvalidRequest,
                "request mode differs from CLI subcommand",
                vec![mode.into()],
            ),
            json_output,
        );
    }
    if let Err(error) = vibe_core::manifest::change::ChangeId::parse(&request.id) {
        return publish(
            wire::refused(
                wire::ResponseRefusedCode::InvalidId,
                error.to_string(),
                vec![],
            ),
            json_output,
        );
    }
    let root = match super::super::resolve_project_root(&args.path) {
        Ok(root) => root,
        Err(error) => {
            return publish(
                wire::refused(
                    wire::ResponseRefusedCode::UnavailableContext,
                    error.to_string(),
                    vec![],
                ),
                json_output,
            );
        }
    };
    publish(
        execute(&root, args.spec_root.as_deref(), &value),
        json_output,
    )
}
fn publish(response: wire::Response, json_output: bool) -> ExitCode {
    if let Err(error) = wire::validate_response(&response) {
        eprintln!("vibe: invalid change response: {error}");
        return ExitCode::from(1);
    }
    let raw = serde_json::to_value(&response).unwrap();
    if wire::read_response(&raw).is_err() {
        eprintln!("vibe: change response decoding invariant");
        return ExitCode::from(1);
    }
    if json_output {
        println!("{}", serde_json::to_string(&response).unwrap());
    } else {
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    }
    if matches!(response, wire::Response::Refused(_)) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
#[cfg(test)]
mod m_02_d_2_cases;
