use super::*;

fn outcome(id: &str, verdict: Verdict, asserts: Vec<AssertOutcome>) -> Outcome {
    Outcome {
        page: "howto/do-it.xml".to_owned(),
        id: id.to_owned(),
        fixture: "empty".to_owned(),
        needs: Some("the vibevm skill".to_owned()),
        runner_code: Some(0),
        tail: "I created the project.".to_owned(),
        asserts,
        verdict,
    }
}

fn report(outcomes: Vec<Outcome>) -> Report {
    Report {
        outcomes,
        unreadable: Vec::new(),
        runner: "agent --quiet".to_owned(),
    }
}

fn held(command: &str) -> AssertOutcome {
    AssertOutcome {
        command: command.to_owned(),
        code: 0,
        output: String::new(),
    }
}

fn broke(command: &str) -> AssertOutcome {
    AssertOutcome {
        command: command.to_owned(),
        code: 1,
        output: "nothing there".to_owned(),
    }
}

#[test]
fn every_assert_exiting_zero_is_the_only_green_state() {
    let r = report(vec![outcome(
        "a",
        Verdict::Held,
        vec![held("test -f vibe.toml"), held("vibe check --quiet")],
    )]);
    assert!(r.ok());
    assert_eq!(r.counts().held, 1);
}

#[test]
fn a_broken_assert_is_red_and_its_code_is_on_the_line() {
    let r = report(vec![outcome(
        "a",
        Verdict::Broke,
        vec![held("test -f vibe.toml"), broke("vibe check --quiet")],
    )]);
    assert!(!r.ok());
    let text = r.render();
    assert!(text.contains("assert 2 exit 1"), "{text}");
    assert!(text.contains("nothing there"), "{text}");
}

/// The tail is for a person, never for a comparison: an agent says
/// something different every time, and comparing what it said would make
/// this check fail on the weather.
#[test]
fn the_agents_last_words_are_shown_under_a_red_prompt() {
    let r = report(vec![outcome(
        "a",
        Verdict::Broke,
        vec![broke("vibe check")],
    )]);
    let text = r.render();
    assert!(text.contains("the agent's last words"), "{text}");
    assert!(text.contains("| I created the project."), "{text}");
}

#[test]
fn a_skipped_prompt_says_why_and_is_not_red() {
    let r = report(vec![outcome(
        "a",
        Verdict::Skipped {
            reason: "illustrative (assert=\"none\") — nothing was claimed to check".to_owned(),
        },
        Vec::new(),
    )]);
    assert!(r.ok());
    assert_eq!(r.counts().skipped, 1);
    assert!(r.render().contains("illustrative"), "{}", r.render());
}

/// A page the pivot refused carries prompts nobody can run.
#[test]
fn an_unreadable_page_is_red_in_its_own_right() {
    let mut r = report(Vec::new());
    r.unreadable.push("a.xml".to_owned());
    assert!(!r.ok());
    assert!(r.render().contains("a.xml does not parse"));
}

#[test]
fn the_summary_names_the_runner_the_prompts_went_to() {
    let r = report(vec![outcome("a", Verdict::Held, vec![held("vibe check")])]);
    assert!(
        r.render().contains("[runner: agent --quiet]"),
        "{}",
        r.render()
    );
}

#[test]
fn the_tail_keeps_the_last_lines_and_not_the_first() {
    let text = (1..=40).map(|n| format!("line {n}\n")).collect::<String>();
    let kept = tail(&text);
    assert_eq!(kept.lines().count(), TAIL_LINES);
    assert!(kept.ends_with("line 40"));
}
