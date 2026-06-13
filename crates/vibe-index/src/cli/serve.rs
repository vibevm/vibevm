//! `vibe-index serve <data-dir>` — boot the HTTP server.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

use crate::error::{Error, Result};
use crate::index::Index;
use crate::lock::ServerLock;
use crate::server::rate_limit::DEFAULT_MAX_BUCKETS;
use crate::server::{AppState, RateLimitConfig, TokenStore, build_app};

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
    /// git push` in the data directory. Slice 5 stub.
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
    let _ = args.auto_commit_push; // parked until slice 9.

    let index = Index::load_from(&args.data_dir).map_err(|e| match e {
        Error::Io { .. } | Error::Malformed(_) => Error::InvalidInput(format!(
            "data-dir `{}` does not look like an initialised index. \
             Run `vibe-index init` first.",
            args.data_dir.display()
        )),
        other => other,
    })?;

    let lock = ServerLock::try_acquire(&args.data_dir)?;

    let tokens = match args.auth_tokens_file.as_deref() {
        Some(path) => TokenStore::load_from_path(path)?,
        None => TokenStore::load(&args.data_dir)?,
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
    );

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
