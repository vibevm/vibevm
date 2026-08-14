//! `vibe-index serve <data-dir>` — boot the HTTP server.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use clap::Parser;

use crate::error::{Error, Result};
use crate::index::Index;
use crate::journal::{default_dir, project, replay};
use crate::lock::ServerLock;
use crate::server::rate_limit::DEFAULT_MAX_BUCKETS;
use crate::server::{AppState, FileTokenStore, RateLimitConfig, build_app};

#[derive(Debug, Parser)]
#[command(about = "Run the HTTP server.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Address to bind. Default: `127.0.0.1:8412` (local-only).
    #[arg(long, value_name = "ADDR", default_value = "127.0.0.1:8412")]
    pub bind: SocketAddr,

    /// File containing one bearer token per line. Slice 5 ignores
    /// this; slice 6 wires the auth layer.
    #[arg(long, value_name = "FILE")]
    pub auth_tokens_file: Option<PathBuf>,

    /// Refuse every mutating endpoint regardless of auth (slice 5
    /// has no mutating endpoints anyway, so the flag effectively
    /// pins the read-only posture).
    #[arg(long)]
    pub read_only: bool,

    /// After every successful mutation, `git add -A && git commit &&
    /// git push` the data directory (which must itself be a git working
    /// copy whose `state/` is gitignored — bearer tokens live there).
    /// The commit message names the change; a push failure is logged at
    /// WARN and counted in `/metrics`, never raised. Startup refuses if
    /// the data dir is not a git repo or `state/` is not ignored.
    #[arg(long)]
    pub auto_commit_push: bool,

    /// Per-token rate limit (requests / minute, per bearer token).
    /// `0` disables (the default). PROP-005 §9 Q10. Bucket capacity
    /// equals the RPM (so a fresh token can burst up to its full
    /// minute allowance, then is throttled to RPM/60 per second
    /// steady-state). Routes `/healthz`, `/readyz`, `/metrics` are
    /// always exempt.
    #[arg(long, value_name = "RPM", default_value_t = 0)]
    pub rate_limit_per_token: u32,

    /// Per-IP rate limit (requests / minute, per anonymous peer
    /// IP). `0` disables. Same semantics as `--rate-limit-per-token`
    /// but for unauthenticated reads.
    #[arg(long, value_name = "RPM", default_value_t = 0)]
    pub rate_limit_per_ip: u32,
}

pub fn run(args: Args) -> Result<()> {
    // `--auto-commit-push` boots the self-publishing path; its preflight
    // (a git working copy, `state/` gitignored) must pass before we
    // serve a single mutation. Observability is unconditional — the
    // binary installs the subscriber for every subcommand (one lever,
    // `VIBE_LOG`) — so only the preflight gate stays under the flag.
    if args.auto_commit_push {
        crate::publish::preflight(&args.data_dir)?;
    }

    // Ф3.2c2 — the server boots from the journal (the truth layer),
    // never from the catalog (PROP-044 §4.4): a data-dir whose catalog
    // is richer than its journal would silently lose that surplus on
    // the first mutation, which replaces the served state with the
    // journal's projection. A data-dir with a catalog but no journal —
    // born before `init` began recording `Initialised` — therefore
    // refuses to boot rather than serve unrebuildable state.
    let index = boot_index(&args.data_dir)?;

    let lock = ServerLock::try_acquire(&args.data_dir)?;

    let tokens = match args.auth_tokens_file.as_deref() {
        Some(path) => FileTokenStore::load_from_path(path)?,
        None => FileTokenStore::load(&args.data_dir)?,
    };

    let rate_limit = RateLimitConfig {
        per_token_rpm: args.rate_limit_per_token,
        per_ip_rpm: args.rate_limit_per_ip,
        max_buckets: DEFAULT_MAX_BUCKETS,
    };

    let state = AppState::with_tokens_and_rate_limit(
        args.data_dir.clone(),
        args.read_only,
        index,
        tokens,
        rate_limit,
    )
    .with_auto_commit_push(args.auto_commit_push);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::Io {
            path: args.data_dir.clone(),
            message: format!("could not build tokio runtime: {e}"),
        })?;

    runtime.block_on(async move {
        let app = build_app(state);
        let listener = tokio::net::TcpListener::bind(args.bind)
            .await
            .map_err(|e| Error::InvalidInput(format!("could not bind {}: {e}", args.bind)))?;

        eprintln!(
            "vibe-index serving `{}` at http://{} (read-only={}, pid={})",
            args.data_dir.display(),
            args.bind,
            args.read_only,
            std::process::id(),
        );

        // `into_make_service_with_connect_info::<SocketAddr>` is what
        // makes peer-IP available to the rate-limit middleware via
        // the `ConnectInfo<SocketAddr>` extension. PROP-005 §9 Q10.
        let make_svc = app.into_make_service_with_connect_info::<std::net::SocketAddr>();
        let server = axum::serve(listener, make_svc);
        tokio::select! {
            r = server => r.map_err(|e| Error::Io {
                path: args.data_dir.clone(),
                message: format!("server: {e}"),
            }),
            _ = tokio::signal::ctrl_c() => {
                eprintln!("vibe-index: SIGINT received, shutting down");
                Ok(())
            }
        }
    })?;

    drop(lock);
    Ok(())
}

/// The server's boot projection: fold the data-dir's journal and
/// nothing else. The catalog on disk (`repomd.json`, `by-name/`, …)
/// is a pure output of this fold — `serve` never reads it — so the
/// served state is always one the truth layer can rebuild (PROP-044
/// §4.4). Extracted from [`run`] so a test can drive the exact boot
/// decision — rises from a journal-only data-dir, refuses a
/// catalog-only one — without binding a listener.
pub fn boot_index(data_dir: &Path) -> Result<Index> {
    let journal_dir = default_dir(data_dir);
    let records = replay(&journal_dir).map_err(|e| match e {
        Error::Malformed(detail) => Error::InvalidInput(format!(
            "data-dir `{}` holds a journal that cannot be read: {detail}. \
             The journal is the truth layer — the catalog on disk cannot substitute for it \
             (violates spec://org.vibevm.core/vibevm/common/PROP-044#truth; \
              fix: restore the journal shard named above, or recreate the data-dir with `vibe-index init`)",
            data_dir.display()
        )),
        other => other,
    })?;
    project(records).map_err(|e| match e {
        Error::Unprojectable(reason) => Error::InvalidInput(format!(
            "data-dir `{}` cannot be served from its journal: {reason}. \
             A catalog on disk, if one is present, is a projection and is not an input \
             (violates spec://org.vibevm.core/vibevm/common/PROP-044#truth; \
              fix: this data-dir predates the journal — recreate it with `vibe-index init`, \
              then publish into it again)",
            data_dir.display()
        )),
        other => other,
    })
}
