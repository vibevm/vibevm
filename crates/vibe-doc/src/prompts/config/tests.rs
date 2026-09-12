use super::*;

fn package(text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(config_path(dir.path()), text).expect("write");
    dir
}

#[test]
fn the_defaults_are_the_ones_a_manual_actually_wants() {
    let dir = package("schema = 1\n\n[doc.prompts]\nrunner = \"agent\"\n");
    let config = read(dir.path()).expect("read");
    assert_eq!(config.timeout, DEFAULT_TIMEOUT_SECS);
    assert_eq!(config.fixture, DEFAULT_FIXTURE);
    assert_eq!(config.sample, 0);
    assert!(config.skill.is_none());
}

/// The fixture cannot live on the page: PROP-045 §7 closes a `<prompt>`
/// to one attribute, and a sixth element of a closed vocabulary is a
/// change to the norm.
#[test]
fn a_row_names_the_fixture_one_prompt_starts_from() {
    let dir = package(
        "schema = 1\n\n[doc.prompts]\nrunner = \"agent\"\nfixture = \"none\"\n\n\
         [[doc.prompts.prompt]]\npage = \"start/first-project.xml\"\nid = \"first-project\"\n\
         fixture = \"empty\"\n",
    );
    let config = read(dir.path()).expect("read");
    assert_eq!(
        config.fixture_of("start/first-project.xml", "first-project"),
        "empty"
    );
    assert_eq!(config.fixture_of("other.xml", "x"), "none");
}

#[test]
fn a_declared_skip_says_what_is_missing() {
    let dir = package(
        "schema = 1\n\n[doc.prompts]\nrunner = \"agent\"\n\n\
         [[doc.prompts.prompt]]\npage = \"a.xml\"\nid = \"p\"\n\
         skip = \"publishes to a real registry\"\n",
    );
    let config = read(dir.path()).expect("read");
    assert_eq!(
        config.skip_of("a.xml", "p"),
        Some("publishes to a real registry")
    );
    assert!(config.skip_of("a.xml", "q").is_none());
}

/// One corpus through two agents is the whole point of leaving the
/// runner out of the file, so an absent `runner` is legal.
#[test]
fn a_file_may_leave_the_runner_to_the_caller() {
    let dir = package("schema = 1\n\n[doc.prompts]\ntimeout = 60\n");
    let config = read(dir.path()).expect("read");
    assert!(config.runner.is_none());
    assert_eq!(config.timeout, 60);
}

#[test]
fn an_unknown_schema_is_refused_rather_than_guessed_at() {
    let dir = package("schema = 2\n\n[doc.prompts]\nrunner = \"agent\"\n");
    let e = read(dir.path()).expect_err("refused");
    assert!(e.to_string().contains("schema 2"), "{e}");
}

/// «Zero prompts run» and «every prompt passed» print the same number.
#[test]
fn a_package_with_no_prompts_file_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let e = read(dir.path()).expect_err("refused");
    assert!(e.to_string().contains("prompts.toml"), "{e}");
    assert!(e.to_string().contains("STYLE-PROMPT-FIRST"), "{e}");
}

#[test]
fn an_unknown_key_is_refused_so_a_typo_is_never_silently_ignored() {
    let dir = package("schema = 1\n\n[doc.prompts]\nruner = \"agent\"\n");
    assert!(read(dir.path()).is_err());
}
