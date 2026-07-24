//! `vibe progress` — the vibevm adapter over `progress-core`
//! (PROP-043 §5). All markup knowledge lives in the core; this file only
//! resolves paths, wires the campaign zone, and renders to the terminal.

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#tool");

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use progress_core::doc::{ParsedDoc, Severity};
use progress_core::model::Audience;
use progress_core::report::View;
use progress_core::{baseline, cache, journal, report, rollup, scope, state, weave};

use crate::cli::{
    ProgressArgs, ProgressCheckArgs, ProgressCommonArgs, ProgressReportArgs, ProgressRescanArgs,
    ProgressSubcommand, ProgressWeaveArgs,
};
use crate::output::Context;

pub fn run(ctx: &Context, args: ProgressArgs) -> Result<()> {
    match args.command {
        ProgressSubcommand::Scan(a) => scan(ctx, &a),
        ProgressSubcommand::Check(a) => check(ctx, &a),
        ProgressSubcommand::Report(a) => report_cmd(ctx, &a),
        ProgressSubcommand::Mirror(a) => mirror(ctx, &a),
        ProgressSubcommand::Weave(a) => weave_cmd(ctx, &a),
        ProgressSubcommand::Rescan(a) => rescan_cmd(ctx, &a),
        ProgressSubcommand::Resume(a) => resume(ctx, &a),
    }
}

/// The observed tree + campaign zone, resolved once per invocation.
struct Ground {
    root: PathBuf,
    docs: Vec<ParsedDoc>,
    campaign: Option<PathBuf>,
}

fn ground(common: &ProgressCommonArgs) -> Result<Ground> {
    let root = common
        .path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", common.path.display()))?;
    let root = super::init::strip_unc_public(root);
    let cfg = scope::load_config(&root)?;
    // Parse without touching any cache: pure read of the tree.
    let files = scope::observed_files(&root, &cfg)?;
    let mut docs = Vec::new();
    for rel in files {
        let full = root.join(&rel);
        let text = std::fs::read_to_string(&full)
            .with_context(|| format!("reading {}", full.display()))?;
        docs.push(progress_core::parse::parse_document(
            &scope::rel_str(&rel),
            &text,
        ));
    }
    let campaign = resolve_campaign(&root, common.campaign.as_deref());
    Ok(Ground {
        root,
        docs,
        campaign,
    })
}

/// `--campaign` wins; otherwise the single `campaigns/<id>/` when exactly
/// one exists; otherwise none (ad-hoc mode — reports work, state does not).
fn resolve_campaign(root: &Path, flag: Option<&Path>) -> Option<PathBuf> {
    if let Some(f) = flag {
        return Some(if f.is_absolute() {
            f.to_path_buf()
        } else {
            root.join(f)
        });
    }
    let zone = root.join("campaigns");
    let entries: Vec<PathBuf> = std::fs::read_dir(&zone)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    match entries.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}

fn campaign_id(campaign: &Path) -> String {
    campaign
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "campaign".into())
}

/// Refresh cache + state under the campaign zone from parsed docs.
fn refresh_state(g: &Ground) -> Result<Option<PathBuf>> {
    let Some(campaign) = &g.campaign else {
        return Ok(None);
    };
    let run_dir = campaign.join("run");
    let cache_path = run_dir.join("cache.json");
    let (mut c, recovered) = cache::Cache::load_tolerant(&cache_path);
    if let Some(warning) = recovered {
        eprintln!("vibe progress: warning: {warning}");
    }
    for doc in &g.docs {
        let r = rollup::rollup_doc(doc);
        c.upsert(doc, &r);
    }
    c.touch();
    c.store(&cache_path)?;
    // Phase is derived from the campaign's own journal (last `phase` event
    // wins; absent ⇒ "A") — never compiled in, never parsed from Markdown.
    let phase = journal::derive_phase(&journal::read_journal(&run_dir.join("journal.jsonl"))?);
    state::write_state(&run_dir.join("state"), &campaign_id(campaign), &phase, &c)?;
    Ok(Some(campaign.clone()))
}

fn scan(ctx: &Context, a: &ProgressCommonArgs) -> Result<()> {
    let g = ground(a)?;
    let wrote = refresh_state(&g)?;
    let markers: usize = g.docs.iter().map(|d| d.markers.len()).sum();
    let facts: usize = g.docs.iter().map(|d| d.fact_count).sum();
    let unmarked: usize = g.docs.iter().map(|d| d.unmarked_facts.len()).sum();
    let errors: usize = g.docs.iter().map(|d| d.error_count()).sum();
    if ctx.is_json() {
        println!(
            "{}",
            serde_json::json!({
                "files": g.docs.len(),
                "markers": markers,
                "facts": facts,
                "unmarked": unmarked,
                "errors": errors,
                "state_written": wrote.is_some(),
            })
        );
    } else {
        println!(
            "progress scan: {} files, {markers} markers, {unmarked}/{facts} facts unmarked, {errors} errors",
            g.docs.len()
        );
        match wrote {
            Some(c) => println!("  state refreshed under {}", c.join("run").display()),
            None => println!("  (no campaign zone — state projections not written)"),
        }
    }
    Ok(())
}

fn check(ctx: &Context, a: &ProgressCheckArgs) -> Result<()> {
    let g = ground(&a.common)?;
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
    refresh_state(&g)?;
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

fn report_cmd(ctx: &Context, a: &ProgressReportArgs) -> Result<()> {
    let g = ground(&a.common)?;
    let view = parse_view(a.view.as_deref())?;
    let audience = parse_audience(a.audience.as_deref())?;
    let rows = report::rows(g.docs.iter(), view, audience);
    let rollups: Vec<(String, rollup::DocRollup)> = g
        .docs
        .iter()
        .map(|d| (d.path.clone(), rollup::rollup_doc(d)))
        .collect();
    if ctx.is_json() {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else if a.md {
        print!("{}", report::render_md(&rows, &rollups));
    } else {
        print!("{}", report::render_xml(&rows, &rollups));
    }
    Ok(())
}

fn mirror(ctx: &Context, a: &ProgressCommonArgs) -> Result<()> {
    let g = ground(a)?;
    let Some(campaign) = &g.campaign else {
        bail!("`vibe progress mirror` needs a campaign zone (campaigns/<id>/ or --campaign)");
    };
    let dir = campaign.join("run").join("mirror");
    for doc in &g.docs {
        let rel = doc.path.replace('/', "__");
        let body = serde_json::to_string_pretty(doc)?;
        cache::write_atomic(&dir.join(format!("{rel}.json")), body.as_bytes())?;
    }
    refresh_state(&g)?;
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

fn rescan_cmd(ctx: &Context, a: &ProgressRescanArgs) -> Result<()> {
    let g = ground(&a.common)?;
    let base = baseline::Baseline::load(&a.baseline)?;
    let rows = baseline::rescan(g.docs.iter(), &base);
    if ctx.is_json() {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }
    let count = |c: &baseline::RescanClass| rows.iter().filter(|r| r.class == *c).count();
    println!(
        "progress rescan vs {}: {} new, {} changed (suspect), {} carried-forward",
        a.baseline.display(),
        count(&baseline::RescanClass::New),
        count(&baseline::RescanClass::Changed),
        count(&baseline::RescanClass::CarriedForward),
    );
    for r in &rows {
        match r.class {
            baseline::RescanClass::CarriedForward if !r.marker_diverged => {}
            _ => println!(
                "  {:?} {}{}",
                r.class,
                r.addr,
                if r.marker_diverged {
                    "  [marker changed outside a campaign]"
                } else {
                    ""
                }
            ),
        }
    }
    Ok(())
}

fn resume(ctx: &Context, a: &ProgressCommonArgs) -> Result<()> {
    let g = ground(a)?;
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
    refresh_state(&g)?;
    if !ctx.is_quiet() {
        print!("{body}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixture campaign zone whose journal carries a hand-appended `phase`
    /// event: `refresh_state` must derive that phase into `campaign.json`
    /// instead of the compiled-in opening phase (DRIFT-003 §4).
    #[test]
    fn refresh_state_derives_phase_from_journal() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let campaign = tmp.path().join("campaigns").join("progress-test");
        let run = campaign.join("run");
        std::fs::create_dir_all(&run).expect("mkdir run");
        // The exact on-disk event the campaign executor appends by hand.
        std::fs::write(
            run.join("journal.jsonl"),
            "{\"kind\":\"phase\",\"value\":\"B\",\"ts\":\"2026-07-24T00:00:00Z\"}\n",
        )
        .expect("write journal fixture");

        let g = Ground {
            root: tmp.path().to_path_buf(),
            docs: Vec::new(),
            campaign: Some(campaign.clone()),
        };
        refresh_state(&g).expect("refresh_state");

        let text = std::fs::read_to_string(run.join("state").join("campaign.json"))
            .expect("read campaign.json");
        let v: serde_json::Value = serde_json::from_str(&text).expect("parse campaign.json");
        assert_eq!(
            v["phase"], "B",
            "campaign.json carries the journal-derived phase"
        );
    }
}
