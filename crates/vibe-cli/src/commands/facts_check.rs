//! Shared markup-lint implementation for `vibe facts check` and its
//! transitional `vibe progress check` alias.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#BOUNDARY-CLI");

use anyhow::{Result, bail};
use progress_core::doc::Severity;
use progress_core::rollup;

use crate::cli::ProgressCheckArgs;
use crate::output::Context;

use super::progress::grounding::{ground, refresh_state};

pub(crate) fn run(ctx: &Context, args: &ProgressCheckArgs) -> Result<()> {
    let mut grounded = ground(&args.common)?;
    let mut errors = 0usize;
    let mut warnings = 0usize;
    for doc in &grounded.docs {
        // PROP-045 ##PROJECTION-READ: an XML-sourced document's diagnostics
        // cite projection-relative lines. The path repeats on every issue
        // line here, so the notice rides once per document — the header
        // form of the mark, not a suffix on each line.
        let folds = rollup::fold_check(doc);
        if grounded.xml_sources.contains(&doc.path)
            && !ctx.is_quiet()
            && (!doc.issues.is_empty() || !folds.is_empty() || args.exhaustive)
        {
            println!("{}", projection_header(&doc.path));
        }
        for issue in &doc.issues {
            match issue.severity {
                Severity::Error => errors += 1,
                Severity::Warning => warnings += 1,
            }
            if !ctx.is_quiet() {
                println!(
                    "{}:{}: {:?} [{:?}] {}",
                    doc.path, issue.line, issue.severity, issue.code, issue.message
                );
            }
        }
        // Lossless folds (PROP-043 §3.9 `POST-CAMPAIGN-FOLD`): a section
        // marker that collapses agreeing units must carry everything they
        // carried. Reported at warning severity because an explicit `#rollup`
        // may deliberately bless the divergence.
        for fold in folds {
            warnings += 1;
            if !ctx.is_quiet() {
                println!("{}:{}: Warning [FoldLossy] {fold}", doc.path, fold.line);
            }
        }
        if args.exhaustive {
            for &(block_index, fact_index) in &doc.unmarked_facts {
                errors += 1;
                if !ctx.is_quiet() {
                    let fact = &doc.blocks[block_index].facts[fact_index];
                    println!(
                        "{}:{}: Error [unmarked] {:?} unit carries no marker (--exhaustive)",
                        doc.path, fact.line, fact.kind
                    );
                }
            }
        }
    }
    // `check` remains read-only by default. The compatibility flag keeps
    // the old opt-in write tail byte-for-byte for both spellings.
    if args.write_state {
        refresh_state(&mut grounded)?;
    }
    if errors > 0 {
        bail!("progress check: {errors} error(s), {warnings} warning(s)");
    }
    if !ctx.is_quiet() {
        println!(
            "progress check: clean ({} files, {warnings} warning(s))",
            grounded.docs.len()
        );
    }
    Ok(())
}

pub(crate) fn projection_header(path: &str) -> String {
    format!("{path}: {}", vibe_specdoc::PROJECTION_NOTICE)
}
