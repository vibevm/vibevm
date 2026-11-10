//! The ordinary plane's judgment and its trust policy, decided without a
//! filesystem, a process or a prompt.

use super::*;
use specmark::verifies;
use vibe_core::manifest::ParamEquals;

fn host(commands: &'static [&'static str]) -> SlotJudgmentContext {
    fn cargo_only(name: &str) -> bool {
        name == "cargo"
    }
    let _ = commands;
    SlotJudgmentContext {
        command_present: cargo_only,
        assume_yes: false,
        ..SlotJudgmentContext::consenting()
    }
}

fn slot() -> std::path::PathBuf {
    std::path::PathBuf::from("/nonexistent-slot-root")
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-grammar")]
fn a_contribution_without_judgment_always_runs() {
    let (run, reason) = host(&[]).decide(None, false, &slot());
    assert!(run);
    assert_eq!(reason, None);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
fn a_false_condition_skips_and_names_the_decider() {
    let declared = SlotJudgment {
        when: vec![Condition::CommandPresent("gcc".into())],
        ..SlotJudgment::default()
    };
    let (run, reason) = host(&[]).decide(Some(&declared), false, &slot());
    assert!(!run);
    assert_eq!(
        reason.as_deref(),
        Some("when `command_present = \"gcc\"` does not hold")
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
fn a_feature_condition_is_false_until_package_scope_configuration_installs() {
    // PROP-069 `#PACKAGE-ADMISSION` still refuses to install a package that
    // declares `[configuration]`, so the selected set is empty by
    // construction and the plan says so rather than pretending.
    for declared in [
        SlotJudgment {
            when: vec![Condition::Feature("tools".into())],
            ..SlotJudgment::default()
        },
        SlotJudgment {
            when: vec![Condition::Param("deep".into())],
            ..SlotJudgment::default()
        },
        SlotJudgment {
            when: vec![Condition::ParamEquals(ParamEquals {
                id: "mode".into(),
                value: "fast".into(),
            })],
            ..SlotJudgment::default()
        },
    ] {
        let (run, reason) = host(&[]).decide(Some(&declared), false, &slot());
        assert!(!run, "{declared:?}");
        assert!(reason.is_some());
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#needs")]
fn an_unmet_need_is_named_and_a_met_one_is_not() {
    let context = host(&[]);
    assert_eq!(
        context.unmet(
            &[Need::Command("cargo".into()), Need::Command("gcc".into())],
            false
        ),
        vec!["command:gcc".to_string()]
    );
    assert!(
        context
            .unmet(&[Need::Command("cargo".into())], false)
            .is_empty()
    );
    // `#needs`: `--offline` turns a `network` need into a refusal rather
    // than a late failure.
    assert_eq!(
        context.unmet(&[Need::Network], true),
        vec!["network".to_string()]
    );
    assert!(context.unmet(&[Need::Network], false).is_empty());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
fn the_trusted_group_match_is_the_exact_one_agent_steps_use() {
    let context = SlotJudgmentContext {
        trusted_groups: vec!["org.example".into(), "org.vibevm".into()],
        ..SlotJudgmentContext::consenting()
    };
    assert!(context.trusts("org.example"));
    assert!(context.trusts("org.vibevm"));
    // No prefix and no glob: a sub-group is a different group, exactly as
    // PROP-064 `#consent` already decides it.
    assert!(!context.trusts("org.vibevm.ai-native"));
    assert!(!context.trusts("org.exampleX"));
    assert!(!context.trusts("ORG.EXAMPLE"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-grammar")]
fn the_plan_rows_print_the_conditions_the_verdict_came_from() {
    let declared = SlotJudgment {
        when: vec![Condition::CommandPresent("cargo".into())],
        unless: vec![Condition::Offline(true)],
        ..SlotJudgment::default()
    };
    assert_eq!(
        condition_lines(Some(&declared)),
        vec![
            "when: command_present = \"cargo\"".to_string(),
            "unless: offline = true".into()
        ]
    );
    assert!(condition_lines(None).is_empty());
}
