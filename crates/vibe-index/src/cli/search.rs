//! `vibe-index search <data-dir> <query>` — full-text search.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use clap::Parser;
use semver::Version;
use serde::Serialize;

use crate::cli::kinds;
use crate::error::{Error, Result};
use crate::index::{Index, search};
use crate::types::PackageKind;

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
    hit_count: usize,
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
}

pub fn run(args: Args) -> Result<()> {
    let index = Index::load_from(&args.data_dir)?;
    let kind_filter = match &args.kind {
        Some(raw) => Some(kinds::parse_kind_flag(raw)?),
        None => None,
    };
    let hits = search::search(&index, &args.query, kind_filter);
    let limited: Vec<&search::SearchHit> = hits.iter().take(args.limit).collect();

    if args.json {
        let env = Envelope {
            command: "search",
            query: args.query.clone(),
            hit_count: limited.len(),
            hits: limited
                .iter()
                .map(|h| HitRow {
                    kind: h.kind.clone(),
                    name: h.name.clone(),
                    latest_stable: h.latest_stable.clone(),
                    score: h.score,
                    matched_tokens: h.matched_tokens.clone(),
                    description: h.description.clone(),
                })
                .collect(),
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&env)
                .map_err(|e| Error::Malformed(format!("envelope: {e}")))?
        );
    } else {
        println!("query     : {}", args.query);
        println!("hits      : {}", limited.len());
        for h in limited {
            print!("  {}:{}", h.kind, h.name);
            if let Some(latest) = &h.latest_stable {
                print!(" @ {latest}");
            }
            println!(" (score {})", h.score);
            if let Some(d) = &h.description {
                println!("    {d}");
            }
        }
    }
    Ok(())
}
