//! `vibe progress` — the vibevm adapter over `progress-core`
//! (PROP-043 §5). All markup knowledge lives in the core; this file only
//! resolves paths, wires the campaign zone, and renders to the terminal.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#tool");

use std::path::Path;

use anyhow::{Context as _, Result, bail};
use progress_core::doc::Severity;
use progress_core::evidence::{EvidenceProvider, NoEvidence};
use progress_core::model::Audience;
use progress_core::report::View;
use progress_core::{cache, journal, report, rollup, state, weave};

use crate::cli::{
    GateStatusArg, ProgressArgs, ProgressCheckArgs, ProgressCommonArgs, ProgressGateArgs,
    ProgressReportArgs, ProgressSubcommand, ProgressWeaveArgs,
};
use crate::output::Context;

// What `tests.rs` and its submodules reach through `use super::*` and the
// adapter itself no longer spells: the grounding cell took the last uses of
// these names with it (DRIFT-025), and the move is only a move if no test
// file had to change.
#[cfg(test)]
use progress_core::{doc::ParsedDoc, sidecar};
#[cfg(test)]
use std::{collections::BTreeMap, path::PathBuf};

/// The campaign-grounding cell every verb enters through: the observed
/// tree, the campaign zone, and the caches behind them (PROP-043 §7.1).
mod grounding;

/// The writer half of the baseline: the campaign's fact-grain verdicts
/// projected onto §7.3's unit-grain record (DRIFT-023).
mod baseline;

/// The rescan half — the only part of the adapter that knows this tree is a
/// git checkout (PROP-043 §7.3).
mod rescan;

/// The seal verb: recording that a file's verdicts hold for its current
/// text, so a hand-sealed campaign stops reading as stale (DRIFT-026).
mod seal;

// Re-exported under their own names so every caller — the verbs below, the
// two verb submodules, the tests — reaches them exactly as before.
use grounding::{Ground, campaign_id, ground, refresh_state, resolve_campaign};

pub fn run(ctx: &Context, args: ProgressArgs) -> Result<()> {
    match args.command {
        ProgressSubcommand::Scan(a) => scan(ctx, &a),
        ProgressSubcommand::Check(a) => check(ctx, &a),
        ProgressSubcommand::Report(a) => report_cmd(ctx, &a),
        ProgressSubcommand::Mirror(a) => mirror(ctx, &a),
        ProgressSubcommand::Weave(a) => weave_cmd(ctx, &a),
        ProgressSubcommand::Rescan(a) => rescan::rescan_cmd(ctx, &a),
        ProgressSubcommand::Baseline(a) => baseline::baseline_cmd(ctx, &a),
        ProgressSubcommand::Resume(a) => resume(ctx, &a),
        ProgressSubcommand::Gate(a) => gate(ctx, &a),
        ProgressSubcommand::Seal(a) => seal::seal_cmd(ctx, &a),
    }
}

fn scan(ctx: &Context, a: &ProgressCommonArgs) -> Result<()> {
    let mut g = ground(a)?;
    let refreshed = refresh_state(&mut g)?;
    let markers: usize = g.docs.iter().map(|d| d.markers.len()).sum();
    let facts: usize = g.docs.iter().map(|d| d.fact_count).sum();
    let unmarked: usize = g.docs.iter().map(|d| d.unmarked_facts.len()).sum();
    let errors: usize = g.docs.iter().map(|d| d.error_count()).sum();
    let (wrote, skipped) = refreshed.tally();
    if ctx.is_json() {
        println!(
            "{}",
            serde_json::json!({
                "files": g.docs.len(),
                "markers": markers,
                "facts": facts,
                "unmarked": unmarked,
                "errors": errors,
                "excluded": g.excluded,
                // Whether there was a campaign zone to write into at all —
                // not whether anything moved, which is `written` below.
                "state_written": refreshed.campaign.is_some(),
                "written": refreshed.writes,
                "skipped": skipped,
            })
        );
    } else {
        println!(
            "progress scan: {} files, {markers} markers, {unmarked}/{facts} facts unmarked, {errors} errors",
            g.docs.len()
        );
        if g.excluded > 0 {
            println!("  {} file(s) dropped by the `exclude` globs", g.excluded);
        }
        match &refreshed.campaign {
            Some(c) => println!(
                "  state refreshed under {} — {wrote} written, {skipped} unchanged and skipped",
                c.join("run").display()
            ),
            None => println!("  (no campaign zone — state projections not written)"),
        }
    }
    Ok(())
}

fn check(ctx: &Context, a: &ProgressCheckArgs) -> Result<()> {
    let mut g = ground(&a.common)?;
    let mut errors = 0usize;
    let mut warnings = 0usize;
    for doc in &g.docs {
        for i in &doc.issues {
            match i.severity {
                Severity::Error => errors += 1,
                Severity::Warning => warnings += 1,
            }
            if !ctx.is_quiet() {
                println!(
                    "{}:{}: {:?} [{:?}] {}",
                    doc.path, i.line, i.severity, i.code, i.message
                );
            }
        }
        // Lossless folds (PROP-043 §3.9 `POST-CAMPAIGN-FOLD`): a section
        // marker that collapses agreeing units must carry everything they
        // carried. Reported at **warning** severity, not error: a document
        // cannot distinguish a lying fold from the deliberate explicit
        // marker `#rollup`'s `EXPLICIT-BEATS` blesses ("a divergence is
        // information, not noise"), so this surfaces the case without
        // failing a gate on legitimate markup. Phase F's folder, which knows
        // it is asserting a fold, runs `fold_check` as a fatal pre-flight.
        for f in rollup::fold_check(doc) {
            warnings += 1;
            if !ctx.is_quiet() {
                println!("{}:{}: Warning [FoldLossy] {f}", doc.path, f.line);
            }
        }
        if a.exhaustive {
            for &(bi, fi) in &doc.unmarked_facts {
                errors += 1;
                if !ctx.is_quiet() {
                    let f = &doc.blocks[bi].facts[fi];
                    println!(
                        "{}:{}: Error [unmarked] {:?} unit carries no marker (--exhaustive)",
                        doc.path, f.line, f.kind
                    );
                }
            }
        }
    }
    refresh_state(&mut g)?;
    if errors > 0 {
        bail!("progress check: {errors} error(s), {warnings} warning(s)");
    }
    if !ctx.is_quiet() {
        println!(
            "progress check: clean ({} files, {warnings} warning(s))",
            g.docs.len()
        );
    }
    Ok(())
}

fn parse_view(s: Option<&str>) -> Result<Option<View>> {
    match s {
        None => Ok(None),
        Some(v) => match View::parse(v) {
            Some(view) => Ok(Some(view)),
            None => bail!("unknown --view `{v}` (expected done|todo|qa|remove|doc)"),
        },
    }
}

fn parse_audience(s: Option<&str>) -> Result<Option<Audience>> {
    match s {
        None => Ok(None),
        Some(v) => match Audience::parse(v) {
            Some(a) => Ok(Some(a)),
            None => bail!("unknown --audience `{v}` (expected user|author|dev)"),
        },
    }
}

/// The rendered report, exactly as `report_cmd` prints it.
///
/// Split out from the printing so the warm/cold equality bar (DRIFT-010
/// §4.4) can be *asserted* on the bytes a user would see, rather than on a
/// proxy for them.
fn report_body(g: &Ground, a: &ProgressReportArgs, json: bool) -> Result<String> {
    let view = parse_view(a.view.as_deref())?;
    let audience = parse_audience(a.audience.as_deref())?;
    // The evidence column is wired only where the index exists. A project
    // without `specmap.json` reports exactly as before and says nothing
    // about it — a missing index is not an error (PROP-043 §6).
    let specmap = super::progress_evidence::SpecmapEvidence::load(&g.root)?;
    let provider: &dyn EvidenceProvider = match &specmap {
        Some(s) => s,
        None => &NoEvidence,
    };
    let rows = report::rows(g.docs.iter(), view, audience, provider);
    let rollups: Vec<(String, rollup::DocRollup)> = g
        .docs
        .iter()
        .map(|d| (d.path.clone(), rollup::rollup_doc(d)))
        .collect();
    Ok(if json {
        format!("{}\n", serde_json::to_string_pretty(&rows)?)
    } else if a.md {
        report::render_md(&rows, &rollups)
    } else {
        report::render_xml(&rows, &rollups)
    })
}

fn report_cmd(ctx: &Context, a: &ProgressReportArgs) -> Result<()> {
    let g = ground(&a.common)?;
    print!("{}", report_body(&g, a, ctx.is_json())?);
    Ok(())
}

fn mirror(ctx: &Context, a: &ProgressCommonArgs) -> Result<()> {
    let mut g = ground(a)?;
    let Some(campaign) = &g.campaign else {
        bail!("`vibe progress mirror` needs a campaign zone (campaigns/<id>/ or --campaign)");
    };
    let dir = campaign.join("run").join("mirror");
    for doc in &g.docs {
        let rel = doc.path.replace('/', "__");
        let body = serde_json::to_string_pretty(doc)?;
        cache::write_atomic(&dir.join(format!("{rel}.json")), body.as_bytes())?;
    }
    refresh_state(&mut g)?;
    if !ctx.is_quiet() {
        println!(
            "progress mirror: {} per-file views under {}",
            g.docs.len(),
            dir.display()
        );
    }
    Ok(())
}

fn weave_cmd(ctx: &Context, a: &ProgressWeaveArgs) -> Result<()> {
    let g = ground(&a.common)?;
    if a.digest {
        let body = weave::weave_digest(g.docs.iter());
        return emit_weave(ctx, &[(0usize, body)], a.out.as_deref(), "digest");
    }
    let files: Vec<(String, String)> = g
        .docs
        .iter()
        .map(|d| -> Result<(String, String)> {
            let text = std::fs::read_to_string(g.root.join(&d.path))
                .with_context(|| format!("re-reading {}", d.path))?;
            Ok((d.path.clone(), text))
        })
        .collect::<Result<_>>()?;
    let shards = weave::weave_full(&files, a.max_tokens);
    let bodies: Vec<(usize, String)> = shards.into_iter().map(|s| (s.index, s.body)).collect();
    emit_weave(ctx, &bodies, a.out.as_deref(), "weave")
}

fn emit_weave(
    ctx: &Context,
    shards: &[(usize, String)],
    out: Option<&Path>,
    stem: &str,
) -> Result<()> {
    match out {
        None => {
            for (_, body) in shards {
                println!("{body}");
            }
        }
        Some(dir) => {
            for (i, body) in shards {
                cache::write_atomic(&dir.join(format!("{stem}-{i:03}.md")), body.as_bytes())?;
            }
            if !ctx.is_quiet() {
                println!(
                    "progress weave: {} shard(s) under {}",
                    shards.len(),
                    dir.display()
                );
            }
        }
    }
    Ok(())
}

fn resume(ctx: &Context, a: &ProgressCommonArgs) -> Result<()> {
    let mut g = ground(a)?;
    let Some(campaign) = &g.campaign else {
        bail!("`vibe progress resume` needs a campaign zone (campaigns/<id>/ or --campaign)");
    };
    let run_dir = campaign.join("run");
    let events = journal::read_journal(&run_dir.join("journal.jsonl"))?;
    let open = journal::open_steps(&events);
    let facts: usize = g.docs.iter().map(|d| d.fact_count).sum();
    let unmarked: usize = g.docs.iter().map(|d| d.unmarked_facts.len()).sum();
    let counters = serde_json::json!({
        "files": g.docs.len(),
        "facts": facts,
        "unmarked": unmarked,
        "journal_events": events.len(),
    });
    let next_hint = if events.is_empty() {
        "The campaign journal is empty — the campaign has not started. Next per the \
         plan: Phase A exit gate, then Phase B batch B0 (convert the legacy \
         `**Status:**` lines), then B1… (paragraph-exhaustive markup batches)."
    } else {
        "Take the first unfinished step above; when none remain, continue with the \
         next queued batch/wave/task per the plan's LOG section."
    };
    // Same journal-derived phase the state projection carries.
    let phase = journal::derive_phase(&events);
    let body = journal::render_resume(&campaign_id(campaign), &phase, &counters, &open, next_hint);
    journal::write_resume(&run_dir.join("RESUME.md"), &body)?;
    refresh_state(&mut g)?;
    if !ctx.is_quiet() {
        print!("{body}");
    }
    Ok(())
}

/// `vibe progress gate <name> --status <green|red|stale> [--detail …]` —
/// record a gate's verdict into the campaign's panel.
///
/// The adapter never *runs* the gate: a CI step or a local script runs the
/// real thing and reports the verdict here (PROP-043 §2 — nothing in this
/// stack measures a floor). Deliberately skips the tree parse `ground`
/// performs; reporting a verdict touches the campaign zone only.
fn gate(ctx: &Context, a: &ProgressGateArgs) -> Result<()> {
    let root = a
        .common
        .path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", a.common.path.display()))?;
    let root = super::init::strip_unc_public(root);
    let Some(campaign) = resolve_campaign(&root, a.common.campaign.as_deref()) else {
        bail!("`vibe progress gate` needs a campaign zone (campaigns/<id>/ or --campaign)");
    };
    let (status, label) = match a.status {
        GateStatusArg::Green => (state::GateStatus::Green, "green"),
        GateStatusArg::Red => (state::GateStatus::Red, "red"),
        GateStatusArg::Stale => (state::GateStatus::Stale, "stale"),
    };
    let record = state::GateRecord {
        name: a.name.clone(),
        status,
        ran_at: cache::now_utc(),
        detail: a.detail.clone(),
    };
    let payload = serde_json::to_string_pretty(&record)?;
    let state_dir = campaign.join("run").join("state");
    state::record_gate(&state_dir, record)?;
    if ctx.is_json() {
        println!("{payload}");
    } else if !ctx.is_quiet() {
        println!(
            "progress gate: `{}` = {label} recorded in {}",
            a.name,
            state_dir.join("campaign.json").display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests;
