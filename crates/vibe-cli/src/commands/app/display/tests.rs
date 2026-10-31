use super::*;

#[test]
fn the_log_mode_comes_from_the_flag_then_the_environment_then_the_terminal() {
    assert_eq!(
        log_mode(Some(AppLogArg::Quiet), Some("live"), true),
        LogMode::Quiet
    );
    assert_eq!(
        log_mode(Some(AppLogArg::Auto), Some("quiet"), true),
        LogMode::Tail
    );
    assert_eq!(
        log_mode(Some(AppLogArg::Auto), Some("quiet"), false),
        LogMode::Live
    );
    for (value, mode) in [
        ("live", LogMode::Live),
        (" TAIL ", LogMode::Tail),
        ("quiet", LogMode::Quiet),
    ] {
        assert_eq!(log_mode(None, Some(value), false), mode, "{value:?}");
    }
    // An unreadable value is no choice at all: the default decides.
    assert_eq!(log_mode(None, Some("garbage"), true), LogMode::Tail);
    assert_eq!(log_mode(None, None, true), LogMode::Tail);
    assert_eq!(log_mode(None, None, false), LogMode::Live);
}

#[test]
fn the_tail_size_comes_from_the_flag_then_the_environment_then_five() {
    assert_eq!(tail_size(Some(0), Some("9")), 0);
    assert_eq!(tail_size(Some(12), None), 12);
    assert_eq!(tail_size(None, Some(" 3 ")), 3);
    assert_eq!(tail_size(None, Some("not a number")), DEFAULT_TAIL);
    assert_eq!(tail_size(None, None), DEFAULT_TAIL);
}

#[test]
fn a_failure_names_the_step_whose_log_tail_is_shown() {
    let failed = AppError::Step(StepError::Failed {
        key: "org.example/tool@1.0.0".into(),
        step: "build".into(),
        kind: "recipe",
        work: "/tmp/work".into(),
        source: Box::new(AppError::Hook {
            hook: "pre_install",
            key: "org.example/tool@1.0.0".into(),
            reason: "exited".into(),
        }),
    });
    assert_eq!(failing_step(&failed), Some("build"));
    let interrupted = AppError::Step(StepError::Interrupted {
        key: "org.example/tool@1.0.0".into(),
        during: "during step `build`".into(),
        step: Some("build".into()),
        work: "/tmp/work".into(),
    });
    assert_eq!(failing_step(&interrupted), Some("build"));
    let before = AppError::Step(StepError::Interrupted {
        key: "org.example/tool@1.0.0".into(),
        during: "before the payload was placed".into(),
        step: None,
        work: "/tmp/work".into(),
    });
    assert_eq!(failing_step(&before), None);
    let hook = AppError::Hook {
        hook: "post_install",
        key: "org.example/tool@1.0.0".into(),
        reason: "exited".into(),
    };
    assert_eq!(failing_step(&hook), Some("post_install"));
}

#[test]
fn a_flat_display_draws_no_panel_and_keeps_the_live_stream() {
    let flat = StepDisplay::flat();
    assert!(flat.panel().is_none());
    assert!(!flat.renders_steps());
    assert!(!flat.quiet_children());
}
