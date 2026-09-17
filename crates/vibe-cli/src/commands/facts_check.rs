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
    let grounding = ctx.progress().task("Grounding facts corpus");
    let mut grounded = match ground(&args.common) {
        Ok(grounded) => {
            grounding.finish();
            grounded
        }
        Err(error) => {
            grounding.fail("facts grounding failed");
            return Err(error);
        }
    };
    let validate = ctx.progress().task("Validating facts markup");
    validate.set_progress(0, Some(grounded.docs.len() as u64), "files");
    let mut errors = 0usize;
    let mut warnings = 0usize;
    for (index, doc) in grounded.docs.iter().enumerate() {
        // Diagnostics are the command's result stream. Buffer one document's
        // lines and suspend the renderer once so an active finite task cannot
        // redraw across stdout (and redirected stdout remains byte-for-byte
        // the same stream as before progress existed).
        let mut output = Vec::new();
        // PROP-045 ##PROJECTION-READ: an XML-sourced document's diagnostics
        // cite projection-relative lines. The path repeats on every issue
        // line here, so the notice rides once per document — the header
        // form of the mark, not a suffix on each line.
        let folds = rollup::fold_check(doc);
        if grounded.xml_sources.contains(&doc.path)
            && !ctx.is_quiet()
            && (!doc.issues.is_empty() || !folds.is_empty() || args.exhaustive)
        {
            output.push(projection_header(&doc.path));
        }
        for issue in &doc.issues {
            match issue.severity {
                Severity::Error => errors += 1,
                Severity::Warning => warnings += 1,
            }
            if !ctx.is_quiet() {
                output.push(format!(
                    "{}:{}: {:?} [{:?}] {}",
                    doc.path, issue.line, issue.severity, issue.code, issue.message
                ));
            }
        }
        // Ordinary explicit rollup divergence is advisory. Fact-owned
        // terminal requirements have no section carrier, so folding them is
        // an irreversible loss and therefore an error.
        for fold in folds {
            let fatal = fold.lost == rollup::FoldLoss::Requirements;
            if fatal {
                errors += 1;
            } else {
                warnings += 1;
            }
            if !ctx.is_quiet() {
                let severity = if fatal { "Error" } else { "Warning" };
                output.push(format!(
                    "{}:{}: {severity} [FoldLossy] {fold}",
                    doc.path, fold.line
                ));
            }
        }
        // `--exhaustive` asks every prose unit for a marker, which is the
        // same demand as a verdict one step earlier — so an exempt file
        // is exempt from it too (PROP-057 `##OBS-NOT-JUDGED`). Without
        // this the third answer would only be half an answer: the
        // manual would stay out of the judging debt and still fail the
        // gate that counts the debt's raw material, and «observed, never
        // judged» would be unusable for the genre it was written for.
        let exempt = grounded
            .judging_exemption
            .covers(std::path::Path::new(&doc.path));
        if args.exhaustive && !exempt {
            for &(block_index, fact_index) in &doc.unmarked_facts {
                errors += 1;
                if !ctx.is_quiet() {
                    let fact = &doc.blocks[block_index].facts[fact_index];
                    output.push(format!(
                        "{}:{}: Error [unmarked] {:?} unit carries no marker (--exhaustive)",
                        doc.path, fact.line, fact.kind
                    ));
                }
            }
        }
        if !output.is_empty() {
            ctx.suspend_progress(|| {
                for line in output {
                    println!("{line}");
                }
            });
        }
        validate.set_progress(
            (index + 1) as u64,
            Some(grounded.docs.len() as u64),
            "files",
        );
    }
    validate.finish();
    // `check` remains read-only by default. The compatibility flag keeps
    // the old opt-in write tail byte-for-byte for both spellings.
    if args.write_state {
        let write = ctx.progress().task("Writing facts state");
        match refresh_state(&mut grounded) {
            Ok(_) => write.finish(),
            Err(error) => {
                write.fail("facts state write failed");
                return Err(error);
            }
        }
    }
    if errors > 0 {
        bail!("progress check: {errors} error(s), {warnings} warning(s)");
    }
    if !ctx.is_quiet() {
        ctx.suspend_progress(|| {
            println!(
                "progress check: clean ({} files, {warnings} warning(s))",
                grounded.docs.len()
            );
        });
    }
    Ok(())
}

pub(crate) fn projection_header(path: &str) -> String {
    format!("{path}: {}", vibe_specdoc::PROJECTION_NOTICE)
}
