//! `vibe progress seal <path>…` — record that a file's verdicts were
//! re-derived against its current text (PROP-043 §5, DRIFT-026).
//!
//! The same shape as [`gate`](super::gate) and for the same reason: the
//! caller did the real work and reports the result, so this adapter runs
//! no verification, computes no verdict and changes none. What lives here
//! is path resolution, the store, and the sentence a human reads before
//! three hundred verdicts are vouched for in one line of a diff.
//!
//! Deliberately skips the tree parse [`ground`](super::ground) performs,
//! exactly as `gate` does: this touches the named files and the campaign
//! zone, and nothing else. That is also what makes the digest honest —
//! every file it seals is read and parsed *here*, so the number recorded
//! is the one the bytes on disk produce and never the one the cache was
//! already carrying (DRIFT-026 §4.1).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#CMD-SEAL");

use std::path::Path;

use anyhow::{Context as _, Result, bail};
use progress_core::seal::{Seal, SealClaim};
use progress_core::{cache, parse, scope, seal};

use crate::cli::ProgressSealArgs;
use crate::output::Context;

/// How many unjudged anchors a refusal names before it stops listing.
const NAMED: usize = 5;

/// Seal each named path against the campaign cache, then store it once.
pub fn seal_cmd(ctx: &Context, a: &ProgressSealArgs) -> Result<()> {
    let root = a
        .common
        .path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", a.common.path.display()))?;
    let root = super::super::init::strip_unc_public(root);
    let Some(campaign) = super::resolve_campaign(&root, a.common.campaign.as_deref()) else {
        bail!("`vibe progress seal` needs a campaign zone (campaigns/<id>/ or --campaign)");
    };
    let cache_path = campaign.join("run").join("cache.json");
    // A cache that failed to load is not a campaign with no verdicts, and
    // the difference is fatal exactly here: every other subcommand
    // degrades to a cold run, while this one would write the *empty*
    // cache back over the file that holds every verdict the campaign has.
    let (mut c, warning) = cache::Cache::load_tolerant(&cache_path);
    if let Some(warning) = warning {
        bail!(
            "refusing to seal against an unreadable cache ({warning}); restore `{}` \
             (it is tracked in git) and re-run",
            cache_path.display()
        );
    }

    let stamp = cache::now_utc();
    let mut refused = 0usize;
    let mut recorded = 0usize;
    for path in &a.paths {
        match one(ctx, &root, &mut c, path, &stamp) {
            Ok(Outcome::Recorded) => recorded += 1,
            // Nothing to seal is not a failure: re-sealing is a no-op
            // that says so and leaves the exit code alone (§4.2).
            Ok(Outcome::Current) => {}
            Ok(Outcome::Refused) => refused += 1,
            // Named, never skipped, and never fatal to the paths after it
            // — the run is per-file by contract (§4.2 edge cases).
            Err(e) => {
                refused += 1;
                eprintln!("vibe progress: error: {e:#}");
            }
        }
    }
    if recorded > 0 {
        c.touch();
        c.store(&cache_path)?;
    }
    if !ctx.is_quiet() && !ctx.is_json() {
        println!(
            "progress seal: {recorded} sealed, {refused} refused, {} already current → {}",
            a.paths.len() - recorded - refused,
            cache_path.display()
        );
    }
    if refused > 0 {
        bail!("progress seal: {refused} path(s) refused");
    }
    Ok(())
}

/// What one path came to. Only [`Outcome::Refused`] reaches the exit
/// code: a file that was already sealed had nothing to record, which is
/// the answer the operator asked for rather than a failure to give it.
enum Outcome {
    Recorded,
    Current,
    Refused,
}

/// Seal one path, saying what it decided.
fn one(
    ctx: &Context,
    root: &Path,
    c: &mut cache::Cache,
    path: &Path,
    stamp: &str,
) -> Result<Outcome> {
    let rel = relative(root, path)?;
    let full = root.join(&rel);
    let text = std::fs::read_to_string(&full)
        .with_context(|| format!("reading {} to seal it", full.display()))?;
    // Parsed here, from the bytes just read: `content_hash` is the digest
    // of what is on disk, which is the whole point of the verb.
    let doc = parse::parse_document(&rel, &text);
    match seal::seal(c, &doc, stamp) {
        Seal::Recorded(claim) => {
            say(ctx, &rel, &claim, "sealed", Some(stamp));
            Ok(Outcome::Recorded)
        }
        // No stamp on this line, deliberately: the date on the record is
        // the one the *first* seal wrote, and printing this run's clock
        // beside "already sealed" reads exactly like the fresh timestamp
        // §4.2 forbids.
        Seal::Current(claim) => {
            say(
                ctx,
                &rel,
                &claim,
                "already sealed against these bytes — nothing recorded",
                None,
            );
            Ok(Outcome::Current)
        }
        Seal::Unjudged { judged, anchors } => {
            let named: Vec<&str> = anchors.iter().take(NAMED).map(String::as_str).collect();
            let more = anchors.len().saturating_sub(named.len());
            eprintln!(
                "vibe progress: refusing to seal `{rel}` — {} of its markers carry no verdict \
                 ({judged} judged): {}{}. Sealing asserts that *every* verdict in a file is \
                 valid for its current text, so re-verify the whole file (or leave it flagged) \
                 rather than sealing the part that was checked",
                anchors.len(),
                named.join(", "),
                if more > 0 {
                    format!(", and {more} more")
                } else {
                    String::new()
                },
            );
            Ok(Outcome::Refused)
        }
        Seal::Unobserved => {
            eprintln!(
                "vibe progress: refusing to seal `{rel}` — the campaign cache carries no record \
                 for it (never observed, or outside the observed scope); there are no verdicts \
                 here for a seal to speak for"
            );
            Ok(Outcome::Refused)
        }
    }
}

/// What is being vouched for, said before it is written.
fn say(ctx: &Context, rel: &str, claim: &SealClaim, verb: &str, stamp: Option<&str>) {
    if ctx.is_json() {
        println!(
            "{}",
            serde_json::json!({
                "path": rel,
                "outcome": verb,
                "verdicts": claim.verdicts,
                "was": claim.was,
                "now": claim.now,
                "verified_at": stamp,
            })
        );
        return;
    }
    if ctx.is_quiet() {
        return;
    }
    println!(
        "progress seal: `{rel}` — vouching for {} verdict(s) against the text on disk",
        claim.verdicts
    );
    println!(
        "  {} → {}",
        claim.was.as_deref().map_or("(never processed)", short),
        short(&claim.now),
    );
    match stamp {
        Some(at) => println!("  {verb} at {at}"),
        None => println!("  {verb}"),
    }
}

/// A digest, shortened to the twelve characters a human compares.
fn short(hash: &str) -> &str {
    hash.get(..12).unwrap_or(hash)
}

/// The cache key for a path the operator typed: repo-relative to the
/// observed root, `/`-separated, exactly as `scope` mints it.
///
/// A path outside the root is refused here rather than looked up and
/// missed, so the message names the actual problem instead of reporting
/// it as a file the campaign never observed.
fn relative(root: &Path, path: &Path) -> Result<String> {
    let full = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let full = super::super::init::strip_unc_public(full);
    let rel = full.strip_prefix(root).map_err(|_| {
        anyhow::anyhow!(
            "`{}` is outside the observed root `{}`",
            full.display(),
            root.display()
        )
    })?;
    Ok(scope::rel_str(rel))
}
