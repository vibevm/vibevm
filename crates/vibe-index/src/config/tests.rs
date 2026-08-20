//! The ladder's unit tests — split from `config.rs` when the module
//! crossed the 600-line budget (the same seam `memory/tests.rs` uses).

use super::*;

/// An env lookup over a fixed map — the process environment is
/// never touched, so these tests run in parallel without races.
/// The map is owned by the closure, so the returned type captures
/// no lifetime.
fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
    let pairs: Vec<(String, String)> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |name| {
        pairs
            .iter()
            .find(|(k, _)| k.as_str() == name)
            .map(|(_, v)| v.clone())
    }
}

/// A ladder whose file rung carries `body` (parsed strictly, so a
/// bad body fails the `unwrap` loudly here).
fn ladder_with(body: &str) -> Ladder {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("config.toml"), body).unwrap();
    Ladder::load(dir.path()).unwrap()
}

/// A ladder whose file rung is absent (fresh empty data-dir).
fn absent_file_ladder() -> Ladder {
    let dir = tempfile::tempdir().unwrap();
    Ladder::load(dir.path()).unwrap()
}

// ---- the four rungs, each alone ----

#[test]
fn flag_rung_resolves_alone() {
    let ladder = absent_file_ladder();
    let r = ladder.resolve(&Member::DUMP_FORMAT, Some("json"), &env_of(&[]));
    assert_eq!(r.value, "json");
    assert_eq!(r.source, Source::Flag("--format"));
}

#[test]
fn env_rung_resolves_alone() {
    let ladder = absent_file_ladder();
    let r = ladder.resolve(
        &Member::GIT,
        None,
        &env_of(&[("VIBE_INDEX_GIT", "C:/tools/git.exe")]),
    );
    assert_eq!(r.value, "C:/tools/git.exe");
    assert_eq!(r.source, Source::Env("VIBE_INDEX_GIT".to_string()));
}

#[test]
fn file_rung_resolves_alone() {
    let ladder = ladder_with("api-base = \"https://ghe.example.invalid/api/v3\"\n");
    let r = ladder.resolve(&Member::API_BASE, None, &env_of(&[]));
    assert_eq!(r.value, "https://ghe.example.invalid/api/v3");
    assert_eq!(r.source, Source::ConfigFile(ladder.path().to_path_buf()));
}

#[test]
fn default_rung_resolves_alone_when_file_absent() {
    let ladder = absent_file_ladder();
    let r = ladder.resolve(&Member::DUMP_FORMAT, None, &env_of(&[]));
    assert_eq!(r.value, "jsonl");
    assert_eq!(r.source, Source::Default);
}

#[test]
fn default_rung_resolves_when_file_present_but_key_absent() {
    let ladder = ladder_with("git = \"git.exe\"\n");
    let r = ladder.resolve(&Member::DUMP_FORMAT, None, &env_of(&[]));
    assert_eq!(r.value, "jsonl");
    assert_eq!(r.source, Source::Default);
}

// ---- pairwise priorities, top rung first ----

#[test]
fn flag_beats_env_and_file() {
    let ladder = ladder_with("dump-format = \"json\"\n");
    let r = ladder.resolve(
        &Member::DUMP_FORMAT,
        Some("jsonl"),
        &env_of(&[("VIBE_INDEX_DUMP_FORMAT", "json")]),
    );
    assert_eq!(r.value, "jsonl");
    assert_eq!(r.source, Source::Flag("--format"));
}

#[test]
fn env_beats_file() {
    let ladder = ladder_with("dump-format = \"json\"\n");
    let r = ladder.resolve(
        &Member::DUMP_FORMAT,
        None,
        &env_of(&[("VIBE_INDEX_DUMP_FORMAT", "jsonl")]),
    );
    assert_eq!(r.value, "jsonl");
    assert_eq!(r.source, Source::Env("VIBE_INDEX_DUMP_FORMAT".to_string()));
}

#[test]
fn file_beats_default() {
    let ladder = ladder_with("api-base = \"https://ghe.example.invalid/api/v3\"\n");
    let r = ladder.resolve(&Member::API_BASE, None, &env_of(&[]));
    assert_eq!(r.value, "https://ghe.example.invalid/api/v3");
    assert_ne!(r.source, Source::Default);
}

#[test]
fn narrow_env_name_beats_broad_one() {
    // VIBE_INDEX_LOG (family-named, narrow) beats VIBE_LOG (the
    // recorded broad legacy lever), which beats the file rung.
    let ladder = ladder_with("log-level = \"trace\"\n");
    let both = ladder.resolve(
        &Member::LOG_LEVEL,
        None,
        &env_of(&[("VIBE_LOG", "error"), ("VIBE_INDEX_LOG", "debug")]),
    );
    assert_eq!(both.value, "debug");
    assert_eq!(both.source, Source::Env("VIBE_INDEX_LOG".to_string()));

    let broad_only = ladder.resolve(&Member::LOG_LEVEL, None, &env_of(&[("VIBE_LOG", "error")]));
    assert_eq!(broad_only.value, "error");
    assert_eq!(broad_only.source, Source::Env("VIBE_LOG".to_string()));
}

#[test]
fn broad_env_name_beats_file() {
    let ladder = ladder_with("log-level = \"trace\"\n");
    let r = ladder.resolve(&Member::LOG_LEVEL, None, &env_of(&[("VIBE_LOG", "error")]));
    assert_eq!(r.value, "error");
    assert_eq!(r.source, Source::Env("VIBE_LOG".to_string()));
}

#[test]
fn empty_flag_value_counts_as_not_passed() {
    let ladder = ladder_with("dump-format = \"json\"\n");
    let r = ladder.resolve(&Member::DUMP_FORMAT, Some("  "), &env_of(&[]));
    assert_eq!(r.value, "json");
    assert_eq!(r.source, Source::ConfigFile(ladder.path().to_path_buf()));
}

// ---- the file rung's strictness ----

#[test]
fn absent_file_is_legal() {
    let dir = tempfile::tempdir().unwrap();
    let ladder = Ladder::load(dir.path()).unwrap();
    assert!(!ladder.file_present());
}

#[test]
fn unknown_key_refuses_loudly() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("config.toml"), "limit = 10\n").unwrap();
    let err = Ladder::load(dir.path()).unwrap_err().to_string();
    assert!(err.contains("limit"), "must name the unknown key: {err}");
    assert!(err.contains("unknown key"), "{err}");
}

#[test]
fn malformed_toml_refuses_loudly() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("config.toml"), "not toml [").unwrap();
    let err = Ladder::load(dir.path()).unwrap_err().to_string();
    assert!(err.contains("does not parse"), "{err}");
}

#[test]
fn non_string_value_refuses_loudly() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("config.toml"), "git = 5\n").unwrap();
    let err = Ladder::load(dir.path()).unwrap_err().to_string();
    assert!(err.contains("git"), "{err}");
    assert!(err.contains("non-string"), "{err}");
}

// ---- member-edge validation ----

#[test]
fn log_filter_validates_the_closed_set_everywhere_but_vibe_log() {
    let ladder = absent_file_ladder();
    let err = resolve_log_filter(&ladder, None, &env_of(&[("VIBE_INDEX_LOG", "loud")]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("loud"), "{err}");
    assert!(
        err.contains("off, error, warn, info, debug, trace"),
        "{err}"
    );

    let file_ladder = ladder_with("log-level = \"loud\"\n");
    let err = resolve_log_filter(&file_ladder, None, &env_of(&[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("log-level"), "{err}");
}

#[test]
fn vibe_log_passes_its_directive_through_verbatim() {
    let ladder = absent_file_ladder();
    let r = resolve_log_filter(
        &ladder,
        None,
        &env_of(&[("VIBE_LOG", "vibe_index=debug,info")]),
    )
    .unwrap();
    assert_eq!(r.value, "vibe_index=debug,info");
    assert_eq!(r.source, Source::Env("VIBE_LOG".to_string()));
}

#[test]
fn log_filter_from_flag_lands_on_the_filter_string() {
    let ladder = absent_file_ladder();
    let r = resolve_log_filter(&ladder, Some(crate::cli::LogLevel::Debug), &env_of(&[])).unwrap();
    assert_eq!(r.value, "debug");
    assert_eq!(r.source, Source::Flag("--log-level"));
}

#[test]
fn git_member_refuses_an_empty_value() {
    let ladder = ladder_with("git = \"\"\n");
    let err = resolve_git(&ladder, &env_of(&[])).unwrap_err().to_string();
    assert!(err.contains("git"), "{err}");
    assert!(err.contains("empty"), "{err}");
}

#[test]
fn dump_format_validates_its_vocabulary() {
    let ladder = ladder_with("dump-format = \"yaml\"\n");
    let err = resolve_dump_format(&ladder, None, &env_of(&[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("yaml"), "{err}");
    assert!(err.contains("jsonl, json"), "{err}");

    let ok = ladder_with("dump-format = \"json\"\n");
    let (parsed, r) = resolve_dump_format(&ok, None, &env_of(&[])).unwrap();
    assert_eq!(parsed, crate::cli::dump::DumpFormat::Json);
    assert_eq!(r.value, "json");
}

// ---- the env dialect itself ----

#[test]
fn env_normalisation_trims_and_drops_empty() {
    assert_eq!(
        normalise_env("  debug ".to_string()),
        Some("debug".to_string())
    );
    assert_eq!(normalise_env("".to_string()), None);
    assert_eq!(normalise_env("   ".to_string()), None);
}
