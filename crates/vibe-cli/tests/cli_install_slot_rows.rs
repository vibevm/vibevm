//! The human rows of an ordinary package's slot plan and outcomes
//! (PROP-065 `#ordinary-plan`, PROP-067 `#list`).

mod common;

use common::hook_slot::{Fixture, setup};

fn human_install(fixture: Fixture) -> String {
    let (_outer, user, project, registry) = setup(fixture);
    let output = user
        .vibe()
        .env_remove("VIBE_AGENT_COMMAND")
        .env_remove("VIBE_AGENT_ARGS")
        .env("NO_COLOR", "1")
        .env("TERM", "dumb")
        .arg("install")
        .arg("org.example/hooked@=0.1.0")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    stdout
}

#[test]
fn slot_plan_is_a_rule_and_numbered_rows() {
    let stdout = human_install(Fixture::JudgmentSkip);
    let heading = "-- plan for org.example/hooked@0.1.0 · slot:post-install ";
    let (_, section) = stdout
        .split_once(heading)
        .expect("slot plan section exists");
    let rows: Vec<_> = section
        .lines()
        .skip(1)
        .skip_while(|line| line.is_empty())
        .take_while(|line| line.trim_start().starts_with("1. "))
        .collect();
    assert_eq!(rows.len(), 1, "{stdout}");
    assert_eq!(
        rows[0],
        "  1.  judged  (script)  Judged contribution  when command_present = \"no-such-command-anywhere\" does not hold  [skip]",
        "{stdout}"
    );
    assert!(!rows[0].contains("provider="), "{stdout}");
}
