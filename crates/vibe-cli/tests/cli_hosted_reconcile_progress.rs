//! Reconciliation and truthful progress at the hosted install-family seam.
//!
//! Three things the earlier hosted e2e files do not cover, all of them about
//! what a report SAYS rather than what the engine does:
//!
//! * a slot-scoped park whose declaration disappears from the next plan is
//!   cancelled at the one slot adoption boundary, and its continuation goes
//!   with it — the mirror of what the phase plan already does;
//! * a scoped `vibe update` reports the slots it really moved: what it
//!   materialised, what it pruned at the removal itself, and the nodes boot
//!   regeneration actually rewrote — with `packages_resolved` a real resolved
//!   count rather than a re-read of the materialised list;
//! * a hosted park makes the COMMAND incomplete even when the materialisation
//!   is finished.
//!
//! The same hit-counting loopback provider the paid slot e2e uses is
//! configured and reachable throughout, so a fall-through to the paid path
//! shows up as a counter, not as "no provider configured".

mod common;

use std::path::Path;

use common::agent_provider::{MockProvider, configure_provider};
use common::hosted_slot::{
    PAID_RESULT, add_version, add_version_without_agent, assert_ok, lifecycle_state, project_at,
    publish_plain, publish_slot_agent, sole_document, sole_root, write_declared_output,
};
use common::{UserScratch, git_available};
use vibe_wire::generated::install_report::InstallReport;
use vibe_wire::generated::lifecycle_state::{ExecutionRecordScope, ExecutionRecordStatus};
use vibe_wire::generated::update_report::UpdateReport;

/// Install `org.demo/tools` in CLI mode, so the project has a locked, already
/// materialised world for update to work against.
fn seed(user: &UserScratch, project: &Path) {
    let output = user
        .vibe()
        .args(["install", "org.demo/tools", "--assume-yes", "--json"])
        .args(["--agent-mode", "cli"])
        .arg("--path")
        .arg(project)
        .output()
        .unwrap();
    assert_ok(&output);
}

fn hosted_update(user: &UserScratch, project: &Path) -> std::process::Output {
    user.vibe()
        .args(["update", "org.demo/tools", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg("--path")
        .arg(project)
        .output()
        .unwrap()
}

/// A slot-scoped park whose DECLARATION IS GONE from the plan the next run
/// builds is cancelled where that plan is adopted, and the continuation
/// nothing owes any more goes with it.
///
/// Without this the run could never complete: the delegated row names work no
/// plan will ever visit, and the persisted continuation names a target set no
/// row still needs. Both are exactly what the state invariant refuses to read
/// back, so leaving them would durably wedge the project.
#[test]
fn a_slot_park_whose_declaration_vanished_is_cancelled_with_its_continuation() {
    if !git_available() {
        eprintln!("skipping hosted reconcile e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent(outer.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    configure_provider(&user, &provider.endpoint());
    seed(&user, project.path());
    let baseline = provider.hits();

    // Park on 0.1.1, which still declares the agent row.
    add_version(&published, "slot:post-install", "0.1.1");
    let parked = hosted_update(&user, project.path());
    assert_ok(&parked);
    let report: UpdateReport = serde_json::from_value(sole_document(&parked.stdout)).unwrap();
    let handoff = report.delegation.expect("the slot row parked");
    let run_id = handoff.run_id.clone();
    let task = handoff.tasks[0].clone();
    assert!(project.path().join(&task).is_file());
    let state = lifecycle_state(project.path());
    assert!(
        state.execution.values().any(|row| {
            row.status == ExecutionRecordStatus::Delegated
                && row.scope == Some(ExecutionRecordScope::Slot)
        }),
        "the park is tagged with its typed slot scope: {state:?}",
    );
    assert!(state.run.slot_continuation.is_some());

    // 0.1.2 drops the declaration entirely. The park is now unreachable.
    add_version_without_agent(&published, "slot:post-install", "0.1.2");
    let reconciled = hosted_update(&user, project.path());
    assert_ok(&reconciled);
    assert_eq!(
        provider.hits(),
        baseline,
        "reconciling a removed declaration never pays for it",
    );

    let after: UpdateReport =
        serde_json::from_value(sole_root(&reconciled.stdout, "update")).unwrap();
    assert!(
        after.delegation.is_none(),
        "nothing is parked any more: {after:?}",
    );
    assert!(
        after.complete,
        "and with nothing owed, the command completes: {after:?}",
    );
    let cancelled: Vec<_> = after
        .contributions
        .iter()
        .filter(|row| row.status == "cancelled")
        .collect();
    assert_eq!(
        cancelled.len(),
        1,
        "the cancellation is REPORTED, not silent: {:?}",
        after.contributions,
    );
    let message = cancelled[0].message.clone().unwrap_or_default();
    assert!(
        message.contains("cancelled the parked execution"),
        "and it says what happened: {message}",
    );
    // The declaration is gone, so the row's provenance is gone with it: this
    // one really was a dependency's, and it STILL reports sentinels rather
    // than a tier and a point nothing can corroborate.
    assert_eq!(cancelled[0].tier, "<unknown>", "{:?}", cancelled[0]);
    assert_eq!(cancelled[0].provider, "<removed-declaration>");
    assert_eq!(
        cancelled[0].reference.as_deref(),
        Some("<removed-declaration>"),
    );
    assert_eq!(
        cancelled[0].point, "<removed-slot-declaration>",
        "pre versus post is not recorded, so it is not claimed: {:?}",
        cancelled[0],
    );

    let state = lifecycle_state(project.path());
    assert!(
        state
            .execution
            .values()
            .all(|row| row.status != ExecutionRecordStatus::Delegated),
        "the unreachable delegated row is gone: {state:?}",
    );
    assert!(
        state.run.slot_continuation.is_none(),
        "and the continuation nothing owed went with it: {state:?}",
    );
    assert!(
        !project.path().join(&task).exists(),
        "the task the cancelled row owned is removed too",
    );
    assert!(
        !project
            .path()
            .join(".vibe/agentic/outbox")
            .join(&run_id)
            .exists(),
        "and its proven-empty run directory is pruned",
    );
}

/// What a scoped `vibe update` reports about the slots it moved.
///
/// Every member is measured where it happened: `materialised` and `skipped`
/// come from the subtree pass, `pruned` from the removal of the superseded
/// slot itself, and `nodes_regenerated` from boot regeneration's own return.
/// `packages_resolved` is the solved subtree's size — a real count, which is
/// why it can legitimately exceed the number of slots that moved.
#[test]
fn a_scoped_update_reports_the_slots_it_really_moved() {
    if !git_available() {
        eprintln!("skipping hosted reconcile e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent(outer.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    configure_provider(&user, &provider.endpoint());
    seed(&user, project.path());
    let baseline = provider.hits();
    add_version_without_agent(&published, "slot:post-install", "0.1.1");

    let updated = user
        .vibe()
        .args(["update", "org.demo/tools", "--json", "--assume-yes"])
        .args(["--agent-mode", "cli"])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_ok(&updated);
    let report: UpdateReport =
        serde_json::from_value(sole_root(&updated.stdout, "update")).unwrap();

    assert!(report.complete, "an unparked update completes: {report:?}");
    assert!(
        !report.unchanged,
        "and it is not the fresh path: {report:?}"
    );
    assert_eq!(
        report.packages_resolved, 1,
        "the resolved count is the solved subtree's size: {report:?}",
    );
    assert!(
        report
            .materialised
            .iter()
            .any(|slot| slot.contains("tools")),
        "the new slot is named: {report:?}",
    );
    assert_eq!(
        report.pruned.len(),
        1,
        "exactly the superseded slot was removed, measured at the removal: {report:?}",
    );
    assert!(
        report.pruned[0].contains("0.1.0"),
        "and it is the OLD version's slot, not the new one: {report:?}",
    );
    assert!(
        !report.materialised.contains(&report.pruned[0]),
        "a pruned slot is never also a materialised one: {report:?}",
    );
    assert!(
        !report.nodes_regenerated.is_empty(),
        "boot regeneration's own node list reaches the report: {report:?}",
    );
    assert_eq!(report.version_bumps.len(), 1, "{report:?}");
    assert!(
        report.version_bumps[0].contains("0.1.0 -> 0.1.1"),
        "{report:?}"
    );
    assert_eq!(provider.hits(), baseline, "no row needed the provider");
}

/// A parked scoped update reports a COMPLETE materialisation and an
/// INCOMPLETE command: the two are different facts, and conflating them is
/// the lie the whole handoff exists to avoid. The prune it performed before
/// parking is reported too — it really happened.
#[test]
fn a_parked_update_reports_its_real_progress_and_an_incomplete_command() {
    if !git_available() {
        eprintln!("skipping hosted reconcile e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent(outer.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    configure_provider(&user, &provider.endpoint());
    seed(&user, project.path());
    let baseline = provider.hits();
    add_version(&published, "slot:post-install", "0.1.1");

    let parked = hosted_update(&user, project.path());
    assert_ok(&parked);
    let report: UpdateReport = serde_json::from_value(sole_document(&parked.stdout)).unwrap();
    assert!(report.delegation.is_some(), "{report:?}");
    assert!(
        !report.complete,
        "a parked command is never complete, whatever it materialised: {report:?}",
    );
    assert!(
        report
            .materialised
            .iter()
            .any(|slot| slot.contains("tools")),
        "and it still reports the slot it really placed: {report:?}",
    );
    assert_eq!(
        report.pruned.len(),
        1,
        "the superseded slot was removed before the park, so it is reported: {report:?}",
    );
    assert_eq!(
        report.packages_resolved, 1,
        "the resolved count survives the park: {report:?}",
    );
    assert_eq!(provider.hits(), baseline);

    // The resume completes it, and only then does the command report done.
    write_declared_output(project.path());
    let resumed = hosted_update(&user, project.path());
    assert_ok(&resumed);
    let after: UpdateReport = serde_json::from_value(sole_root(&resumed.stdout, "update")).unwrap();
    assert!(after.delegation.is_none(), "{after:?}");
    assert!(after.complete, "{after:?}");
    assert_eq!(provider.hits(), baseline, "the resume never pays");
}

/// A `phase:install` agent row parks AFTER the whole apply is durable: the
/// slots are placed, the lock is written, boot is regenerated. Everything the
/// materialisation reports is therefore complete — and the COMMAND still is
/// not, because it is waiting on the hosting agent.
///
/// This is the case the two flags exist to keep apart. A report that read
/// `complete` off the materialisation record would say `true` here and be
/// wrong in the one way the handoff cannot afford.
#[test]
fn a_phase_install_park_reports_finished_materialisation_and_an_unfinished_command() {
    if !git_available() {
        eprintln!("skipping hosted reconcile e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent(outer.path(), "slot:pre-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    configure_provider(&user, &provider.endpoint());

    // The dependency resolves to a version with NO agent row of its own, so
    // the only park in this run is the project's phase row. Otherwise the slot
    // row would park first, before materialisation, and prove nothing about
    // what a post-everything park reports.
    add_version_without_agent(&published, "slot:pre-install", "0.2.0");

    // The row is the PROJECT's, at a phase point, so it runs in the
    // post-durability ritual rather than at a slot callback.
    let manifest = project.path().join("vibe.toml");
    // The host needs its own `<group>/<name>` coordinate: a prompt address is
    // resolved against the CONTRIBUTING package, and a coordinate-less host
    // can address no document at all.
    let mut body = std::fs::read_to_string(&manifest).unwrap().replace(
        "name = \"demo\"",
        "name = \"demo\"
group = \"org.demo\"",
    );
    body.push_str(
        r#"
[[extension]]
id = "phase-produce"
point = "phase:install"
handler = { kind = "agent", prompt = "spec://org.demo/demo/common/agent-prompt#root" }
config.outputs = [
  { path = "docs/phase.md", kind = "file", accept = "non-empty file" },
]
"#,
    );
    std::fs::write(&manifest, body).unwrap();
    let specs = project.path().join("vibevm/vibespecs/common");
    std::fs::create_dir_all(&specs).unwrap();
    std::fs::write(
        specs.join("agent-prompt.md"),
        "# Prompt {#root}\n\nWrite the declared phase document.\n",
    )
    .unwrap();

    let parked = user
        .vibe()
        .args(["install", "org.demo/tools", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_ok(&parked);
    assert_eq!(provider.hits(), 0, "a park never reaches the provider");

    let report: InstallReport = serde_json::from_value(sole_document(&parked.stdout)).unwrap();
    let handoff = report.delegation.as_ref().expect("the phase row parked");
    assert_eq!(handoff.resume, "vibe install");
    assert!(project.path().join(&handoff.tasks[0]).is_file());
    assert!(
        report
            .materialised
            .iter()
            .any(|slot| slot.contains("tools")),
        "the apply finished before the phase ritual, and says so: {report:?}",
    );
    assert!(
        !report.nodes_regenerated.is_empty(),
        "boot regeneration ran too — this park is post-everything: {report:?}",
    );
    assert!(
        !report.complete,
        "and yet the COMMAND is not complete: it is waiting on a handoff: {report:?}",
    );

    // The state confirms the two facts are independent: a PHASE-scoped park,
    // and no slot continuation, because no slot work is owed.
    let state = lifecycle_state(project.path());
    assert!(
        state.execution.values().any(|row| {
            row.status == ExecutionRecordStatus::Delegated
                && row.scope == Some(ExecutionRecordScope::Phase)
        }),
        "the park carries the PHASE scope, not the slot one: {state:?}",
    );
    assert!(
        state.run.slot_continuation.is_none(),
        "a phase park owes no slot work, so it records no continuation: {state:?}",
    );

    // Satisfy it: the same command resumes and NOW reports a complete run.
    std::fs::create_dir_all(project.path().join("docs")).unwrap();
    std::fs::write(project.path().join("docs/phase.md"), "hosted phase\n").unwrap();
    let resumed = user
        .vibe()
        .args(["install", "org.demo/tools", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_ok(&resumed);
    assert_eq!(provider.hits(), 0, "and the resume never pays either");
    let after: InstallReport =
        serde_json::from_value(sole_root(&resumed.stdout, "install")).unwrap();
    assert!(after.delegation.is_none(), "{after:?}");
    assert!(
        after.complete,
        "with nothing owed, the command completes: {after:?}",
    );
}

/// The two counts are not the same count.
///
/// A whole-graph update resolves EVERY declared package; only the ones whose
/// slot actually moved appear in `materialised`. Reading `packages_resolved`
/// off the materialised list — as this command once did — silently
/// under-reports exactly the runs that changed the least, and reports zero for
/// a run that resolved a full graph and moved nothing.
#[test]
fn packages_resolved_counts_the_graph_not_the_slots_that_moved() {
    if !git_available() {
        eprintln!("skipping hosted reconcile e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent(outer.path(), "slot:post-install", "0.1.0");
    add_version_without_agent(&published, "slot:post-install", "0.2.0");
    publish_plain(&published.registry, "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    configure_provider(&user, &provider.endpoint());

    let seeded = user
        .vibe()
        .args(["install", "org.demo/tools", "org.demo/plain"])
        .args(["--assume-yes", "--json", "--agent-mode", "cli"])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_ok(&seeded);
    // Widen both declared constraints: `vibe install` pins what it resolved,
    // and a pinned graph can never move, which would make the second half of
    // this case vacuous rather than false.
    let manifest = project.path().join("vibe.toml");
    let widened: String = std::fs::read_to_string(&manifest)
        .unwrap()
        .lines()
        .map(|line| match line.split_once(" = ") {
            Some((name, _)) if name.contains("org.demo/") => format!(
                "{name} = \"*\"
"
            ),
            _ => format!(
                "{line}
"
            ),
        })
        .collect();
    std::fs::write(&manifest, widened).unwrap();

    // FRESH: the lock is already current, so nothing is resolved at all.
    let fresh = user
        .vibe()
        .args(["update", "--all", "--json", "--assume-yes"])
        .args(["--agent-mode", "cli"])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_ok(&fresh);
    let report: UpdateReport = serde_json::from_value(sole_root(&fresh.stdout, "update")).unwrap();
    assert!(report.unchanged, "the fresh fast path ran: {report:?}");
    assert!(report.complete, "{report:?}");
    assert_eq!(
        report.packages_resolved, 0,
        "the fresh path skips resolution, so it resolved nothing: {report:?}",
    );
    assert!(report.materialised.is_empty(), "{report:?}");
    assert!(
        !report.nodes_regenerated.is_empty(),
        "boot is still regenerated on the fresh path: {report:?}",
    );

    // RESOLVED BUT NOT MATERIALISED: a whole-graph update that parks at the
    // EARLIEST slot point stops before any slot is written. It resolved the
    // graph — that is a fact about the run — and materialised nothing yet.
    // Reading the count off the materialised list reported zero here.
    let parking = tempfile::tempdir().unwrap();
    let published = publish_slot_agent(parking.path(), "slot:pre-install", "0.1.0");
    publish_plain(&published.registry, "0.1.0");
    let second = project_at(&user, &published.registry);
    let manifest = second.path().join("vibe.toml");
    let mut body = std::fs::read_to_string(&manifest).unwrap();
    body.push_str(
        "
[requires.packages]
\"org.demo/tools\" = \"*\"
\"org.demo/plain\" = \"*\"
",
    );
    std::fs::write(&manifest, body).unwrap();

    let parked = user
        .vibe()
        .args(["update", "--all", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg("--path")
        .arg(second.path())
        .output()
        .unwrap();
    assert_ok(&parked);
    let report: UpdateReport = serde_json::from_value(sole_document(&parked.stdout)).unwrap();
    assert!(report.delegation.is_some(), "{report:?}");
    assert!(
        !report.complete,
        "a parked command is not complete: {report:?}"
    );
    assert_eq!(
        report.packages_resolved, 2,
        "the run resolved BOTH declared packages: {report:?}",
    );
    assert!(
        report.materialised.len() < report.packages_resolved as usize,
        "and the park stopped it part-way through writing them, so the two          counts differ — reading one off the other would report the wrong          number here: {report:?}",
    );
    assert!(
        report
            .materialised
            .iter()
            .any(|slot| slot.contains("tools")),
        "what it did place before parking is still named: {report:?}",
    );
    assert_eq!(provider.hits(), 0);
}
