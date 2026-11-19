use super::*;
use crate::cli::{AgentModeArg, Cli};
use crate::output::Context;
use clap::Parser;

fn supported_for(argv: &[&str]) -> bool {
    let cli = Cli::try_parse_from(argv).unwrap();
    supported(&cli.command)
}

#[test]
fn both_user_verbs_take_the_stop_source() {
    assert!(supported_for(&["vibe", "user", "init"]));
    assert!(supported_for(&["vibe", "user", "status"]));
}

#[test]
fn the_existing_commands_keep_their_decisions() {
    assert!(supported_for(&["vibe", "install"]));
    assert!(!supported_for(&["vibe", "install", "-g", "app:example"]));
    assert!(supported_for(&["vibe", "update"]));
    assert!(!supported_for(&["vibe", "update", "-g"]));
    assert!(supported_for(&["vibe", "dl"]));
    // `exec` needs a command, and every `tool` spelling takes the source.
    assert!(supported_for(&["vibe", "tool", "px", "exec", "--", "true"]));
    // Unrelated commands still refuse the input.
    assert!(!supported_for(&["vibe", "init"]));
    assert!(!supported_for(&["vibe", "list"]));
    assert!(!supported_for(&["vibe", "check"]));
    assert!(!supported_for(&["vibe", "version"]));
}

#[test]
fn the_gate_diagnostic_names_the_supported_commands() {
    let cli = Cli::try_parse_from(["vibe", "list"]).unwrap();
    let ctx = Context::from_flags(false, false, None, false, AgentModeArg::Auto);
    let error = for_command(&ctx, &cli.command, Some(PathBuf::from("/tmp/stop")))
        .err()
        .expect("unsupported command refuses")
        .to_string();
    for named in [
        "project install/update",
        "registry publish",
        "dl",
        "user init/status",
    ] {
        assert!(error.contains(named), "{error}");
    }
}

#[test]
fn a_supported_command_without_a_stop_file_still_gets_a_source() {
    let cli = Cli::try_parse_from(["vibe", "user", "status"]).unwrap();
    let ctx = Context::from_flags(false, false, None, false, AgentModeArg::Auto);
    assert!(for_command(&ctx, &cli.command, None).is_ok());
}
