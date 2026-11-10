//! The install-family registered roots: `cli-install-report`,
//! `cli-update-report` and `cli-reinstall-report`.
//!
//! Each command emits exactly ONE document of its own root, whatever it did —
//! normal apply, the fresh fast path, or a hosted park. These cases pin that
//! the root never changes with runtime status, that documents written before
//! the handoff member existed still parse, and that the progress model stays
//! slot-level: the engine measures directories, never a per-file census.

use vibe_wire::generated::install_report::{
    InstallDelegation, InstallRecommendation, InstallRecommendationOutcome, InstallReport,
    InstallSuggestion,
};
use vibe_wire::generated::lifecycle_report::LifecycleDelegation;
use vibe_wire::generated::reinstall_report::ReinstallReport;
use vibe_wire::generated::update_report::{UpdateReport, UpdateReportScope};

/// The slot-level facts every install-family root carries at its TOP level —
/// the same members the pre-R7.3 documents carried, kept there rather than
/// nested behind a new object.
fn root_facts() -> serde_json::Map<String, serde_json::Value> {
    serde_json::json!({
        "complete": true,
        "unchanged": false,
        "materialised": ["vibedeps/org.demo.tools/0.1.0"],
        "skipped": [],
        "pruned": [],
        "nodes_regenerated": ["."]
    })
    .as_object()
    .unwrap()
    .clone()
}

fn with_root_facts(mut base: serde_json::Value) -> serde_json::Value {
    let object = base.as_object_mut().unwrap();
    object.extend(root_facts());
    base
}

/// A document written before `delegation` existed still parses, and still
/// re-serialises without inventing the member.
#[test]
fn a_pre_r73_install_report_parses_and_stays_delegation_free() {
    let authored = with_root_facts(serde_json::json!({
        "ok": true,
        "command": "install",
        "project": "C:/work/demo",
    }));
    let report: InstallReport = serde_json::from_value(authored.clone()).unwrap();
    assert_eq!(report.delegation, None);
    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        authored,
        "an absent handoff round-trips as absent, never as `null`",
    );
}

/// The weak account: a row per recommendation and per suggestion, with the
/// closed outcome vocabulary spelled as the schema spells it — and both
/// members ABSENT from a document that reached no weak table, so every
/// install report written before them still round-trips unchanged
/// (`AV-FMT-105`, `PROP-077#PROJECT-RECOMMENDS-REPORT`).
#[test]
fn the_weak_members_are_rows_with_closed_outcomes_and_vanish_when_empty() {
    let authored = with_root_facts(serde_json::json!({
        "ok": true,
        "command": "install",
        "project": "C:/work/demo",
        "recommended": [
            {
                "package": "org.demo/companion",
                "by": "org.demo/recommender",
                "outcome": "installed"
            },
            {
                "package": "org.demo/absent",
                "by": "org.demo/recommender",
                "outcome": "skipped",
                "reason": "no version satisfies it, or it conflicts with the resolved graph"
            }
        ],
        "suggested": [{"package": "org.demo/hint", "by": "org.demo/recommender"}],
    }));
    let report: InstallReport = serde_json::from_value(authored.clone()).unwrap();
    assert_eq!(
        report.recommended[0].outcome,
        InstallRecommendationOutcome::Installed
    );
    assert_eq!(report.recommended[0].reason, None);
    assert_eq!(
        report.recommended[1].outcome,
        InstallRecommendationOutcome::Skipped
    );
    assert!(report.recommended[1].reason.is_some(), "a skip says why");
    assert_eq!(report.suggested[0].package, "org.demo/hint");
    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        authored,
        "the rows round-trip verbatim, reason and all",
    );

    // A replay reaches no weak table, and the document says nothing rather
    // than saying nothing happened.
    let bare = InstallReport {
        ok: true,
        command: "install".into(),
        project: "C:/work/demo".into(),
        complete: true,
        unchanged: true,
        materialised: Vec::new(),
        skipped: Vec::new(),
        pruned: Vec::new(),
        nodes_regenerated: Vec::new(),
        contributions: Vec::new(),
        notices: Vec::new(),
        hooks: Vec::new(),
        recommended: Vec::new(),
        suggested: Vec::new(),
        trace: None,
        delegation: None,
    };
    let value = serde_json::to_value(&bare).unwrap();
    assert!(value.get("recommended").is_none());
    assert!(value.get("suggested").is_none());

    // And the rows the command really builds carry the same spellings.
    let row = InstallRecommendation {
        package: "org.demo/companion".into(),
        by: "org.demo/recommender".into(),
        outcome: InstallRecommendationOutcome::Satisfied,
        reason: None,
    };
    assert_eq!(
        serde_json::to_value(&row).unwrap()["outcome"],
        serde_json::json!("satisfied"),
    );
    let hint = InstallSuggestion {
        package: "org.demo/hint".into(),
        by: "org.demo/recommender".into(),
    };
    assert_eq!(
        serde_json::to_value(&hint).unwrap(),
        serde_json::json!({"package": "org.demo/hint", "by": "org.demo/recommender"}),
    );
}

/// The parked document: still ONE `InstallReport` with `command = "install"`,
/// still `ok = true` (a handoff is durable, not a failure), carrying the
/// dependency progress that really completed before the park.
#[test]
fn a_parked_install_report_round_trips_with_its_typed_handoff() {
    let authored = with_root_facts(serde_json::json!({
        "ok": true,
        "command": "install",
        "project": "C:/work/demo",
        "delegation": {
            "run_id": "00112233445566778899aabbccddeeff",
            "tasks": [
                ".vibe/agentic/outbox/00112233445566778899aabbccddeeff/task-org.demo%2Ftools%23slot-produce.md"
            ],
            "resume": "vibe install"
        }
    }));
    let report: InstallReport = serde_json::from_value(authored.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        authored,
        "the handoff member round-trips byte-identically",
    );
    assert!(report.ok, "a park is exit-0 durable handoff, not a failure");
    assert_eq!(report.command, "install");
    let handoff = report.delegation.as_ref().unwrap();
    assert!(!handoff.tasks.is_empty(), "a handoff names its work");
    assert!(
        handoff
            .tasks
            .iter()
            .all(|task| task.starts_with(&format!(".vibe/agentic/outbox/{}/", handoff.run_id))),
        "every task lives under the run the same document reports",
    );
    assert_eq!(
        report.materialised.len(),
        1,
        "partial dependency progress before the park is reported, not erased",
    );
}

/// Progress is SLOT-level and explicitly partial when the run stopped early.
/// Nothing in the shape can express a file count, so no report can invent one.
#[test]
fn a_partial_park_records_what_it_changed_and_says_it_did_not_finish() {
    let report = InstallReport {
        ok: true,
        command: "install".into(),
        project: "C:/work/demo".into(),
        complete: false,
        unchanged: false,
        materialised: vec!["vibedeps/org.demo.first/1.0.0".into()],
        skipped: Vec::new(),
        pruned: Vec::new(),
        nodes_regenerated: Vec::new(),
        contributions: Vec::new(),
        notices: Vec::new(),
        hooks: Vec::new(),
        recommended: Vec::new(),
        suggested: Vec::new(),
        trace: None,
        delegation: Some(InstallDelegation {
            run_id: "00112233445566778899aabbccddeeff".into(),
            tasks: vec![".vibe/agentic/outbox/00112233445566778899aabbccddeeff/task-k.md".into()],
            resume: "vibe install".into(),
        }),
    };
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["complete"], serde_json::json!(false));
    assert_eq!(
        value["materialised"],
        serde_json::json!(["vibedeps/org.demo.first/1.0.0"]),
        "the slot that survived is named; the rolled-back one is not",
    );
    for absent in ["files_written", "paths", "installed", "progress"] {
        assert!(
            !serde_json::to_string(&value).unwrap().contains(absent),
            "the shape cannot express `{absent}` — a file census the engine never measured",
        );
    }
    assert_eq!(
        value["pruned"],
        serde_json::json!([]),
        "a required collection is emitted empty, never omitted",
    );
}

/// The fresh fast path: nothing moved, and the document says exactly that.
#[test]
fn the_fresh_fast_path_reports_a_complete_run_that_moved_no_slot() {
    let authored = serde_json::json!({
        "ok": true,
        "command": "install",
        "project": "C:/work/demo",
        "complete": true,
        "unchanged": true,
        "materialised": [],
        "skipped": [],
        "pruned": [],
        "nodes_regenerated": ["."]
    });
    let report: InstallReport = serde_json::from_value(authored.clone()).unwrap();
    assert!(report.unchanged && report.complete);
    assert!(report.materialised.is_empty());
    assert_eq!(serde_json::to_value(&report).unwrap(), authored);
}

/// The three registered formats carry the same three-field handoff record.
/// JTD has no cross-schema `ref`, so the duplication is deliberate — and
/// pinned here, on the wire, rather than trusted to stay in step.
#[test]
fn every_install_family_handoff_record_is_the_same_wire_shape() {
    let authored = serde_json::json!({
        "run_id": "00112233445566778899aabbccddeeff",
        "tasks": [".vibe/agentic/outbox/00112233445566778899aabbccddeeff/task-k.md"],
        "resume": "vibe create"
    });
    let install: InstallDelegation = serde_json::from_value(authored.clone()).unwrap();
    let lifecycle: LifecycleDelegation = serde_json::from_value(authored.clone()).unwrap();
    let update: vibe_wire::generated::update_report::UpdateDelegation =
        serde_json::from_value(authored.clone()).unwrap();
    let reinstall: vibe_wire::generated::reinstall_report::ReinstallDelegation =
        serde_json::from_value(authored.clone()).unwrap();
    for rendered in [
        serde_json::to_value(&install).unwrap(),
        serde_json::to_value(&lifecycle).unwrap(),
        serde_json::to_value(&update).unwrap(),
        serde_json::to_value(&reinstall).unwrap(),
    ] {
        assert_eq!(rendered, authored);
    }
}

/// `vibe update` reports UPDATE — its own command, scope and resume line. It
/// never impersonates install, even though it runs on the install substrate.
#[test]
fn a_parked_update_report_never_impersonates_install() {
    let authored = with_root_facts(serde_json::json!({
        "ok": true,
        "command": "update",
        "project": "C:/work/demo",
        "scope": "scoped",
        "packages": ["org.demo/tools"],
        "packages_resolved": 1,
        "version_bumps": [],
        "hooks": [],
        "delegation": {
            "run_id": "00112233445566778899aabbccddeeff",
            "tasks": [".vibe/agentic/outbox/00112233445566778899aabbccddeeff/task-k.md"],
            "resume": "vibe update"
        }
    }));
    let report: UpdateReport = serde_json::from_value(authored.clone()).unwrap();
    assert_eq!(report.command, "update");
    assert_eq!(report.scope, UpdateReportScope::Scoped);
    assert_eq!(report.packages, ["org.demo/tools"]);
    assert_eq!(
        report.delegation.as_ref().unwrap().resume,
        "vibe update",
        "the resume line is the command the operator actually ran",
    );
    assert_eq!(serde_json::to_value(&report).unwrap(), authored);
}

/// `vibe reinstall` likewise owns its root and its resume line.
#[test]
fn a_parked_reinstall_report_owns_its_command_and_resume() {
    let authored = with_root_facts(serde_json::json!({
        "ok": true,
        "command": "reinstall",
        "project": "C:/work/demo",
        "forced": true,
        "hooks": [],
        "delegation": {
            "run_id": "00112233445566778899aabbccddeeff",
            "tasks": [".vibe/agentic/outbox/00112233445566778899aabbccddeeff/task-k.md"],
            "resume": "vibe reinstall"
        }
    }));
    let report: ReinstallReport = serde_json::from_value(authored.clone()).unwrap();
    assert_eq!(report.command, "reinstall");
    assert_eq!(report.delegation.as_ref().unwrap().resume, "vibe reinstall");
    assert_eq!(serde_json::to_value(&report).unwrap(), authored);
}
