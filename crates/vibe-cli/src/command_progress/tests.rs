use super::*;
use crate::cli::Cli;
use clap::{CommandFactory, Parser};
use std::sync::{Arc, Mutex};
use vibe_core::progress::{ProgressEvent, ProgressEventKind, ProgressObserver};

#[derive(Default)]
struct Recorder(Mutex<Vec<ProgressEvent>>);

impl ProgressObserver for Recorder {
    fn observe(&self, event: ProgressEvent) {
        self.0.lock().unwrap().push(event);
    }
}

fn parsed_policy(argv: &[&str], terminal_attached: bool) -> CommandPolicy {
    let cli = Cli::try_parse_from(argv)
        .unwrap_or_else(|error| panic!("parse {argv:?} for policy: {error}"));
    policy(&cli.command, terminal_attached)
}

#[test]
fn top_level_policy_inventory_matches_clap() {
    let mut actual = Cli::command()
        .get_subcommands()
        .filter(|command| !command.is_hide_set())
        .map(|command| command.get_name().to_string())
        .collect::<Vec<_>>();
    actual.sort();
    let expected = TOP_LEVEL_COMMANDS
        .iter()
        .map(|name| (*name).to_string())
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn existing_roots_do_not_gain_duplicate_fallback_tasks() {
    for argv in [
        &["vibe", "install"][..],
        &["vibe", "self", "install"][..],
        &["vibe", "self", "update"][..],
        &["vibe", "self", "reinstall"][..],
        &["vibe", "self", "gc", "--build"][..],
    ] {
        assert_eq!(parsed_policy(argv, true), CommandPolicy::ExistingRoot);
    }
    assert_eq!(
        parsed_policy(&["vibe", "install", "--global", "app:example"], true),
        CommandPolicy::FiniteFallback("Command global install")
    );
}

#[test]
fn terminal_and_protocol_owners_get_no_generic_fallback() {
    for argv in [
        &["vibe", "mcp", "serve"][..],
        &["vibe", "doc", "serve"][..],
        &["vibe", "prefs", "ui"][..],
    ] {
        let policy = parsed_policy(argv, true);
        assert_eq!(policy.progress_mode(true, false), ProgressMode::Plain);
        assert_eq!(policy.fallback_label(), None, "{argv:?}");
    }
}

#[test]
fn only_finite_fallback_starts_and_terminalizes_a_generic_activity() {
    let recorder = Arc::new(Recorder::default());
    let progress = Progress::new(recorder.clone());
    for policy in [
        CommandPolicy::Interactive,
        CommandPolicy::PersistentProtocol,
        CommandPolicy::TerminalPassthrough,
        CommandPolicy::FastMetadata,
        CommandPolicy::SilentMetadata,
        CommandPolicy::ExistingRoot,
    ] {
        let activity = CommandActivity::start_with(&progress, policy);
        activity.complete(true);
    }
    assert!(recorder.0.lock().unwrap().is_empty());

    let activity = CommandActivity::start_with(
        &progress,
        CommandPolicy::FiniteFallback("Command bounded-work"),
    );
    activity.complete(false);
    let events = recorder.0.lock().unwrap();
    assert!(matches!(
        &events[0].kind,
        ProgressEventKind::Started { label } if label == "Command bounded-work"
    ));
    assert!(matches!(
        &events[1].kind,
        ProgressEventKind::Failed { message } if message == "command failed"
    ));
}

#[test]
fn fast_metadata_is_silent_while_passthrough_commands_allow_plain_stages() {
    for argv in [
        &["vibe", "extensions"][..],
        &["vibe", "self", "env"][..],
        &["vibe", "prefs", "list"][..],
    ] {
        let policy = parsed_policy(argv, true);
        assert_eq!(policy.progress_mode(true, false), ProgressMode::Disabled);
        assert_eq!(policy.fallback_label(), None, "{argv:?}");
    }
    for argv in [
        &["vibe", "term"][..],
        &["vibe", "frame"][..],
        &["vibe", "bin", "exec", "tool"][..],
        &["vibe", "trace"][..],
    ] {
        let policy = parsed_policy(argv, true);
        assert_eq!(policy.progress_mode(true, false), ProgressMode::Plain);
        assert_eq!(policy.fallback_label(), None, "{argv:?}");
    }
    assert_eq!(
        parsed_policy(&["vibe", "version"], true).progress_mode(true, false),
        ProgressMode::Disabled
    );
}

#[test]
fn tree_and_doc_serve_special_cases_follow_actual_typed_flags() {
    assert_eq!(
        parsed_policy(&["vibe", "tree"], true),
        CommandPolicy::Interactive
    );
    assert_eq!(
        parsed_policy(&["vibe", "tree"], false),
        CommandPolicy::FiniteFallback("Command tree")
    );
    assert_eq!(
        parsed_policy(&["vibe", "tree", "--plain"], true),
        CommandPolicy::FiniteFallback("Command tree")
    );
    assert_eq!(
        parsed_policy(&["vibe", "tree", "--terminal"], true),
        CommandPolicy::TerminalPassthrough
    );
    assert_eq!(
        parsed_policy(&["vibe", "tree", "--terminal"], false),
        CommandPolicy::TerminalPassthrough
    );
    assert_eq!(
        parsed_policy(&["vibe", "tree", "--terminal", "--plain"], false),
        CommandPolicy::FiniteFallback("Command tree")
    );
    assert_eq!(
        parsed_policy(&["vibe", "doc", "serve", "--print-shell"], true),
        CommandPolicy::FiniteFallback("Command doc serve --print-shell")
    );
}

#[test]
fn finite_fallback_is_plain_and_uses_only_typed_safe_names() {
    assert_eq!(
        parsed_policy(&["vibe", "search", "token=do-not-copy"], true),
        CommandPolicy::FiniteFallback("Command search")
    );
    assert_eq!(
        parsed_policy(&["vibe", "search", "token=do-not-copy"], true).progress_mode(true, false),
        ProgressMode::Plain
    );
    assert_eq!(
        parsed_policy(&["vibe", "install"], true).progress_mode(true, false),
        ProgressMode::Interactive
    );
    assert_eq!(
        parsed_policy(&["vibe", "install"], true).progress_mode(false, false),
        ProgressMode::Plain
    );
    for argv in [
        &["vibe", "list"][..],
        &["vibe", "agentic", "explain"][..],
        &["vibe", "command"][..],
        &["vibe", "prefs", "set", "tree.palette", "dark"][..],
        &["vibe", "prefs", "migrate"][..],
        &["vibe", "requirements"][..],
        &["vibe", "friends", "org.example/package"][..],
        &["vibe", "tools"][..],
        &["vibe", "bin", "list"][..],
    ] {
        assert!(
            matches!(parsed_policy(argv, true), CommandPolicy::FiniteFallback(_)),
            "{argv:?}"
        );
    }
}

#[test]
fn documented_opt_out_accepts_only_truthy_values_and_overrides_verbose_rendering() {
    for value in ["1", "true", "TRUE", " yes ", "On"] {
        assert!(progress_disabled(false, Some(value)), "{value:?}");
    }
    for value in ["", "0", "false", "no", "off", "garbage"] {
        assert!(!progress_disabled(false, Some(value)), "{value:?}");
    }
    assert!(progress_disabled(true, None));
    assert_eq!(
        CommandPolicy::ExistingRoot.progress_mode(true, true),
        ProgressMode::Disabled
    );
}
