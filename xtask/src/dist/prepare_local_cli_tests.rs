use std::path::Path;

use clap::Parser;

use super::{DistCommand, run_prepare_local_into};

#[derive(Parser)]
struct DistArgs {
    #[command(subcommand)]
    command: DistCommand,
}

#[test]
fn local_prepare_defaults_to_version_110_without_a_skip() {
    let parsed = DistArgs::try_parse_from(["dist", "prepare-local"]).unwrap();
    let DistCommand::PrepareLocal {
        version,
        skip_targets,
        skip_reason,
        ..
    } = parsed.command
    else {
        panic!("wrong command")
    };
    assert_eq!(version, "1.1.0");
    assert!(skip_targets.is_empty());
    assert!(skip_reason.is_none());
}

#[test]
fn skip_arguments_require_each_other_and_validate_before_output_open() {
    assert!(
        DistArgs::try_parse_from([
            "dist",
            "prepare-local",
            "--skip-smoke-target",
            "x86_64-unknown-linux-gnu"
        ])
        .is_err()
    );
    assert!(
        DistArgs::try_parse_from(["dist", "prepare-local", "--skip-reason", "operator choice"])
            .is_err()
    );
    let parsed = DistArgs::try_parse_from([
        "dist",
        "prepare-local",
        "--skip-smoke-target",
        "x86_64-unknown-linux-gnu",
        "--skip-reason",
        "operator choice",
    ])
    .unwrap();
    assert!(matches!(parsed.command, DistCommand::PrepareLocal { .. }));

    let absent = tempfile::tempdir().unwrap();
    let output = absent.path().join("does-not-exist");
    for (targets, reason) in [
        (vec!["x86_64-unknown-linux-gnu".into()], "".into()),
        (vec!["wrong-target".into()], "reason".into()),
        (
            vec![
                "x86_64-unknown-linux-gnu".into(),
                "x86_64-unknown-linux-gnu".into(),
            ],
            "reason".into(),
        ),
    ] {
        let mut stdout = Vec::new();
        assert!(
            run_prepare_local_into(
                Path::new("/missing-repo"),
                "1.1.0",
                &output,
                targets,
                Some(reason),
                &mut stdout
            )
            .is_err()
        );
        assert!(stdout.is_empty());
        assert!(!output.exists());
    }
    let mut stdout = Vec::new();
    assert!(
        run_prepare_local_into(
            Path::new("/missing-repo"),
            "1.0.0",
            &output,
            Vec::new(),
            None,
            &mut stdout
        )
        .is_err()
    );
    assert!(!output.exists());
}
