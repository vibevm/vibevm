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
    assert_eq!(args.migration_recovery_store, None);
    assert!(args.migration_file.is_empty());
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
