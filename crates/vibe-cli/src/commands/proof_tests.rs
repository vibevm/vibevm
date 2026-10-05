//! The two verbs share argument grammar while keeping operation identity.

use clap::Parser;

use crate::cli::{Cli, Command};

const SUBJECT: &str = "spec://org.example/same-subject/Protocol#identity";

#[test]
fn proof_commands_parse_required_subject_witness_and_trusted_directories() {
    for name in ["proof-check", "proof-verify"] {
        let cli = Cli::try_parse_from([
            "vibe",
            name,
            "--subject",
            SUBJECT,
            "--witness",
            "proof.json",
            "--lean-toolchain-dir",
            "lean",
            "--rust-toolchain-dir",
            "rust",
            "--json",
        ])
        .unwrap();
        let args = match cli.command {
            Command::ProofCheck(args) if name == "proof-check" => args,
            Command::ProofVerify(args) if name == "proof-verify" => args,
            _ => panic!("proof operation routing changed"),
        };
        assert_eq!(args.path, std::path::Path::new("."));
        assert_eq!(args.subject, SUBJECT);
        assert_eq!(args.witness, std::path::Path::new("proof.json"));
        assert_eq!(
            args.lean_toolchain_dir.as_deref(),
            Some(std::path::Path::new("lean"))
        );
        assert_eq!(
            args.rust_toolchain_dir.as_deref(),
            Some(std::path::Path::new("rust"))
        );
        assert!(cli.json);
    }
}

#[test]
fn proof_commands_refuse_missing_and_unknown_options() {
    for name in ["proof-check", "proof-verify"] {
        for tail in [
            vec!["--subject", SUBJECT],
            vec!["--witness", "proof.json"],
            vec![
                "--subject",
                SUBJECT,
                "--witness",
                "proof.json",
                "--operation",
                "check",
            ],
        ] {
            let mut argv = vec!["vibe", name];
            argv.extend(tail);
            assert!(Cli::try_parse_from(argv).is_err());
        }
    }
}
