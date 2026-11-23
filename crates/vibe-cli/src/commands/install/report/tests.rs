//! Successful install trailer accounting, independent of quiet summaries.
use super::*;

fn progress(materialised: usize, nodes: usize) -> vibe_install::InstallProgress {
    vibe_install::InstallProgress {
        complete: true,
        fresh: false,
        materialised: (0..materialised).map(|i| format!("p{i}")).collect(),
        skipped: Vec::new(),
        pruned: Vec::new(),
        nodes_regenerated: (0..nodes).map(|i| format!("n{i}")).collect(),
    }
}

#[test]
fn install_trailer_counts_contributions_that_ran() {
    let summary = WorldCallbackSummary {
        selected_contributions: 3,
        executed_contributions: 3,
        successful_contributions: 1,
        fresh_contributions: 0,
        notices: 0,
    };
    assert_eq!(
        trailer_what(&progress(1, 1), summary),
        "installed 1 package · 1 boot node · 1 contribution ran · 2 not run"
    );
}

#[test]
fn install_trailer_names_kept_removed_and_fresh_paths() {
    let mut moved = progress(2, 2);
    moved.skipped = vec!["s".into()];
    moved.pruned = vec!["p".into()];
    assert_eq!(
        trailer_what(&moved, WorldCallbackSummary::default()),
        "installed 2 packages · 2 boot nodes · 1 already present · 1 stale removed"
    );
    let mut fresh = progress(0, 1);
    fresh.fresh = true;
    assert_eq!(
        trailer_what(&fresh, WorldCallbackSummary::default()),
        "vibe.lock unchanged · nothing to re-resolve · 1 node up to date"
    );
}

#[test]
fn install_trailer_separates_fresh_not_run_and_notices() {
    let summary = WorldCallbackSummary {
        selected_contributions: 5,
        executed_contributions: 4,
        successful_contributions: 2,
        fresh_contributions: 1,
        notices: 1,
    };
    assert_eq!(
        trailer_what(&progress(0, 0), summary),
        "installed 0 packages · 0 boot nodes · 2 contributions ran · 1 fresh · 2 not run · 1 notice"
    );
}
