//! Occurrences across one run: the fresh skip that records a fingerprint, the
//! fresh observation that refuses and records NOTHING, and the interrupted
//! occurrence that is continued rather than forked.

use vibe_core::manifest::SpecFormat;
use vibe_wire::generated::compiler_trace_index::e1::index::{RunStatus, Scope, ScopeStatus};

use super::super::*;
use super::support::*;
use crate::compile_trace::{RunOutcome, node_descriptor};

const PARENT_BASE: &str = "unit:org.vibevm/parent#static-md";
const ROOT_BASE: &str = "node:.#static-md";

/// RED 3 — dirty unit then fresh unit in the SAME run: the next occurrence
/// carries the byte-equal output fingerprint, the skipped occurrence has zero
/// events, and the artifact's mtime is untouched.
#[test]
fn a_fresh_unit_skips_with_the_same_fingerprint_and_zero_events() {
    let (ws_dir, ws, resolution, _srcs) = unit_and_root_fixture();
    let run = traced_run(&ws.root);
    apply_traced(&ws, &resolution, Some(&run)).expect("the first install compiles the unit");
    let static_md = unit_static(ws_dir.path(), "parent");
    let before = fs::metadata(&static_md).unwrap().modified().unwrap();

    regenerate_boot_from_traced(&ws, &resolution, SpecFormat::Mixed, Some(&run))
        .expect("the second regeneration is fresh where it may be");

    let index = run_index(&ws.root);
    let unit_scopes: Vec<&Scope> = index
        .scopes
        .iter()
        .filter(|scope| scope.id.starts_with(PARENT_BASE))
        .collect();
    assert_eq!(unit_scopes.len(), 2, "the unit base has two occurrences");
    let (compiled, skipped) = (unit_scopes[0], unit_scopes[1]);
    assert_eq!(compiled.status, ScopeStatus::Compiled);
    assert_eq!(skipped.status, ScopeStatus::Skipped);
    assert_eq!(
        compiled.fingerprint.as_deref(),
        skipped.fingerprint.as_deref(),
        "the skip records the SAME output fingerprint, byte for byte"
    );
    assert!(
        index.events.iter().all(|event| event.scope != skipped.id),
        "a skipped scope is silent"
    );
    assert!(
        index.events.iter().any(|event| event.scope == compiled.id),
        "only the compiled occurrence has events"
    );
    let after = fs::metadata(&static_md).unwrap().modified().unwrap();
    assert_eq!(
        before, after,
        "a fresh unit is not rewritten — no mtime churn"
    );
    // The root node compiled again: next occurrence, sequence still dense.
    assert_eq!(
        occurrences(&index, ROOT_BASE),
        vec!["node:.#static-md::attempt:1", "node:.#static-md::attempt:2"]
    );
    for (position, event) in index.events.iter().enumerate() {
        assert_eq!(
            event.sequence, position as u32,
            "the run stays one sequence"
        );
    }
}

/// The correction's headline law (§1): a fresh unit whose existing output
/// cannot be observed safely declares NO occurrence at all.
///
/// The mutation is deterministic and touches nothing the freshness decision
/// reads: a SECOND HARD LINK to the already-compiled `STATIC.md`. The stored
/// input fingerprint in `INDEX.md` is untouched, so the unit is still exactly
/// fresh and is still not rewritten — but `vibe-safefs` refuses to treat a
/// file with two names as exclusively owned, so the no-follow/single-link read
/// fails. The install must still succeed, the run must carry one bounded
/// warning, the unit base must NOT gain a second attempt, and `finish(Ok)`
/// must still finalise durably.
#[test]
fn a_fresh_unit_whose_output_cannot_be_observed_declares_no_occurrence() {
    let (ws_dir, ws, resolution, _srcs) = unit_and_root_fixture();
    let run = traced_run(&ws.root);
    apply_traced(&ws, &resolution, Some(&run)).expect("the first install compiles the unit");
    let static_md = unit_static(ws_dir.path(), "parent");
    let before_bytes = fs::read(&static_md).unwrap();
    let before_mtime = fs::metadata(&static_md).unwrap().modified().unwrap();
    assert_eq!(
        occurrences(&run_index(&ws.root), PARENT_BASE),
        vec!["unit:org.vibevm/parent#static-md::attempt:1"]
    );

    // The mutation: a second name for the same inode. Nothing the
    // dirty-subgraph reads changes — only the file's link count.
    let alias = static_md.with_file_name("STATIC.md.alias");
    fs::hard_link(&static_md, &alias).expect("a hard link inside one temp filesystem");

    regenerate_boot_from_traced(&ws, &resolution, SpecFormat::Mixed, Some(&run))
        .expect("the observation refusal is never an install failure");

    let index = run_index(&ws.root);
    assert_eq!(
        occurrences(&index, PARENT_BASE),
        vec!["unit:org.vibevm/parent#static-md::attempt:1"],
        "the fresh attempt declared no occurrence at all"
    );
    assert_eq!(
        index
            .scopes
            .iter()
            .find(|scope| scope.id.starts_with(PARENT_BASE))
            .map(|scope| &scope.status),
        Some(&ScopeStatus::Compiled),
        "the earlier compiled occurrence is untouched"
    );
    // The proved freshness stands: bytes and mtime are exactly as compiled.
    assert_eq!(fs::read(&static_md).unwrap(), before_bytes);
    assert_eq!(
        fs::metadata(&static_md).unwrap().modified().unwrap(),
        before_mtime,
        "a refused observation rewrites nothing"
    );

    let warnings: Vec<String> = run
        .summary()
        .warnings
        .iter()
        .map(|warning| format!("{warning}"))
        .collect();
    assert_eq!(
        warnings.len(),
        1,
        "exactly one bounded warning names the refusal: {warnings:?}"
    );
    assert!(
        warnings[0].contains("no occurrence was declared") && warnings[0].contains("STATIC.md"),
        "{warnings:?}"
    );

    // And the run still ends durably `ok`: nothing is left pending.
    let summary = run.finish(&RunOutcome::Ok, at(2_000));
    assert!(summary.finalised, "the run finalises durably");
    assert_eq!(summary.status, RunStatus::Ok);
    let final_index = run_index(&ws.root);
    assert_eq!(final_index.status, RunStatus::Ok);
    assert!(
        final_index
            .scopes
            .iter()
            .all(|scope| scope.status != ScopeStatus::Pending),
        "no occurrence was left pending for work nobody attempted"
    );
}

/// RED 4 — a pending interrupted node occurrence is REACQUIRED by the next
/// regeneration in the same run, not forked into a sibling.
#[test]
fn a_pending_interrupted_node_occurrence_is_reacquired() {
    let (_ws_dir, ws, resolution, _srcs) = unit_and_root_fixture();
    let run = traced_run(&ws.root);
    // The crash shape: a node occurrence was declared and never resolved.
    let interrupted = run
        .acquire_scope_lossy(&node_descriptor(".", SpecFormat::Mixed))
        .expect("the interrupted occurrence declares");
    assert_eq!(interrupted.id(), "node:.#static-md::attempt:1");

    apply_traced(&ws, &resolution, Some(&run)).expect("the install resumes through it");

    let index = run_index(&ws.root);
    assert_eq!(
        occurrences(&index, ROOT_BASE),
        vec!["node:.#static-md::attempt:1"],
        "the interrupted occurrence is continued exactly, never forked"
    );
    assert!(
        index
            .scopes
            .iter()
            .all(|scope| scope.status == ScopeStatus::Compiled),
        "the continued occurrence reached the same terminal word as the unit"
    );
}
