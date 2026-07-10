//! fractality-mission-control — the scheduler daemon (plan D3/D4/D9/D10).
//!
//! One process per machine: it owns the append-only journal (the
//! profiling-metadata store, invariant I3), the in-memory run registry
//! (a pure fold of the journal), the lockfile + bearer, and the
//! versioned localhost HTTP bus every other fractality process talks
//! through. Pods supervise workers and re-register across daemon
//! restarts — adoption is a protocol feature, not journal archaeology.
//!
//! The crate is a library so tests (and the pod crate's loopback tests)
//! can embed a real daemon in-process; `src/main.rs` is the thin binary.

pub mod admission;
pub mod http;
pub mod http_questions;
pub mod http_sessions;
pub mod identity;
pub mod journal_store;
pub mod kill;
pub mod metrics;
pub mod registry;
pub mod scopes;
pub mod sessions;
pub mod state;
pub mod workspace;

use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use fractality_mc_client::lock::Lockfile;
use specmark::spec;
use tokio::task::JoinHandle;

use crate::state::AppState;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Daemon configuration: everything hangs off the home directory.
///
/// ```
/// use fractality_mission_control::Config;
///
/// let cfg = Config::new("C:/tmp/fractality-home");
/// assert!(cfg.journal_dir().as_str().ends_with("journal"));
/// assert!(cfg.runs_root().as_str().ends_with("runs"));
/// ```
#[derive(Debug, Clone)]
pub struct Config {
    pub home: Utf8PathBuf,
}

impl Config {
    pub fn new(home: impl Into<Utf8PathBuf>) -> Self {
        Self { home: home.into() }
    }

    pub fn journal_dir(&self) -> Utf8PathBuf {
        self.home.join("journal")
    }

    pub fn runs_root(&self) -> Utf8PathBuf {
        self.home.join("runs")
    }
}

/// Errors on the daemon lifecycle path.
///
/// ```
/// use fractality_mission_control::ServerError;
///
/// let e = ServerError::AlreadyRunning { pid: 4242, port: 50123 };
/// assert!(e.to_string().contains("already running"));
/// assert!(e.to_string().contains("violates spec://"));
/// ```
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://fractality/PROP-001#architecture")]
pub enum ServerError {
    #[error(
        "mission-control is already running (pid {pid}, port {port}) \
         (violates spec://fractality/PROP-001#architecture; fix: stop it first with `fractality mc stop`)"
    )]
    AlreadyRunning { pid: u32, port: u16 },

    #[error(
        "cannot prepare home `{home}`: {message} (violates spec://fractality/PROP-001#architecture; fix: ensure the home directory is writable and exists)"
    )]
    Home { home: Utf8PathBuf, message: String },

    #[error(
        "journal is unusable: {0} (violates spec://fractality/PROP-001#architecture; fix: repair the journal directory or reinitialize home)"
    )]
    Journal(String),

    #[error(
        "bind/serve failed: {0} (violates spec://fractality/PROP-001#architecture; fix: free the port or check permissions and restart the daemon)"
    )]
    Io(#[from] std::io::Error),
}

/// A started daemon, embeddable in tests.
///
/// The canonical lifecycle:
///
/// ```no_run
/// # async fn demo() -> Result<(), fractality_mission_control::ServerError> {
/// use fractality_mission_control::{Config, start};
///
/// let server = start(Config::new("C:/tmp/fractality-home")).await?;
/// println!("bus at {}", server.addr);
/// server.stop().await; // graceful: drains, removes the lockfile
/// # Ok(())
/// # }
/// ```
pub struct RunningServer {
    pub addr: std::net::SocketAddr,
    pub state: Arc<AppState>,
    serve_task: JoinHandle<()>,
    reaper_task: JoinHandle<()>,
}

impl RunningServer {
    /// Graceful stop: signal, await the serve loop, remove the lockfile.
    pub async fn stop(self) {
        // A watch channel, not Notify: the signal is *state* ("shutting
        // down"), so a waiter that subscribes after the send still sees
        // it — Notify::notify_waiters loses the wakeup if it fires before
        // the waiter's first poll (a real hang this project hit).
        let _ = self.state.shutdown.send_replace(true);
        let _ = self.serve_task.await;
        self.reaper_task.abort();
        let _ = Lockfile::remove(&self.state.cfg.home);
        tracing::info!("mission-control stopped");
    }

    /// Hard kill for restart tests: abort the serve loop **without**
    /// removing the lockfile — the on-disk state a crashed daemon leaves
    /// behind. Caveat (learned the hard way): this does NOT sever live
    /// keep-alive connections — hyper's per-connection tasks outlive the
    /// aborted accept loop, so a client with a pooled connection can
    /// still talk to the "dead" generation. Tests that need true crash
    /// semantics against live clients must kill a real daemon *process*
    /// (see the pod crate's loopback restart test).
    pub async fn kill_for_test(self) {
        self.serve_task.abort();
        self.reaper_task.abort();
        let _ = self.serve_task.await;
        let _ = self.reaper_task.await;
    }
}

/// Starts the daemon: stale-lock check, journal replay, scope beacon,
/// ephemeral bind, lockfile publish, serve + reaper tasks.
///
/// A second start on the same home refuses while the first is alive:
///
/// ```no_run
/// # async fn demo() -> Result<(), fractality_mission_control::ServerError> {
/// use fractality_mission_control::{Config, start};
///
/// let first = start(Config::new("C:/tmp/fractality-home")).await?;
/// let second = start(Config::new("C:/tmp/fractality-home")).await;
/// assert!(second.is_err(), "one daemon per home");
/// first.stop().await;
/// # Ok(())
/// # }
/// ```
pub async fn start(cfg: Config) -> Result<RunningServer, ServerError> {
    // A live daemon behind the lockfile means refuse; a dead one means
    // clean up its lock and proceed.
    match Lockfile::read(&cfg.home) {
        Ok(Some(stale)) => {
            // An unbuildable probe client counts as "cannot prove alive"
            // — same disposition as a dead daemon.
            let alive = match fractality_mc_client::McClient::from_lockfile(&stale) {
                Ok(probe) => probe.health().await.is_ok(),
                Err(_) => false,
            };
            if alive {
                return Err(ServerError::AlreadyRunning {
                    pid: stale.pid,
                    port: stale.port,
                });
            }
            tracing::warn!(
                pid = stale.pid,
                port = stale.port,
                "removing stale lockfile of a dead daemon"
            );
            let _ = Lockfile::remove(&cfg.home);
        }
        Ok(None) => {}
        Err(e) => tracing::warn!(error = %e, "unreadable lockfile; replacing"),
    }

    std::fs::create_dir_all(cfg.runs_root().as_std_path()).map_err(|e| ServerError::Home {
        home: cfg.home.clone(),
        message: e.to_string(),
    })?;

    let state = Arc::new(AppState::open(cfg.clone()).map_err(ServerError::Journal)?);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let lock = Lockfile {
        schema: 1,
        port: addr.port(),
        pid: std::process::id(),
        bearer: state.bearer.clone(),
        started_ts_ms: state.started_ts_ms,
    };
    lock.write(&cfg.home).map_err(|e| ServerError::Home {
        home: cfg.home.clone(),
        message: e,
    })?;

    let app = http::router(state.clone());
    let mut shutdown_rx = state.shutdown.subscribe();
    let serve_task = tokio::spawn(async move {
        let result = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                // wait_for checks the current value first: a signal sent
                // before this future's first poll is still seen.
                let _ = shutdown_rx.wait_for(|v| *v).await;
            })
            .await;
        if let Err(e) = result {
            tracing::error!(error = %e, "serve loop ended with an error");
        }
    });
    let reaper_task = tokio::spawn(registry::reaper_loop(state.clone()));

    tracing::info!(%addr, home = %state.cfg.home, "mission-control up");
    Ok(RunningServer {
        addr,
        state,
        serve_task,
        reaper_task,
    })
}

/// Resolves the daemon home like the CLI does (shared helper).
///
/// ```
/// use fractality_mission_control::resolve_home;
///
/// let home = resolve_home(Some(camino::Utf8Path::new("C:/x"))).expect("explicit wins");
/// assert_eq!(home, camino::Utf8PathBuf::from("C:/x"));
/// ```
pub fn resolve_home(explicit: Option<&Utf8Path>) -> Result<Utf8PathBuf, String> {
    fractality_mc_client::home::resolve(explicit)
}
