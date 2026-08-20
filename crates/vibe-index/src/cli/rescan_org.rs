//! `vibe-index rescan-org <data-dir> --from-github <org>` — enumerate
//! the GitHub org **unconditionally** and refresh the org-image
//! cache. PROP-005 §2.8 / slice A3.
//!
//! Why a separate verb (Р4, УТОЧНИ-5): the org-image cache and its
//! conditional validator are a *cheap probabilistic* freshness check
//! — a `304` on page 1 means "page 1 unchanged", not "the whole org
//! is provably unchanged". A change the probe misses is invisible
//! from inside the index: no freshness mechanism gives a hard
//! guarantee, only a full traversal does. `rescan-org` is that full
//! traversal, made explicit so an operator can force one on demand
//! without remembering a flag combination. It ignores the cache and
//! the validator (never sends `If-None-Match`), walks every page, and
//! still rewrites the image so the next `reindex --cache-org` starts
//! from a known-fresh baseline.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use clap::Parser;

use crate::cli::reindex::{Plan, build_github_scanner, run_plan};
use crate::error::Result;
use crate::scanner::org_cache;

#[derive(Debug, Parser)]
#[command(about = "Unconditionally re-enumerate the GitHub org and refresh the org-image cache.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// The GitHub org to enumerate unconditionally.
    #[arg(long, value_name = "ORG")]
    pub from_github: String,

    /// File containing the host API token (one line, no trailing newline).
    #[arg(long, value_name = "FILE")]
    pub token_file: Option<PathBuf>,

    /// GitHub REST API base URL. Defaults to `https://api.github.com`
    /// — the default is the ladder's last rung, so
    /// `VIBE_INDEX_API_BASE` and an `api-base` key in
    /// `<data-dir>/state/config.toml` also feed this member (flag
    /// beats env beats file beats default). Override for tests or
    /// self-hosted GitHub Enterprise instances.
    #[arg(long, value_name = "URL")]
    pub api_base: Option<String>,

    /// Where the scanner clones repos. Defaults to a fresh tempdir
    /// removed at the end of the run. Pass an explicit path to keep a
    /// warm cache (subsequent runs reuse it).
    #[arg(long, value_name = "DIR")]
    pub clone_cache: Option<PathBuf>,

    /// Emit JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: Args) -> Result<()> {
    // Ladder (PROP-005 §3.5, B-086): the same two scanner-facing
    // members `reindex` resolves — `git` for the shell-out layer,
    // `api-base` for the GitHub client.
    let ladder = crate::config::Ladder::load(&args.data_dir)?;
    crate::scanner::git_cli::set_binary(
        crate::config::resolve_git(&ladder, &crate::config::live_env)?.value,
    );
    let api_base = crate::config::resolve_api_base(
        &ladder,
        args.api_base.as_deref(),
        &crate::config::live_env,
    )?
    .value;

    // Р4 — probe_freshness = false: ignore the cache AND its validator,
    // enumerate every page unconditionally. The image is still
    // re-persisted (org_cache_path = Some) so the next run benefits.
    let (scanner, temp_guard) = build_github_scanner(
        args.token_file.as_deref(),
        &api_base,
        &args.from_github,
        args.clone_cache.clone(),
        Some(org_cache::path(&args.data_dir)),
        false,
    )?;
    run_plan(Plan {
        data_dir: args.data_dir.clone(),
        scanner,
        source: "github",
        mode: "full",
        json: args.json,
        _temp_guard: temp_guard,
    })
}
