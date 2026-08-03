//! `vibe progress baseline` — write `baseline.json` from the campaign's
//! own verdicts (PROP-043 §7.3, DRIFT-023).
//!
//! The writer facing [`rescan`](super::rescan): close-out runs this, the
//! next campaign runs `rescan --baseline` against what it wrote, and the
//! recurrence §6 describes becomes a loop that can actually be run rather
//! than a paragraph. Everything it knows it reads out of `run/cache.json`
//! — this command verifies nothing, re-judges nothing, and invents no
//! verdict; the projection is in `progress-core`, and what lives here is
//! path resolution and the summary a human reads.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#baseline");

use std::path::PathBuf;

use anyhow::{Result, bail};
use progress_core::baseline::project::{Projection, project};

use crate::cli::ProgressBaselineArgs;
use crate::output::Context;

/// Project the campaign's fact-grain verdicts onto §7.3's unit-grain
/// record and write it out, saying what it wrote and what it left out.
pub fn baseline_cmd(ctx: &Context, a: &ProgressBaselineArgs) -> Result<()> {
    let g = super::ground(&a.common)?;
    let Some(campaign) = &g.campaign else {
        bail!(
            "`vibe progress baseline` needs a campaign zone (campaigns/<id>/ or --campaign): \
             the baseline is that zone's artifact and its verdicts come from the zone's cache"
        );
    };
    // A cache that failed to load is not a campaign with no verdicts. The
    // difference matters exactly here: every other subcommand degrades to
    // a cold run, while this one would write a *truncated* baseline, and a
    // truncated baseline reads as knowledge — every unit it silently
    // dropped carries forward as `new` without anyone being told why.
    if let Some(warning) = &g.cache_warning {
        bail!(
            "refusing to write a baseline from an unreadable cache ({warning}); \
             restore `{}` (it is tracked in git) and re-run",
            campaign.join("run").join("cache.json").display()
        );
    }
    let out: PathBuf = a
        .out
        .clone()
        .unwrap_or_else(|| campaign.join("baseline.json"));

    let p = project(g.docs.iter(), &g.cache, &super::campaign_id(campaign));
    let wrote = p.baseline.store(&out)?;

    for addr in &p.collisions {
        eprintln!(
            "vibe progress: warning: two units answer to `{addr}` — only the last one is in the \
             baseline; give one of them a distinct anchor"
        );
    }
    for path in &p.undated {
        eprintln!(
            "vibe progress: warning: `{path}` carries verdicts with no `verified_at` — its units \
             are omitted rather than carried forward undated"
        );
    }
    for path in &p.stale {
        eprintln!(
            "vibe progress: warning: `{path}` moved after it was judged (its `processed_hash` is \
             not the hash this run parsed) — its units carry forward a verdict formed against \
             different text; re-verify it before sealing this baseline"
        );
    }
    report(ctx, &p, &out, wrote)
}

/// What the run did, in the two registers the adapter speaks.
fn report(ctx: &Context, p: &Projection, out: &std::path::Path, wrote: bool) -> Result<()> {
    let written = p.baseline.units.len();
    if ctx.is_json() {
        println!(
            "{}",
            serde_json::json!({
                "out": out.display().to_string(),
                // Whether the bytes moved — a second run over an unchanged
                // campaign reports `false` and leaves the file alone.
                "written": wrote,
                "units": written,
                "omitted": p.omitted.len(),
                "verdicts": p.verdicts,
                // Named, not counted: the per-file `_elements` bundles are
                // expected and the rest are coverage the campaign thinks
                // it has and the baseline does not carry.
                "unresolved": p.unresolved,
                "collisions": p.collisions,
                "undated": p.undated,
                "stale": p.stale,
            })
        );
        return Ok(());
    }
    if ctx.is_quiet() {
        return Ok(());
    }
    let breakdown = if p.verdicts.is_empty() {
        "none".to_string()
    } else {
        p.verdicts
            .iter()
            .map(|(v, n)| format!("{n} {v}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!(
        "progress baseline: {written} unit(s) → {} ({})",
        out.display(),
        if wrote {
            "written"
        } else {
            "unchanged, nothing written"
        }
    );
    println!("  verdicts: {breakdown}");
    println!(
        "  omitted for want of a judged fact: {} unit(s) — they read as `new` on the next rescan",
        p.omitted.len()
    );
    if !p.unresolved.is_empty() {
        println!(
            "  {} verdict key(s) matched no fact anchor (per-file `_elements` bundles and the like)",
            p.unresolved.len()
        );
    }
    if !p.stale.is_empty() {
        println!(
            "  {} file(s) moved after they were judged — their units carry a verdict formed \
             against different text (see the warnings above)",
            p.stale.len()
        );
    }
    if written == 0 {
        println!(
            "  (no verdicts in the campaign cache — an empty baseline, not a missing one: \
             every unit will be re-verified next time)"
        );
    }
    Ok(())
}
