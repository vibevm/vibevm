use super::*;

use std::fs;

fn dir_with(name: &str, text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(dir.path().join(name), text).expect("write");
    dir
}

fn code(line: &str, cwd: &Path) -> i32 {
    let parsed = parse(line).expect("the assert parses");
    run(&parsed, Path::new("vibe"), cwd, cwd, 5).code
}

/// The corpus writes exactly three programs, and nothing else may run: an
/// assert is a check on the result, not a script.
#[test]
fn only_the_three_programs_the_corpus_writes_are_admitted() {
    assert_eq!(
        parse("vibe check --quiet").expect("ok").program,
        Program::Vibe
    );
    assert_eq!(parse("test -f a.toml").expect("ok").program, Program::Test);
    assert_eq!(
        parse("grep -q \"x\" a.toml").expect("ok").program,
        Program::Grep
    );
    let e = parse("rm -rf /").expect_err("refused");
    assert!(e.to_string().contains("not one of the programs"), "{e}");
}

#[test]
fn a_pipeline_needs_a_shell_and_this_runner_is_not_one() {
    assert!(parse("grep -q x a.toml | wc -l").is_err());
    assert!(parse("test -f a && test -f b").is_err());
}

/// `test` and `grep` are implemented here because this check runs on
/// Windows, where neither exists. A `test -f` that fails for want of
/// `test` reports the agent's work as broken.
#[test]
fn the_file_questions_answer_about_the_sandbox() {
    let dir = dir_with("vibe.toml", "[package]\nname = \"x\"\n");
    assert_eq!(code("test -f vibe.toml", dir.path()), 0);
    assert_eq!(code("test -f vibe.lock", dir.path()), 1);
    assert_eq!(code("test ! -e vibe.lock", dir.path()), 0);
    assert_eq!(code("test ! -e vibe.toml", dir.path()), 1);
    assert_eq!(code("test -d .", dir.path()), 0);
    assert_eq!(code("test -s vibe.toml", dir.path()), 0);
}

#[test]
fn an_operator_the_runner_does_not_know_fails_loudly() {
    let dir = tempfile::tempdir().expect("temp dir");
    let parsed = parse("test -z x").expect("parses");
    let got = run(&parsed, Path::new("vibe"), dir.path(), dir.path(), 5);
    assert_eq!(got.code, 2);
    assert!(got.stderr.contains("-f, -d, -e, -s"), "{}", got.stderr);
}

#[test]
fn grep_answers_about_the_content_of_a_file_in_the_sandbox() {
    let dir = dir_with("vibe.lock", "packages = [\"org.vibevm.world/wal\"]\n");
    assert_eq!(
        code("grep -q \"org.vibevm.world/wal\" vibe.lock", dir.path()),
        0
    );
    assert_eq!(code("grep -q \"org.other/thing\" vibe.lock", dir.path()), 1);
}

/// A missing file is `grep`'s own code 2, not a silent mismatch: «the
/// agent wrote nothing» and «the agent wrote the wrong thing» are
/// different reports.
#[test]
fn grep_on_a_file_that_is_not_there_says_so() {
    let dir = tempfile::tempdir().expect("temp dir");
    let parsed = parse("grep -q \"x\" vibe.lock").expect("parses");
    let got = run(&parsed, Path::new("vibe"), dir.path(), dir.path(), 5);
    assert_eq!(got.code, 2);
    assert!(got.stderr.contains("no such file"), "{}", got.stderr);
}

#[test]
fn a_pattern_that_will_not_compile_is_a_failed_assert_with_the_reason() {
    let dir = dir_with("a.txt", "x\n");
    let parsed = parse("grep -q \"[\" a.txt").expect("parses");
    let got = run(&parsed, Path::new("vibe"), dir.path(), dir.path(), 5);
    assert_eq!(got.code, 2);
    assert!(got.stderr.contains("pattern"), "{}", got.stderr);
}
