//! What the runner will and will not execute.

use super::{Program, parse, split};

#[test]
fn quotes_group_a_word_that_carries_spaces() {
    let parts = split("vibe select --where \"uri:spec://a/b#C depth:1\"").expect("splits");
    assert_eq!(parts.len(), 4);
    assert_eq!(parts[3], "uri:spec://a/b#C depth:1");
}

#[test]
fn an_empty_quoted_word_is_still_a_word() {
    let parts = split("vibe init --name \"\"").expect("splits");
    assert_eq!(parts, ["vibe", "init", "--name", ""]);
}

#[test]
fn the_three_programs_are_recognised_and_nothing_else_is() {
    assert_eq!(parse("vibe list").expect("vibe").program, Program::Vibe);
    assert_eq!(parse("cargo init").expect("cargo").program, Program::Cargo);
    assert_eq!(parse("cat vibe.toml").expect("cat").program, Program::Cat);
    let refused = parse("powershell -File install.ps1").expect_err("not in the set");
    assert!(refused.to_string().contains("not one of the programs"));
    assert!(refused.to_string().contains("PROP-057#PIPE-EXAMPLE-RUNNER"));
}

#[test]
fn a_pipeline_is_refused_because_the_runner_is_not_a_shell() {
    let refused = parse("vibe list | grep wal").expect_err("needs a shell");
    assert!(refused.to_string().contains("needs a shell"));
}

#[test]
fn the_windows_spelling_of_the_binary_is_the_same_program() {
    assert_eq!(
        parse("vibe.exe --version").expect("same").program,
        Program::Vibe
    );
}
