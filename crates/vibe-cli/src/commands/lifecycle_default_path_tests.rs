//! The default path's structural reds — the shared command cell owns the
//! composition, the clean composer is unreachable from it, and the lease
//! owner is retained for exactly the window the command owes bytes.
//!
//! Split out of `lifecycle.rs` when that file reached its 600-line budget
//! (A15b followup amendment). The tests read `lifecycle.rs` as TEXT: needles
//! are literal here because the scanned file is a different file.

/// `run` (between the DEFAULT-PATH markers) is parse → lease → config →
/// prepare → ports → classify. Every name below belongs to the shared
/// `vibe_orchestrator` command cell or to the CLI-only clean composition —
/// one reappearing inside the default path means the composition crept back
/// into the surface. This is the region proof the Opus review requires
/// (whole-file fences trip on the legitimate `run_prelude` wrapper and the
/// clean composer below).
#[test]
fn the_default_path_composes_through_the_shared_cell_alone() {
    let source = include_str!("lifecycle.rs");
    let start = source.find("DEFAULT-PATH-START").expect("markers exist");
    let end = source.find("DEFAULT-PATH-END").expect("markers exist");
    assert!(start < end);
    let region = &source[start..end];
    for needle in [
        "SelectedManifest::read",
        "run_prelude",
        "PhaseRun",
        "run_phases",
        "inclusive_chain",
        // …and the default path must never delegate to the CLI-only clean
        // composition below, either.
        "execute(",
    ] {
        assert!(
            !region.contains(needle),
            "the shared command cell owns this composition: `{needle}` reappeared \
             inside the default path",
        );
    }
}

/// The structural twin: within this file, the ONE `PhaseRun` construction
/// (and its `run_phases` call) lives in the clean composer BELOW the
/// default-path markers — so the only `PhaseRun` construction reachable
/// from `run` is the one inside the shared command cell. A second
/// construction site above or beside the default path turns this red.
#[test]
fn the_only_phaserun_construction_site_is_the_clean_composer() {
    let source = include_str!("lifecycle.rs");
    let end = source.find("DEFAULT-PATH-END").expect("markers exist");
    for needle in ["PhaseRun", "run_phases"] {
        assert_eq!(
            source.matches(needle).count(),
            1,
            "exactly one `{needle}` site may exist in this file"
        );
        let site = source.find(needle).expect("counted above");
        assert!(
            site > end,
            "the `{needle}` site belongs to the clean composer below the \
             default path, never beside it"
        );
    }
}

/// The retained lease owner outlives the executed region AND the trace
/// funnel: retained before `prepared.run`, dropped only after
/// `render_finalized`. `run(self, …)` CONSUMES the value holding the lease,
/// so forgetting `retain_lease()` would release the cooperative lock
/// silently — the ordering below is the only proof a single-process test
/// can hold (Opus review B2).
#[test]
fn the_lease_owner_is_retained_before_the_run_and_dropped_after_the_render() {
    let source = include_str!("lifecycle.rs");
    let start = source.find("DEFAULT-PATH-START").expect("markers exist");
    let end = source.find("DEFAULT-PATH-END").expect("markers exist");
    let region = &source[start..end];
    let retain = region
        .find("retain_lease()")
        .expect("the default path retains a lease owner");
    let runs = region
        .find("prepared.run(")
        .expect("the default path hands the run to the cell");
    let finalize = region
        .find("finalize(")
        .expect("the default path finalises the trace");
    let render = region
        .find("render_finalized(")
        .expect("the default path renders the finalised report");
    let drop = region
        .find("drop(lease_owner)")
        .expect("the default path drops its lease owner explicitly");
    assert!(
        retain < runs,
        "the owner is retained BEFORE the consuming run releases the cell's share",
    );
    assert!(
        runs < finalize && finalize < render,
        "the executed region, the trace funnel and the render all run while the \
         owner is held",
    );
    assert!(
        render < drop,
        "the owner is dropped only AFTER the final report is rendered",
    );
}

/// The LIVE clean composer derives its chain exactly the way the shared
/// cell's twin derivation says it does: `request.steps()` first, then
/// `steps.iter().map(step_name)` — the canonical steps→names route whose
/// equivalence with the cell's `inclusive_chain` is pinned in the
/// orchestrator's command tests. This is the discriminating source RED for
/// the divergence surface: it reads the REAL composer region (below the
/// default-path markers), not two library functions.
#[test]
fn the_clean_composer_derives_its_chain_through_the_canonical_steps() {
    let source = include_str!("lifecycle.rs");
    let end = source.find("DEFAULT-PATH-END").expect("markers exist");
    let clean_region = &source[end..];
    let steps = clean_region
        .find("request.steps()")
        .expect("the clean composer expands the request through its canonical steps");
    let chain = clean_region
        .find("steps.iter().map(step_name)")
        .expect("and derives the chain from those steps, by name");
    assert!(
        steps < chain,
        "the chain is derived FROM the expanded steps, in that order"
    );
}

/// Exactly ONE PhaseOutcome → registered-family classifier exists, and BOTH
/// paths reach it: `classify_outcome` appears exactly three times (one
/// definition, the ordinary path's call, the clean composer's call), the
/// install-barrier choice (`RegisteredReportDraft::Install`) exactly once
/// inside it. A second mapping grown beside either path — the review's
/// trap half 2 — turns both counts red.
#[test]
fn exactly_one_registered_family_classifier_serves_both_paths() {
    let source = include_str!("lifecycle.rs");
    let start = source.find("DEFAULT-PATH-START").expect("markers exist");
    let end = source.find("DEFAULT-PATH-END").expect("markers exist");
    assert_eq!(
        source.matches("classify_outcome").count(),
        3,
        "one definition plus exactly the two path call sites — no third caller, \
         no duplicated classifier",
    );
    assert_eq!(
        source.matches("RegisteredReportDraft::Install").count(),
        1,
        "the install-barrier-vs-lifecycle family choice lives in ONE place",
    );
    // …and both live-outside-the-region sites are the ones the counts named.
    assert!(source[start..end].contains("classify_outcome("));
    assert!(source[end..].contains("classify_outcome("));
}
