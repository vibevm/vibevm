//! Real clap derivation over the owned MCP argument definitions; no server I/O.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES");

use clap::{Parser, error::ErrorKind};
use std::path::PathBuf;
#[allow(dead_code)]
#[path = "../src/cli/mcp.rs"]
mod mcp_args;

#[derive(Debug, Parser)]
struct NativeMcpParser {
    #[command(flatten)]
    args: mcp_args::McpArgs,
}
fn serve(argv: &[&str]) -> mcp_args::McpServeArgs {
    let parsed = NativeMcpParser::try_parse_from(argv).unwrap();
    let mcp_args::McpSubcommand::Serve(args) = parsed.args.command else {
        panic!("expected serve")
    };
    args
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 2
)]
fn absent_migration_flags_preserve_compatibility() {
    let args = serve(&["probe", "serve"]);
    assert_eq!(args.path, PathBuf::from("."));
    assert_eq!(args.fixed_package, None);
    assert_eq!(args.fixed_cargo, None);
    assert_eq!(args.fixed_validate, None);
    assert_eq!(args.ttl_ms, None);
    assert_eq!(args.migration_recovery_store, None);
    assert!(args.migration_file.is_empty());
}

#[test]
fn fixed_validate_mode_requires_explicit_policy_flags_and_excludes_other_hosts() {
    let issue = serve(&[
        "probe",
        "serve",
        "--path",
        "/project",
        "--fixed-validate",
        "issue",
        "--ttl-ms",
        "60000",
        "--approve-captured-script",
        "--allow-detach",
    ]);
    assert_eq!(
        issue.fixed_validate,
        Some(mcp_args::McpFixedValidateMode::Issue)
    );
    assert!(issue.approve_captured_script && issue.allow_detach);
    assert!(!issue.allow_cancel_pre_dispatch && !issue.allow_cancel_deferred_boundary);
    let cold = serve(&[
        "probe",
        "serve",
        "--path",
        "/project",
        "--fixed-validate",
        "reattach",
    ]);
    assert_eq!(
        cold.fixed_validate,
        Some(mcp_args::McpFixedValidateMode::Reattach)
    );
    assert!(!cold.approve_captured_script);
    assert!(
        NativeMcpParser::try_parse_from(["probe", "serve", "--approve-captured-script",]).is_err()
    );
    assert!(
        NativeMcpParser::try_parse_from([
            "probe",
            "serve",
            "--fixed-validate",
            "issue",
            "--fixed-package",
            "issue",
        ])
        .is_err()
    );
}

#[test]
fn fixed_package_mode_is_explicit_and_ttl_is_issue_only() {
    let issue = serve(&[
        "probe",
        "serve",
        "--path",
        "/project",
        "--fixed-package",
        "issue",
        "--ttl-ms",
        "60000",
    ]);
    assert_eq!(
        issue.fixed_package,
        Some(mcp_args::McpFixedPackageMode::Issue)
    );
    assert_eq!(issue.ttl_ms, Some(60_000));
    let reattach = serve(&[
        "probe",
        "serve",
        "--path",
        "/project",
        "--fixed-package",
        "reattach",
    ]);
    assert_eq!(
        reattach.fixed_package,
        Some(mcp_args::McpFixedPackageMode::Reattach)
    );
    assert_eq!(reattach.ttl_ms, None);
    assert!(NativeMcpParser::try_parse_from(["probe", "serve", "--ttl-ms", "60000"]).is_err());
    assert!(
        NativeMcpParser::try_parse_from(["probe", "serve", "--fixed-package", "other"]).is_err()
    );
}

#[test]
fn fixed_cargo_mode_is_explicit_and_excludes_package_grant() {
    let issue = serve(&[
        "probe",
        "serve",
        "--path",
        "/project",
        "--fixed-cargo",
        "issue",
        "--ttl-ms",
        "60000",
    ]);
    assert_eq!(issue.fixed_cargo, Some(mcp_args::McpFixedCargoMode::Issue));
    assert_eq!(issue.fixed_package, None);
    assert_eq!(issue.ttl_ms, Some(60_000));
    let reattach = serve(&[
        "probe",
        "serve",
        "--path",
        "/project",
        "--fixed-cargo",
        "reattach",
    ]);
    assert_eq!(
        reattach.fixed_cargo,
        Some(mcp_args::McpFixedCargoMode::Reattach)
    );
    assert_eq!(reattach.ttl_ms, None);
    assert!(
        NativeMcpParser::try_parse_from([
            "probe",
            "serve",
            "--fixed-package",
            "issue",
            "--fixed-cargo",
            "issue",
        ])
        .is_err()
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 2
)]
fn explicit_store_preserves_repeatable_member_selection() {
    let args = serve(&[
        "probe",
        "serve",
        "--path",
        "/project",
        "--migration-recovery-store",
        "/recovery",
        "--migration-file",
        "member/vibe.toml",
        "--migration-file",
        "other/vibe.toml",
    ]);
    assert_eq!(args.path, PathBuf::from("/project"));
    assert_eq!(
        args.migration_recovery_store,
        Some(PathBuf::from("/recovery"))
    );
    assert_eq!(args.migration_file, ["member/vibe.toml", "other/vibe.toml"]);
    let args = serve(&["probe", "serve", "--migration-recovery-store", "/recovery"]);
    assert!(args.migration_file.is_empty()); // Default file admission belongs to ServerContext.
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 2
)]
fn migration_files_require_store_and_unknown_grammar_is_refused() {
    let error =
        NativeMcpParser::try_parse_from(["probe", "serve", "--migration-file", "member/vibe.toml"])
            .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
    let error = NativeMcpParser::try_parse_from(["probe", "serve", "--schema", "2"]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UnknownArgument);
    let error = NativeMcpParser::try_parse_from(["probe", "serve", "--migration-recovery-store"])
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidValue);
}
