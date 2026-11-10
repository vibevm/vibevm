//! The shared judgment half, proven against the closed vocabulary rather
//! than against one plane's planner.

use super::*;
use specmark::verifies;

/// Facts whose every answer is the "nothing is here" one, so each test turns
/// on exactly the member it is about.
fn bare() -> ConditionFacts<'static> {
    ConditionFacts {
        feature_selected: &|_| false,
        param_truthy: &|_| Ok(false),
        param_equals: &|_, _| Ok(false),
        command_present: &|_| false,
        file_present: &|_| Ok(false),
        env_present: &|_| false,
        agent_configured: false,
        offline: false,
    }
}

fn no_needs() -> NeedFacts<'static> {
    NeedFacts {
        command_present: &|_| false,
        network: false,
        agent: false,
        terminal: false,
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
fn every_key_of_the_closed_vocabulary_is_decided_by_its_own_fact() {
    let yes = ConditionFacts {
        feature_selected: &|id| id == "tools",
        param_truthy: &|id| Ok(id == "deep"),
        param_equals: &|id, value| Ok(id == "mode" && value == "fast"),
        command_present: &|name| name == "cargo",
        file_present: &|path| Ok(path == "README.md"),
        env_present: &|name| name == "CI",
        agent_configured: true,
        offline: true,
    };

    let holds = [
        Condition::Feature("tools".into()),
        Condition::Param("deep".into()),
        Condition::ParamEquals(ParamEquals {
            id: "mode".into(),
            value: "fast".into(),
        }),
        Condition::CommandPresent("cargo".into()),
        Condition::FilePresent("README.md".into()),
        Condition::EnvPresent("CI".into()),
        Condition::AgentConfigured(true),
        Condition::Offline(true),
    ];
    // Eight conditions for eight keys: the whole vocabulary of `#conditions`
    // is exercised, so a widening of the enum fails this count.
    assert_eq!(holds.len(), 8);
    for condition in &holds {
        assert_eq!(
            condition_holds(condition, &yes),
            Ok(true),
            "{condition} must hold"
        );
        assert_eq!(
            condition_holds(condition, &bare()),
            Ok(false),
            "{condition} must not hold against empty facts"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
fn the_two_boolean_conditions_compare_rather_than_assert() {
    let offline = ConditionFacts {
        offline: true,
        agent_configured: true,
        ..bare()
    };
    assert_eq!(
        condition_holds(&Condition::Offline(false), &offline),
        Ok(false)
    );
    assert_eq!(
        condition_holds(&Condition::Offline(true), &offline),
        Ok(true)
    );
    assert_eq!(
        condition_holds(&Condition::AgentConfigured(false), &offline),
        Ok(false)
    );
    assert_eq!(
        condition_holds(&Condition::AgentConfigured(false), &bare()),
        Ok(true)
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
fn a_caller_refusal_reaches_the_planner_instead_of_reading_as_false() {
    let refusing = ConditionFacts {
        param_truthy: &|id| Err(format!("no parameter `{id}` is declared")),
        file_present: &|path| Err(format!("`{path}` names a forbidden variable")),
        ..bare()
    };
    assert_eq!(
        condition_holds(&Condition::Param("ghost".into()), &refusing),
        Err("no parameter `ghost` is declared".into())
    );
    assert_eq!(
        condition_holds(&Condition::FilePresent("$out/x".into()), &refusing),
        Err("`$out/x` names a forbidden variable".into())
    );
    assert!(
        verdict(&[Condition::Param("ghost".into())], &[], &refusing).is_err(),
        "a refusal must not be swallowed into a skip"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
fn when_is_a_conjunction_unless_a_negated_disjunction_and_the_reason_names_the_decider() {
    let cargo = ConditionFacts {
        command_present: &|name| name == "cargo",
        ..bare()
    };

    assert_eq!(
        verdict(
            &[
                Condition::CommandPresent("cargo".into()),
                Condition::CommandPresent("gcc".into()),
            ],
            &[],
            &cargo,
        ),
        Ok(Verdict::Skip {
            reason: "when `command_present = \"gcc\"` does not hold".into(),
        })
    );
    assert_eq!(
        verdict(
            &[Condition::CommandPresent("cargo".into())],
            &[Condition::Offline(false)],
            &cargo,
        ),
        Ok(Verdict::Skip {
            reason: "unless `offline = false` holds".into(),
        })
    );
    assert_eq!(
        verdict(&[], &[Condition::Offline(true)], &cargo),
        Ok(Verdict::Run)
    );
    assert_eq!(verdict(&[], &[], &cargo), Ok(Verdict::Run));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
fn when_decides_before_unless_so_a_reason_is_stable() {
    let both_decide = ConditionFacts {
        offline: true,
        ..bare()
    };
    // `when` fails AND `unless` holds; the printed reason is `when`'s.
    assert_eq!(
        verdict(
            &[Condition::CommandPresent("gcc".into())],
            &[Condition::Offline(true)],
            &both_decide,
        )
        .unwrap()
        .reason(),
        Some("when `command_present = \"gcc\"` does not hold")
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#needs")]
fn every_need_of_the_closed_vocabulary_is_decided_by_its_own_fact() {
    let all = NeedFacts {
        command_present: &|name| name == "cargo",
        network: true,
        agent: true,
        terminal: true,
    };
    let needs = [
        Need::Network,
        Need::Command("cargo".into()),
        Need::Agent,
        Need::Terminal,
    ];
    assert_eq!(needs.len(), 4);
    for need in &needs {
        assert!(need_met(need, &all), "{need} must be met");
        assert!(
            !need_met(need, &no_needs()),
            "{need} must be unmet against empty facts"
        );
    }
    assert!(unmet_needs(&needs, &all).is_empty());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#needs")]
fn unmet_needs_keeps_declaration_order_so_a_refusal_names_the_first() {
    let facts = NeedFacts {
        command_present: &|name| name == "cargo",
        network: true,
        agent: false,
        terminal: false,
    };
    let needs = vec![
        Need::Command("cargo".into()),
        Need::Terminal,
        Need::Command("rustfmt".into()),
        Need::Agent,
    ];
    let missing: Vec<String> = unmet_needs(&needs, &facts)
        .into_iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        missing,
        vec![
            "terminal".to_string(),
            "command:rustfmt".into(),
            "agent".into()
        ]
    );
}
