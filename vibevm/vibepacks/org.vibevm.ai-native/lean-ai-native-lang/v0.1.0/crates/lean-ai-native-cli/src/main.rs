use clap::{Parser, Subcommand};
use lean_ai_native_check::{Context, Reply, Request, execute};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "lean-ai-native",
    version,
    about = "Targeted exact-subject Lean model proofs"
)]
struct Cli {
    #[arg(long, default_value = ".")]
    path: PathBuf,
    #[arg(long)]
    toolchain_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Operation,
}

#[derive(Subcommand)]
enum Operation {
    Scan {
        #[arg(long, default_value = "lean-proof.toml")]
        carrier: PathBuf,
    },
    Check {
        #[arg(long, default_value = "lean-proof.toml")]
        carrier: PathBuf,
        #[arg(long)]
        subject: String,
    },
    VerifyWitness {
        #[arg(long, default_value = "lean-proof.toml")]
        carrier: PathBuf,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        witness: PathBuf,
    },
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                error.exit();
            }
            let reply = Reply::invalid_request(error.to_string());
            if let Ok(json) = serde_json::to_string_pretty(&reply) {
                println!("{json}");
            }
            std::process::exit(2);
        }
    };
    let request = match cli.command {
        Operation::Scan { carrier } => Request::Scan { carrier },
        Operation::Check { carrier, subject } => Request::Check { carrier, subject },
        Operation::VerifyWitness {
            carrier,
            subject,
            witness,
        } => Request::VerifyWitness {
            carrier,
            subject,
            witness,
        },
    };
    let result = execute(
        &Context {
            project_root: cli.path,
            toolchain_dir: cli.toolchain_dir,
        },
        request,
    );
    match serde_json::to_string_pretty(&result) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("serialization_failed: {error}");
            std::process::exit(2);
        }
    }
    if !result.ok {
        std::process::exit(1);
    }
}
