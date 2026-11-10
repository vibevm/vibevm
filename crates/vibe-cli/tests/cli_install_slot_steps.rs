//! The judgment half of an ordinary package's installation at the CLI
//! boundary: a condition that skips, a `needs` that refuses up front, and
//! the trust an untrusted dependency's contributions need before any of
//! them runs.
//!
//! PROP-065 `#conditions`, `#needs`, `#ordinary-plan`, `#ordinary-trust`.
//! The sugar half of the same engine is `cli_hook_lifecycle.rs`, and it is
//! deliberately left alone: `[hooks]` keeps the behaviour it has.

mod common;

use std::fs;

use common::hook_slot::{Fixture, documents, install, install_unconsented, setup, slot_outcomes};

#[test]
fn a_false_condition_is_planned_reported_skipped_and_never_run() {
    let (_outer, user, project, registry) = setup(Fixture::JudgmentSkip);
    let output = install(&user, project.path(), &registry);
    assert!(
        output.status.success(),
        "a skipped contribution must not fail the install: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let docs = documents(&output.stdout);
    let outcomes = slot_outcomes(&docs);
    assert_eq!(outcomes.len(), 1, "the row is reported, not omitted");
    assert_eq!(outcomes[0]["status"], "skip");
    let message = outcomes[0]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("command_present") && message.contains("no-such-command-anywhere"),
        "the skip must name the deciding condition: {message}",
    );
    // The handler it would have run appends to this file; it must not exist.
    let slot = project
        .path()
        .join(common::slot_dir("org.example.hooked", "0.1.0"));
    assert!(
        !slot.join("hook-order.txt").exists(),
        "a skipped contribution must not execute",
    );
}

#[test]
fn an_unmet_need_refuses_the_installation_up_front() {
    let (_outer, user, project, registry) = setup(Fixture::JudgmentNeed);
    let output = install(&user, project.path(), &registry);
    assert!(
        !output.status.success(),
        "an unmet need of a required contribution must refuse",
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        combined.contains("judged") && combined.contains("command:no-such-command-anywhere"),
        "the refusal names the contribution and the need: {combined}",
    );
    assert!(
        combined.contains("PROP-065#needs"),
        "and cites the law it enforces: {combined}",
    );
    // Refused UP FRONT: the slot was never populated, so nothing of the
    // package was materialised, registered or projected.
    assert!(
        !project
            .path()
            .join(common::slot_dir("org.example.hooked", "0.1.0"))
            .exists(),
        "a run that refused before it started populates no slot",
    );
}

#[test]
fn an_untrusted_dependency_is_refused_naming_the_group_and_both_remedies() {
    let (_outer, user, project, registry) = setup(Fixture::JudgmentTrust);
    let output = install_unconsented(&user, project.path(), &registry);
    assert!(
        !output.status.success(),
        "an unattended run must not execute an untrusted dependency's contributions",
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    for expected in [
        "org.example",
        "judged",
        "trusted_groups",
        "agents.toml",
        "--assume-yes",
        "PROP-065#ordinary-trust",
    ] {
        assert!(
            combined.contains(expected),
            "the refusal must name `{expected}`: {combined}",
        );
    }
    assert!(
        !project
            .path()
            .join(common::slot_dir("org.example.hooked", "0.1.0"))
            .exists(),
        "nothing of the untrusted package was materialised",
    );
}

#[test]
fn assume_yes_is_the_per_run_escape_and_runs_the_same_contributions() {
    let (_outer, user, project, registry) = setup(Fixture::JudgmentTrust);
    let output = install(&user, project.path(), &registry);
    assert!(
        output.status.success(),
        "--assume-yes is the escape agent steps already use: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let slot = project
        .path()
        .join(common::slot_dir("org.example.hooked", "0.1.0"));
    assert_eq!(
        fs::read_to_string(slot.join("hook-order.txt"))
            .unwrap()
            .trim(),
        "post",
        "the consented contribution really ran",
    );
    let outcomes = slot_outcomes(&documents(&output.stdout));
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0]["status"], "ok");
}
