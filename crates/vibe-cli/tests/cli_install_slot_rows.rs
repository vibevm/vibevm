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
        .arg("--plain")
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
    // Plain progress owns running outcomes on stderr; skips remain flat stdout rows.
    format!("{stdout}{}", String::from_utf8_lossy(&output.stderr))
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
#[test]
fn slot_outcome_of_a_skip_is_one_glyph_row() {
    let stdout = human_install(Fixture::JudgmentSkip);
    assert!(
        stdout.contains(
            "- ` judged  Judged contribution · skipped: when command_present = \"no-such-command-anywhere\" does not hold"
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("provider="), "{stdout}");
    assert!(!stdout.contains("log of `"), "{stdout}");
}

#[test]
fn slot_outcome_of_a_run_is_one_glyph_row() {
    let stdout = human_install(Fixture::JudgmentTrust);
    assert!(
        stdout.contains("+ ` judged  Judged contribution"),
        "{stdout}"
    );
    assert!(!stdout.contains("provider="), "{stdout}");
}

/// A real failed process keeps its failure and names its actual evidence log once.
#[test]
fn failed_slot_names_its_log_once() {
    use std::fs;

    let (_outer, user, project, registry) = setup(Fixture::JudgmentTrust);
    let hooks = registry.join("org.example/hooked/v0.1.0/hooks");
    fs::write(
        hooks.join("post.sh"),
        "printf FAILED-OUT\nprintf FAILED-ERR >&2\nexit 17\n",
    )
    .unwrap();
    fs::write(
        hooks.join("post.ps1"),
        "Write-Output FAILED-OUT\n[Console]::Error.Write('FAILED-ERR')\nexit 17\n",
    )
    .unwrap();
    let output = user
        .vibe()
        .env_remove("VIBE_AGENT_COMMAND")
        .env_remove("VIBE_AGENT_ARGS")
        .env("NO_COLOR", "1")
        .env("TERM", "dumb")
        .arg("--plain")
        .arg("install")
        .arg("org.example/hooked@=0.1.0")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Post-install script failure is advisory; preserve the existing CLI exit policy.
    assert!(output.status.success(), "{combined}");
    let versions = user
        .settings
        .join("cache/install-steps/org.example/hooked/0.1.0");
    let runs: Vec<_> = fs::read_dir(versions)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(runs.len(), 1, "fresh scratch has exactly this run");
    let record = &runs[0];
    let document: vibe_wire::generated::install_steps::InstallSteps =
        serde_json::from_slice(&fs::read(record.join("steps.json")).unwrap()).unwrap();
    assert_eq!(document.protocol, "vibe-install-steps/1");
    assert_eq!(document.contributions.len(), 1);
    let failed = &document.contributions[0];
    assert_eq!(
        failed.outcome,
        vibe_wire::generated::install_steps::Outcome::Fail
    );
    let log = record.join(failed.log.as_deref().expect("failed process logged"));
    assert!(log.is_file(), "{}", log.display());
    let outcome = combined
        .lines()
        .find(|line| line.starts_with("x ` "))
        .unwrap_or_else(|| panic!("missing failure outcome: {combined}"));
    assert!(
        outcome.starts_with(&format!("x ` {}", failed.id)),
        "{outcome}"
    );
    let displayed = outcome.split_once(" · log ").expect("log on failure row").1;
    // The renderer may shorten the middle, but must still name this actual file.
    let actual = log.to_string_lossy();
    if let Some((head, tail)) = displayed.split_once('…') {
        assert!(
            actual.starts_with(head) && actual.ends_with(tail),
            "{displayed}: {actual}"
        );
    } else {
        assert_eq!(displayed, actual);
    }
    assert_eq!(combined.matches(displayed).count(), 1, "{combined}");
    // Counting only the shortened spelling would miss the old full-path announcements.
    assert_eq!(
        combined
            .matches(log.file_name().unwrap().to_str().unwrap())
            .count(),
        1,
        "{combined}"
    );
    assert!(!combined.contains("log of `"), "{combined}");
}
