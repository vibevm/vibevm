//! `vibe progress rescan` — the recurrence entry point (PROP-043 §7.3).
//!
//! One of the two places that know the observed tree is a git checkout —
//! the other asks only which branch it is on, to key the payload sidecar
//! (`super::grounding::payload_dir`), and asks it once per run.
//! Whether a crate moved under a verdict reaches the core as *data*
//! (`RescanOptions`) — asking git is adapter work, and it happens here,
//! once per crate the baseline names and never once per unit. The core has
//! no business knowing this project uses git (the separability law, §2).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#baseline");

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;
use progress_core::baseline::{self, Baseline, CrateState, RescanClass, RescanOptions};
use vibe_registry::{GitError, ShellGit};

use crate::cli::ProgressRescanArgs;
use crate::output::Context;

/// Three-way compare (sources ↔ markers ↔ baseline) plus the code-side
/// rules, printing the four classes with their counts.
pub fn rescan_cmd(ctx: &Context, a: &ProgressRescanArgs) -> Result<()> {
    let g = super::ground(&a.common)?;
    let base = Baseline::load(&a.baseline)?;
    let (crate_states, warning) = probe_crates(&g.root, &base);
    if let Some(w) = warning {
        eprintln!("vibe progress: warning: {w}");
    }
    let rows = baseline::rescan(
        g.docs.iter(),
        &base,
        &RescanOptions {
            crate_states,
            control_rate: a.control_rate,
        },
    );
    if ctx.is_json() {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }
    let count = |c: &RescanClass| rows.iter().filter(|r| r.class == *c).count();
    println!(
        "progress rescan vs {}: {} new, {} changed (suspect), {} carried-forward, {} control-sample",
        a.baseline.display(),
        count(&RescanClass::New),
        count(&RescanClass::Changed),
        count(&RescanClass::CarriedForward),
        count(&RescanClass::ControlSample),
    );
    for r in &rows {
        if r.class == RescanClass::CarriedForward && !r.marker_diverged {
            continue;
        }
        let mut why = String::new();
        if let Some(name) = &r.crate_moved {
            why.push_str(&format!("  [crate `{name}` moved after the verdict]"));
        }
        if r.marker_diverged {
            why.push_str("  [marker changed outside a campaign]");
        }
        println!("  {:?} {}{why}", r.class, r.addr);
    }
    Ok(())
}

/// Date every crate the baseline names — one `git log` per crate, never one
/// per unit.
///
/// Returns the map the core invalidates against, plus the single warning
/// line to print when the probe could not answer. Every unanswerable case
/// skips the rule rather than failing it: no git binary, a tree that is not
/// a checkout, a baseline vendored into a project that carries no history.
/// A named crate that is simply *gone* from the tree is the opposite case —
/// the strongest possible evidence its code moved — so it is reported as
/// such rather than passed over.
fn probe_crates(root: &Path, base: &Baseline) -> (BTreeMap<String, CrateState>, Option<String>) {
    let named: BTreeSet<&str> = base
        .units
        .values()
        .flat_map(|u| u.crates.iter().map(String::as_str))
        .collect();
    if named.is_empty() {
        return (BTreeMap::new(), None);
    }
    let git = ShellGit::new();
    // One probe first: can this tree be asked about history at all? Without
    // it, a baseline vendored into a non-checkout would read as every named
    // crate having vanished and re-verify the entire corpus.
    if let Err(e) = git.last_commit_iso(root, ".") {
        // One line, not the backend's full REQ-citing diagnostic: a rule
        // stepping aside is not a failure the operator has to act on.
        let reason = match e {
            GitError::NotInstalled => "no git binary on PATH",
            _ => "this tree is not a readable git checkout",
        };
        return (
            BTreeMap::new(),
            Some(format!(
                "{reason} — the named-crate invalidation rule is skipped"
            )),
        );
    }
    let mut states = BTreeMap::new();
    let mut unanswered = 0usize;
    for name in named {
        if !root.join("crates").join(name).is_dir() {
            states.insert(name.to_string(), CrateState::Gone);
            continue;
        }
        match git.last_commit_iso(root, &format!("crates/{name}")) {
            Ok(Some(stamp)) => {
                states.insert(name.to_string(), CrateState::LastCommit(stamp));
            }
            // Never committed: nothing to compare a verdict against.
            Ok(None) => {}
            Err(_) => unanswered += 1,
        }
    }
    let warning = (unanswered > 0).then(|| {
        format!(
            "git could not date {unanswered} of the crates the baseline names — \
             the named-crate rule is skipped for those"
        )
    });
    (states, warning)
}
