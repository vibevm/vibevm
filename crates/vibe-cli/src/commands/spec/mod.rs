//! Early read-only source-query dispatch; no registry/progress/effect setup.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#GENERATED-SOURCE-QUERY-CONTRACT");
use clap::{Args, Subcommand, ValueEnum};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::ExitCode;
use vibe_workspace::source_query::SourceQueryService;

#[derive(Debug, Args)]
pub struct SpecArgs {
    #[command(subcommand)]
    pub command: SpecCommand,
}
#[derive(Debug, Subcommand)]
pub enum SpecCommand {
    Read(ReadArgs),
    Resolve(ReadArgs),
}
#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Md,
    Xml,
}
#[derive(Debug, Args)]
pub struct ReadArgs {
    pub address: Option<String>,
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
    #[arg(long)]
    pub request_json: Option<String>,
    #[arg(
        long,
        value_enum,
        default_value = "md",
        conflicts_with = "request_json"
    )]
    pub format: OutputFormat,
    #[arg(long)]
    pub no_children: bool,
    #[arg(long)]
    pub outline: bool,
    #[arg(long)]
    pub cursor: Option<String>,
    #[arg(long)]
    pub chunk_bytes: Option<u32>,
    #[arg(long)]
    pub expected_resolution_hash: Option<String>,
    #[arg(long)]
    pub expected_source_hash: Option<String>,
    #[arg(long)]
    pub expected_document_hash: Option<String>,
}

pub fn run(args: &SpecArgs, json_output: bool) -> ExitCode {
    let (args, resolve) = match &args.command {
        SpecCommand::Read(args) => (args, false),
        SpecCommand::Resolve(args) => (args, true),
    };
    let request = match request(args, resolve) {
        Ok(request) => request,
        Err(message) => {
            let error = json!({"schema":1,"code":"invalid_request","message":message,"boundary":"CLI request","candidates":[]});
            if json_output {
                println!("{error}")
            } else {
                eprintln!("vibe: {message}");
            }
            return ExitCode::from(1);
        }
    };
    let response = SourceQueryService::execute_json(&args.path, &request);
    let failed = response.is_error();
    let value = response.into_value();
    if json_output {
        println!("{value}");
    } else if failed {
        eprintln!(
            "vibe: {}",
            value["message"].as_str().unwrap_or("Source query refused")
        );
    } else if let Some(content) = value["content"]
        .as_str()
        .filter(|content| !content.is_empty())
    {
        print!("{content}");
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&value).expect("validated source result")
        );
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn request(args: &ReadArgs, resolve: bool) -> Result<Value, &'static str> {
    if let Some(raw) = &args.request_json {
        if args.address.is_some()
            || args.outline
            || args.no_children
            || args.cursor.is_some()
            || args.chunk_bytes.is_some()
            || args.expected_resolution_hash.is_some()
            || args.expected_source_hash.is_some()
            || args.expected_document_hash.is_some()
        {
            return Err("--request-json cannot be combined with field options");
        }
        return serde_json::from_str(raw)
            .map_err(|_| "--request-json must contain one JSON request");
    }
    let address = args
        .address
        .as_ref()
        .ok_or("Address or --request-json is required")?;
    let mut value = json!({"schema":1,"address":address,"mode":if args.outline {"outline"} else if resolve {"resolve"} else {"read"},"format":match args.format {OutputFormat::Md=>"md",OutputFormat::Xml=>"xml"},"children":!args.no_children});
    for (key, field) in [
        ("cursor", &args.cursor),
        ("expected_resolution_hash", &args.expected_resolution_hash),
        ("expected_source_hash", &args.expected_source_hash),
        ("expected_document_hash", &args.expected_document_hash),
    ] {
        if let Some(field) = field {
            value[key] = json!(field);
        }
    }
    if let Some(size) = args.chunk_bytes {
        value["chunk_bytes"] = json!(size);
    }
    Ok(value)
}

#[cfg(test)]
mod m_01_d_2_cases {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;

    #[test]
    fn raw_contract_request_cannot_silently_override_explicit_cli_format() {
        let raw = r#"{"schema":1,"address":"spec://org.demo/self/DOC","mode":"read","format":"md","children":true}"#;
        let error = Cli::try_parse_from([
            "vibe",
            "spec",
            "read",
            "--request-json",
            raw,
            "--format",
            "xml",
        ])
        .unwrap_err();
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        let cli = Cli::try_parse_from(["vibe", "spec", "read", "--request-json", raw]).unwrap();
        let crate::cli::Command::Spec(SpecArgs {
            command: SpecCommand::Read(args),
        }) = cli.command
        else {
            panic!("spec route");
        };
        assert!(
            vibe_wire::behaviour::source_query::read_request(&request(&args, false).unwrap())
                .is_ok()
        );
    }
}
