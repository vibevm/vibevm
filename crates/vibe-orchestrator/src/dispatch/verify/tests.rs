//! Reds for the boundary arithmetic alone — where the prefix ends, and when
//! no reconciliation is owed at all.
//!
//! They drive the REAL `RitualPlan` a fixture project collects, because the
//! quantity under test is a position in that plan: a hand-built index would
//! stay green after the rank computation was replaced by a string comparison,
//! which is exactly the mutation these exist to kill.

use std::fs;

use vibe_lifecycle::Phase;

use super::{boundary, stops};
use crate::world;
use vibe_wire::generated::shared::EvidenceStatus;

/// One `phase:build` row and one `phase:verify` row, in that plan order.
fn project(rows: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        format!("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n{rows}"),
    )
    .unwrap();
    dir
}

const BUILD_ROW: &str = "\n[[extension]]\nid = 'b'\npoint = 'phase:build'\n\
     handler = { kind = \"builtin\", name = \"log\" }\nconfig = { message = \"B\" }\n";
const VERIFY_ROW: &str = "\n[[extension]]\nid = 'v'\npoint = 'phase:verify'\n\
     handler = { kind = \"builtin\", name = \"log\" }\nconfig = { message = \"V\" }\n";

fn chain(phases: &[Phase]) -> Vec<String> {
    phases
        .iter()
        .map(|phase| phase.as_str().to_string())
        .collect()
}

/// The prefix ends at the first verify row, so a build row contributes
/// evidence and the verify row it stops before does not.
#[test]
fn the_boundary_is_the_first_verify_row() {
    let dir = project(&format!("{BUILD_ROW}{VERIFY_ROW}"));
    let phases = [Phase::Build, Phase::Verify];
    let plan = world::plan_default(dir.path(), &phases).expect("the plan loads");
    assert_eq!(plan.executions.len(), 2, "the fixture plans both rows");

    assert_eq!(boundary(&plan, &chain(&phases)), Some(1));
}

/// Verify is requested but no row marks it: the boundary is the END of the
/// plan, so a project with zero verify contributions still gets its member
/// and cannot bypass the gate by declaring nothing.
#[test]
fn an_empty_verify_phase_still_reconciles_after_the_whole_prefix() {
    let dir = project(BUILD_ROW);
    let phases = [Phase::Build, Phase::Verify];
    let plan = world::plan_default(dir.path(), &phases).expect("the plan loads");
    assert_eq!(plan.count_for(Phase::Verify), 0, "no verify row exists");

    assert_eq!(boundary(&plan, &chain(&phases)), Some(1));
}

/// A chain that never asked for verify owes no comparison, however many rows
/// it ran.
#[test]
fn a_chain_without_verify_owes_no_boundary() {
    let dir = project(BUILD_ROW);
    let phases = [Phase::Build];
    let plan = world::plan_default(dir.path(), &phases).expect("the plan loads");

    assert_eq!(boundary(&plan, &chain(&phases)), None);
}

/// Rank, not spelling: `package` is verify-or-LATER because the chain says so,
/// and a lexical `>=` would place it before `verify` and reconcile after the
/// package row had already run.
#[test]
fn a_later_phase_that_sorts_earlier_alphabetically_still_closes_the_prefix() {
    const PACKAGE_ROW: &str = "\n[[extension]]\nid = 'p'\npoint = 'phase:package'\n\
         handler = { kind = \"builtin\", name = \"log\" }\nconfig = { message = \"P\" }\n";
    assert!("package" < "verify", "the lexical trap this test names");

    let dir = project(&format!("{BUILD_ROW}{PACKAGE_ROW}"));
    let phases = [Phase::Build, Phase::Verify, Phase::Package];
    let plan = world::plan_default(dir.path(), &phases).expect("the plan loads");

    assert_eq!(
        boundary(&plan, &chain(&phases)),
        Some(1),
        "the package row is AFTER verify and never joins the evidence prefix",
    );
}

/// Two words continue and three stop — the closed vocabulary, stated once.
#[test]
fn only_a_real_mismatch_stops_verify() {
    assert!(!stops(&EvidenceStatus::Matched));
    assert!(
        !stops(&EvidenceStatus::Unavailable),
        "an undeclared project keeps today's empty verify posture",
    );
    assert!(stops(&EvidenceStatus::Stale));
    assert!(stops(&EvidenceStatus::Missing));
    assert!(stops(&EvidenceStatus::Unstable));
}
