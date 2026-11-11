//! The three pure questions a Qwen Code launch asks before it may use the
//! identity carrier: which session shape it is, whether anything in the
//! launch forbids the carrier, and what the carrier file is called.

use std::path::Path;

use super::*;

const DIR: &str = "/home/u/.vibe/agent/qwen";

fn mode(args: &[&str], stdin_is_tty: bool) -> Result<QwenMode, String> {
    qwen_mode(&QwenSession { args, stdin_is_tty })
}

#[test]
fn acp_wins_over_every_other_shape() {
    for args in [
        &["--acp"][..],
        &["--experimental-acp"],
        &["--input-format", "stream-json"],
        &["--input-format=stream-json"],
        // ACP is the first question Qwen Code's own resolver asks, so a
        // prompt beside it does not make the session headless.
        &["--acp", "-p", "one turn"],
        &["-i", "a prompt", "--acp"],
    ] {
        assert_eq!(mode(args, true), Ok(QwenMode::Acp), "{args:?}");
        assert_eq!(mode(args, false), Ok(QwenMode::Acp), "{args:?}");
    }
    // Another input format is not ACP.
    assert_eq!(
        mode(&["--input-format", "text"], true),
        Ok(QwenMode::Interactive)
    );
    // And a boolean switched OFF is not the flag: `--acp=false` is what the
    // harness's own parser reads as no ACP.
    for off in ["--acp=false", "--acp=0", "--acp=FALSE"] {
        assert_eq!(mode(&[off], true), Ok(QwenMode::Interactive), "{off}");
    }
    assert_eq!(mode(&["--acp=true"], true), Ok(QwenMode::Acp));
}

#[test]
fn a_prompt_interactive_value_is_interactive_with_or_without_a_terminal() {
    for args in [
        &["-i", "do the thing"][..],
        &["--prompt-interactive", "do the thing"],
        &["--prompt-interactive=do the thing"],
    ] {
        assert_eq!(mode(args, true), Ok(QwenMode::Interactive), "{args:?}");
        assert_eq!(mode(args, false), Ok(QwenMode::Interactive), "{args:?}");
    }
    // `-i` with no value is falsy to Qwen Code, so the terminal decides —
    // which is the one place where "present with no value" and "absent"
    // part company.
    assert_eq!(mode(&["-i"], true), Ok(QwenMode::Interactive));
    assert_eq!(mode(&["-i"], false), Ok(QwenMode::Headless));
}

#[test]
fn a_prompt_is_headless_and_a_terminal_alone_is_interactive() {
    for args in [
        &["-p", "one turn"][..],
        &["--prompt", "one turn"],
        &["--prompt=one turn"],
    ] {
        assert_eq!(mode(args, true), Ok(QwenMode::Headless), "{args:?}");
        assert_eq!(mode(args, false), Ok(QwenMode::Headless), "{args:?}");
    }
    assert_eq!(mode(&[], true), Ok(QwenMode::Interactive));
    assert_eq!(mode(&[], false), Ok(QwenMode::Headless));
    // Boolean options do not take values, so nothing after them is bare.
    assert_eq!(mode(&["--yolo", "-d"], true), Ok(QwenMode::Interactive));
}

#[test]
fn a_value_taking_option_does_not_make_its_value_a_prompt() {
    for args in [
        &["-m", "fake-1"][..],
        &["--model", "fake-1"],
        &["--model=fake-1"],
        &["--openai-base-url", "http://127.0.0.1:8761/v1"],
        &["--approval-mode", "yolo"],
        &["--include-directories", "../other"],
        &["--max-session-turns", "3"],
    ] {
        assert_eq!(mode(args, true), Ok(QwenMode::Interactive), "{args:?}");
    }
}

#[test]
fn a_bare_word_with_a_terminal_is_unclassifiable_and_without_one_is_headless() {
    // It is either the session's prompt — headless — or a subcommand, in
    // which case there is no prompt and a session with a terminal is
    // interactive. Nothing but Qwen Code's own command table decides which.
    let unknown = mode(&["mcp"], true).expect_err("a bare word with a terminal is unknown");
    assert!(unknown.contains("`mcp`"), "{unknown}");
    assert!(unknown.contains("subcommand"), "{unknown}");
    assert!(mode(&["say ok"], true).is_err());
    // An option's value is not a bare word, but a second word after it is.
    assert!(mode(&["-m", "fake-1", "say ok"], true).is_err());
    // Without a terminal both readings are headless, so the word costs
    // nothing.
    assert_eq!(mode(&["mcp"], false), Ok(QwenMode::Headless));
    assert_eq!(mode(&["mcp", "list"], false), Ok(QwenMode::Headless));
    // An explicit prompt settles it even with a bare word beside it.
    assert_eq!(
        mode(&["-p", "one turn", "extra"], true),
        Ok(QwenMode::Headless)
    );
}

#[test]
fn the_styled_sentence_is_asked_for_only_by_the_one_flag_a_launcher_can_read() {
    assert!(!qwen_styled(&[]));
    assert!(qwen_styled(&["--output-style", "Concise"]));
    assert!(qwen_styled(&["--output-style=Explanatory"]));
    // `default` selects no style, which the flag's own help text says.
    assert!(!qwen_styled(&["--output-style", "default"]));
    assert!(!qwen_styled(&["--output-style", "DEFAULT"]));
    assert!(!qwen_styled(&["--output-style"]));
}

#[test]
fn every_precondition_names_what_the_user_asked_for() {
    let dir = Path::new(DIR);
    let clean = QwenAmbient::default();
    assert!(qwen_precondition(&[], &clean, dir).is_none());
    assert!(qwen_precondition(&["-p", "hello"], &clean, dir).is_none());

    let system_prompt = qwen_precondition(&["--system-prompt", "mine.md"], &clean, dir)
        .expect("a replaced prompt has no identity slot in it");
    assert!(system_prompt.contains("--system-prompt"), "{system_prompt}");
    assert!(
        qwen_precondition(&["--system-prompt=mine.md"], &clean, dir).is_some(),
        "the `=` spelling is the same flag"
    );

    let safe = qwen_precondition(&["--safe-mode"], &clean, dir).expect("safe mode asks for less");
    assert!(safe.contains("--safe-mode"), "{safe}");
    let bare = qwen_precondition(&["--bare"], &clean, dir).expect("bare asks for less");
    assert!(bare.contains("--bare"), "{bare}");
    // A boolean switched off is not the flag, so it is not a precondition
    // either: the carrier is delivered exactly when the harness's own
    // parser would have read `--bare` as false.
    assert!(qwen_precondition(&["--bare=false"], &clean, dir).is_none());
    assert!(qwen_precondition(&["--safe-mode=0"], &clean, dir).is_none());
}

#[test]
fn qwen_system_md_is_read_the_way_qwen_code_reads_it() {
    let dir = Path::new(DIR);
    for value in ["1", "true", "TRUE", "/home/u/system.md", " /x "] {
        let ambient = QwenAmbient {
            system_md: Some(value),
            identity_md: None,
        };
        let reason = qwen_precondition(&[], &ambient, dir)
            .unwrap_or_else(|| panic!("`{value}` replaces the base prompt"));
        assert!(reason.contains(QWEN_SYSTEM_MD_ENV), "{reason}");
    }
    for value in ["", "0", "false", "  "] {
        let ambient = QwenAmbient {
            system_md: Some(value),
            identity_md: None,
        };
        assert!(
            qwen_precondition(&[], &ambient, dir).is_none(),
            "`{value}` is off"
        );
    }
}

#[test]
fn an_inherited_carrier_of_ours_is_not_a_foreign_owner_of_the_slot() {
    let dir = Path::new(DIR);
    let ours = format!("{DIR}/qwen-identity-interactive-0123456789abcdef.md");
    assert!(qwen_carrier_is_ours(&ours, dir));
    assert!(!qwen_carrier_is_ours("/etc/brand-identity.md", dir));
    assert!(!qwen_carrier_is_ours("", dir));

    let inherited = QwenAmbient {
        system_md: None,
        identity_md: Some(ours.as_str()),
    };
    assert!(qwen_precondition(&[], &inherited, dir).is_none());

    let foreign = QwenAmbient {
        system_md: None,
        identity_md: Some("/etc/brand-identity.md"),
    };
    let reason = qwen_precondition(&[], &foreign, dir).expect("someone else owns the slot");
    assert!(reason.contains(QWEN_IDENTITY_ENV), "{reason}");
    assert!(reason.contains("/etc/brand-identity.md"), "{reason}");
}

#[test]
fn the_carrier_name_carries_the_two_facts_that_tell_two_files_apart() {
    let dir = Path::new(DIR);
    // The session shape and the styled-ness: a file composed for one is not
    // the file another launch may use, and the digest the command layer
    // appends keeps a newer lane from rewriting a live session's file.
    let names: Vec<String> = [
        (QwenMode::Interactive, false),
        (QwenMode::Headless, false),
        (QwenMode::Acp, false),
        (QwenMode::Interactive, true),
        (QwenMode::Headless, true),
        (QwenMode::Acp, true),
    ]
    .into_iter()
    .map(|(mode, styled)| {
        qwen_carrier_prefix(dir, mode, styled)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    })
    .collect();
    assert_eq!(
        names,
        [
            "qwen-identity-interactive-",
            "qwen-identity-headless-",
            "qwen-identity-acp-",
            "qwen-identity-interactive-styled-",
            "qwen-identity-headless-styled-",
            "qwen-identity-acp-styled-",
        ]
    );
    for prefix in &names {
        assert!(qwen_carrier_is_ours(&format!("{DIR}/{prefix}abc.md"), dir));
    }
}
