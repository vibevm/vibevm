//! What `vibe search` prints for a person — the registry status lines,
//! the hit table, the issued-short-name block and the one-line summary.
//!
//! Split out of the command so that the walk (which registries answered,
//! what each contributed) and the rendering (how the one answer reads)
//! are separate cells. It is a pure function of the report the command
//! already built for `--json`, so the two modes cannot drift: anything
//! the human output says is a member of `SearchReport`.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use anyhow::Result;

use super::report::SearchReport;
use super::support::truncate;
use crate::output;

/// Print the full report. `full_scan` is the flag the run was given,
/// which decides whether the registry line mentions the org-walk at all
/// — the only fact here that is not in the report itself.
pub(super) fn render(ctx: &output::Context, report: &SearchReport, full_scan: bool) -> Result<()> {
    let SearchReport {
        query,
        registries_searched: searched,
        registries_unconfigured: unconfigured,
        registries_unreachable: unreachable,
        registries_policy_skipped: policy_skipped,
        registries_full_scanned: full_scanned,
        registries_full_scan_unsupported: full_scan_unsupported,
        hits,
        ..
    } = report;
    println!("query     : {query}");
    println!(
        "registries: {} searched, {} unreachable, {} without index URL{}",
        searched.len(),
        unreachable.len(),
        unconfigured.len(),
        if full_scan {
            format!(
                ", {} full-scanned, {} full-scan unsupported",
                full_scanned.len(),
                full_scan_unsupported.len()
            )
        } else {
            String::new()
        },
    );
    if !searched.is_empty() {
        println!("  searched: {}", searched.join(", "));
    }
    if !full_scanned.is_empty() {
        println!("  full-scanned: {}", full_scanned.join(", "));
    }
    for u in unreachable {
        println!("  unreachable: {} — {}", u.name, u.reason);
    }
    for skipped in policy_skipped {
        println!(
            "  policy skipped {} (availability unknown): {}",
            skipped.name, skipped.reason
        );
    }
    for u in full_scan_unsupported {
        println!("  full-scan unsupported: {} — {}", u.name, u.reason);
    }
    if !unconfigured.is_empty() {
        println!("  no VIBEVM_INDEX_URL_<R> set: {}", unconfigured.join(", "),);
    }
    println!();

    if hits.is_empty() {
        if searched.is_empty() {
            ctx.summary(
                "(no registry has VIBEVM_INDEX_URL_<R> configured; search returns empty.\n\
                 To install a known package, run `vibe install <group>/<name>` directly —\n\
                 install resolves through `[[registry]]` over git and does not need an\n\
                 index. The index is a discovery optimisation, not a runtime dependency.\n\
                 See docs/commands/search.md for setting up an index server.)",
            );
        } else {
            ctx.summary("(no matches)");
        }
        return Ok(());
    }

    println!(
        "KIND    NAME                          LATEST       SCORE  SOURCE      REGISTRY              DESCRIPTION"
    );
    for h in hits {
        let latest = h.latest_stable.as_deref().unwrap_or("-");
        let desc = h.description.as_deref().unwrap_or("");
        println!(
            "{:<6}  {:<28}  {:<11}  {:<5}  {:<10}  {:<20}  {}",
            h.kind,
            truncate(&h.name, 28),
            latest,
            h.score,
            h.source,
            truncate(&h.registry, 20),
            truncate(desc, 60),
        );
    }
    // The spellings an operator may actually type bare. Since the
    // owner's 2026-11-12 ruling a package's own name is not one of
    // them unless a registry issued it, so a hit list that did not say
    // which names are issued would read as an install menu that half
    // of it is not (PROP-008 §2.6 `##ISSUED-NAMES-ONLY`).
    let issued_lines: Vec<String> = hits
        .iter()
        .filter(|hit| !hit.short_names.is_empty())
        .map(|hit| format!("{} → {}", hit.short_names.join(", "), hit.name))
        .collect();
    if !issued_lines.is_empty() {
        println!();
        println!("short names these registries issue (type these bare; every other hit needs its");
        println!("qualified `<group>/<name>`):");
        for line in &issued_lines {
            println!("  {line}");
        }
    }
    println!();
    ctx.summary(&format!(
        "{} hit{} across {} registr{}",
        hits.len(),
        if hits.len() == 1 { "" } else { "s" },
        searched.len(),
        if searched.len() == 1 { "y" } else { "ies" },
    ));
    Ok(())
}
