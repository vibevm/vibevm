//! `vibe-index search <data-dir> <query>` — full-text search.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use clap::Parser;
use semver::Version;
use serde::Serialize;

use crate::cli::kinds;
use crate::error::{Error, Result};
use crate::index::quarantine::{Unavailable, unavailable_for};
use crate::index::{Index, search};
use crate::types::PackageKind;
use crate::wire_count::checked_u32;

#[derive(Debug, Parser)]
#[command(about = "Full-text search across the index.")]
pub struct Args {
    pub data_dir: PathBuf,
    pub query: String,

    /// Keep only hits of this kind: flow, feat, stack, tool, mcp,
    /// lang. The wire vocabulary is open, but the ARGUMENT speaks: a
    /// kind this build does not know is refused with a message, not
    /// filtered away in silence.
    #[arg(long, value_name = "KIND")]
    pub kind: Option<String>,

    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct Envelope {
    command: &'static str,
    query: String,
    hit_count: u32,
    hits: Vec<HitRow>,
}

#[derive(Debug, Serialize)]
struct HitRow {
    kind: PackageKind,
    name: String,
    latest_stable: Option<Version>,
    score: u32,
    matched_tokens: Vec<String>,
    description: Option<String>,
    /// Versions of this hit's package this build refuses to act on —
    /// named, not hidden (PROP-044 §4.5). Absent when there are none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    unavailable: Vec<Unavailable>,
}

pub fn run(args: Args) -> Result<()> {
    let index = Index::load_from(&args.data_dir)?;
    let kind_filter = match &args.kind {
        Some(raw) => Some(kinds::parse_kind_flag(raw)?),
        None => None,
    };
    let hits = search::search(&index, &args.query, kind_filter);
    let limited: Vec<&search::SearchHit> = hits.iter().take(args.limit).collect();
    // The refusal rows live on the hit's package, not on the scored
    // version: a hit names a package, and the package's unusable
    // versions are part of the honest answer about it.
    let rows: Vec<HitRow> = limited
        .iter()
        .map(|h| {
            Ok(HitRow {
                kind: h.kind.clone(),
                name: h.name.clone(),
                latest_stable: h.latest_stable.clone(),
                score: checked_u32("score", h.score)?,
                matched_tokens: h.matched_tokens.clone(),
                description: h.description.clone(),
                unavailable: index
                    .get(&h.group, &h.name)
                    .map(unavailable_for)
                    .unwrap_or_default(),
            })
        })
        .collect::<Result<_>>()?;

    if args.json {
        let hit_count = checked_u32("hit_count", rows.len())?;
        let env = Envelope {
            command: "search",
            query: args.query.clone(),
            hit_count,
            hits: rows,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&env)
                .map_err(|e| Error::Malformed(format!("envelope: {e}")))?
        );
    } else {
        println!("query     : {}", args.query);
        println!("hits      : {}", rows.len());
        for h in &rows {
            print!("  {}:{}", h.kind, h.name);
            if let Some(latest) = &h.latest_stable {
                print!(" @ {latest}");
            }
            println!(" (score {})", h.score);
            if let Some(d) = &h.description {
                println!("    {d}");
            }
            if !h.unavailable.is_empty() {
                let listed = h
                    .unavailable
                    .iter()
                    .map(|u| format!("{} (missing: {})", u.version, u.missing.join(",")))
                    .collect::<Vec<_>>()
                    .join("; ");
                println!("    unavailable : {listed}");
            }
        }
    }
    Ok(())
}
