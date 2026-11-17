use super::*;
use specmark::verifies;
use std::path::Path;

fn root() -> PathBuf {
    PathBuf::from("/opt/vibe/user-project")
}

#[test]
fn absolute_settings_admit_the_personal_root() {
    assert_eq!(
        admit_root(Some(PathBuf::from("/opt/vibe"))).unwrap(),
        root()
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-ROOT-ADMISSION")]
fn a_missing_settings_root_refuses_with_a_null_root() {
    let refusal = admit_root(None).unwrap_err();
    let report = refusal.report();
    assert_eq!(report.root, None);
    assert_eq!(report.state, State::Unavailable);
    assert_eq!(report.outcome, Outcome::Refused);
    assert_eq!(report.effects, Effects::None);
    assert_eq!(report.publication, Publication::NotStarted);
    assert!(!report.is_success());
    assert!(report.reason.as_deref().unwrap().contains("unresolved"));
    assert!(refusal.to_string().contains("VIBE_SETTINGS"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-ROOT-ADMISSION")]
fn a_relative_settings_root_refuses_with_a_null_root() {
    let refusal = admit_root(Some(PathBuf::from("rel/settings"))).unwrap_err();
    assert_eq!(refusal.report().root, None);
    assert!(refusal.reason().contains("relative"));
    assert_eq!(refusal.report().command, Command::Status);
}

#[test]
fn a_refusal_always_carries_a_remedy() {
    for refusal in [admit_root(None), admit_root(Some(PathBuf::from("x")))] {
        assert!(refusal.unwrap_err().report().remedy.is_some());
    }
}

#[test]
fn the_outcome_is_derived_from_verb_and_state() {
    let case = |command, state| {
        Report::success(
            command,
            Some(root()),
            state,
            Effects::None,
            Publication::NotStarted,
        )
        .outcome
    };
    assert_eq!(case(Command::Init, State::Absent), Outcome::Created);
    assert_eq!(
        case(Command::Init, State::Initialized),
        Outcome::AlreadyInitialized
    );
    assert_eq!(case(Command::Status, State::Absent), Outcome::Observed);
    assert_eq!(case(Command::Status, State::Initialized), Outcome::Observed);
}

#[test]
fn commands_report_their_protocol_spelling() {
    assert_eq!(Command::Init.as_str(), "user:init");
    assert_eq!(Command::Status.as_str(), "user:status");
}

#[test]
fn only_finished_outcomes_are_success() {
    let refused = Report::refused(Command::Init, None, "x", "y");
    assert!(!refused.is_success());
    let created = Report::success(
        Command::Init,
        Some(root()),
        State::Initialized,
        Effects::Changed,
        Publication::Committed,
    );
    assert!(created.is_success());
}

#[test]
fn the_report_can_be_corrected_by_cleanup() {
    let mut refusal = UserProjectRefusal::new(Report::refused(Command::Init, None, "x", "y"));
    refusal.report_mut().effects = Effects::None;
    refusal.report_mut().publication = Publication::Restored;
    assert_eq!(refusal.report().publication, Publication::Restored);
}

#[test]
fn an_unavailable_report_keeps_the_root_and_turns_success_off() {
    let report = Report::refused(
        Command::Init,
        Some(root()),
        "the root is nonempty but carries no `vibe.toml`",
        "inspect the directory",
    );
    assert_eq!(report.root, Some(root()));
    assert!(!report.is_success());
    assert_eq!(report.effects, Effects::None);
    assert_eq!(report.publication, Publication::NotStarted);
}

#[test]
fn the_refusal_display_leads_with_the_reason() {
    let refusal = UserProjectRefusal::new(Report::refused(
        Command::Init,
        Some(root()),
        "the root is nonempty",
        "move it aside",
    ));
    assert!(refusal.to_string().starts_with("the root is nonempty"));
    assert!(refusal.to_string().contains("move it aside"));
}

#[test]
fn the_root_display_uses_the_existing_machine_encoding() {
    assert_eq!(
        display(Some(Path::new("/opt/vibe/user-project"))).as_deref(),
        Some("/opt/vibe/user-project")
    );
    assert_eq!(display(None), None);
}

#[test]
fn absolute_parent_traversal_is_not_admitted_or_normalized() {
    for path in ["/existing/../settings", "/missing/../settings"] {
        let failure = admit_root(Some(PathBuf::from(path))).unwrap_err();
        assert!(failure.report().root.is_none());
        assert!(failure.reason().contains("parent traversal"));
        assert!(failure.to_string().contains("without '..'"));
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_settings_path_is_explicitly_unsupported_without_lossy_conversion() {
    use std::os::unix::ffi::OsStringExt;
    let path = PathBuf::from(std::ffi::OsString::from_vec(b"/settings/\xff".to_vec()));
    let failure = admit_root(Some(path)).unwrap_err();
    assert!(failure.report().root.is_none());
    assert!(failure.reason().contains("unsupported non-UTF-8"));
}

#[test]
fn unavailable_observation_is_not_successful() {
    let mut report = Report::refused(Command::Status, Some(root()), "unavailable", "repair");
    report.outcome = Outcome::Observed;
    assert!(!report.is_success());
}
