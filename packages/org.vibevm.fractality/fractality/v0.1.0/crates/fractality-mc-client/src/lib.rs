//! Client of the mission-control bus (plan D10/D13).
//!
//! Everything that talks to the daemon goes through this crate: the CLI
//! (client leg), the pods (pod leg), and mission-control itself (the
//! stale-lock liveness probe). Discovery is **lockfile-first**: the
//! daemon binds an ephemeral localhost port and publishes
//! `{port, pid, bearer}` in `<home>/mc.lock`; clients and pods re-read
//! the lockfile on every (re)connect, which is exactly how a pod finds a
//! *restarted* daemon on its new port with its new bearer (D3 adoption).

pub mod home;
pub mod lock;

use fractality_core::api::{
    Ack, ErrorResponse, HealthResponse, NodeResponse, PodEventRequest, PodHeartbeat,
    PodHeartbeatResponse, PodRegisterRequest, PodRegisterResponse, RegisterRunRequest,
    RunListResponse, ShutdownResponse, TreeNode,
};
use fractality_core::ids::{PodId, RunId};
use fractality_core::run::{RunRecord, RunState};
use specmark::spec;

use crate::lock::Lockfile;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Client-side errors, with the fix surface in the message (D14).
///
/// Every rendering names the violated contract and the fix:
///
/// ```
/// use fractality_mc_client::ClientError;
///
/// let e = ClientError::NotRunning { home: "C:/x/.fractality".into() };
/// assert!(e.to_string().contains("fractality mc start"));
/// assert!(e.to_string().contains("violates spec://"));
/// ```
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://fractality/PROP-001#architecture")]
pub enum ClientError {
    #[error(
        "mission-control is not running (no live lockfile under `{home}`) \
         (violates spec://fractality/PROP-001#architecture; fix: start it with `fractality mc start`)"
    )]
    NotRunning { home: String },

    #[error(
        "mission-control answered {status} on {verb}: {message} (violates spec://fractality/PROP-001#architecture; fix: reconcile the request with the running mission-control)"
    )]
    Api {
        verb: String,
        status: u16,
        message: String,
    },

    #[error(
        "HTTP transport to mission-control failed: {0} (violates spec://fractality/PROP-001#architecture; fix: check that mission-control is reachable)"
    )]
    Transport(#[from] reqwest::Error),

    #[error(
        "lockfile is unreadable: {0} (violates spec://fractality/PROP-001#architecture; fix: repair or remove the lockfile so the daemon can republish it)"
    )]
    Lock(String),

    #[error(
        "could not auto-start mission-control: {message} \
         (violates spec://fractality/PROP-001#architecture; fix: start it manually with `fractality mc start` or set FRACTALITY_MC_BIN)"
    )]
    AutoStart { message: String },
}

/// One connected client: base URL + bearer from a live lockfile.
///
/// ```
/// use fractality_mc_client::{McClient, lock::Lockfile};
///
/// let lock = Lockfile {
///     schema: 1,
///     port: 50123,
///     pid: 4242,
///     bearer: "bearer-string".into(),
///     started_ts_ms: 0,
/// };
/// let client = McClient::from_lockfile(&lock).expect("builds without probing");
/// # let _ = client;
/// ```
#[derive(Debug, Clone)]
pub struct McClient {
    base: String,
    bearer: String,
    http: reqwest::Client,
}

impl McClient {
    /// Builds a client from a lockfile without probing liveness. The
    /// error leg exists for the type system, not for practice: with no
    /// TLS backend compiled in, reqwest's builder has nothing left to
    /// fail on — but the contract stays honest instead of panicking.
    pub fn from_lockfile(lock: &Lockfile) -> Result<Self, ClientError> {
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(2))
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Ok(Self {
            base: format!("http://127.0.0.1:{}", lock.port),
            bearer: lock.bearer.clone(),
            http,
        })
    }

    /// Reads `<home>/mc.lock` and probes `/v0/health`; `Ok(None)` when
    /// there is no lockfile or the daemon it names is gone.
    pub async fn connect(home: &camino::Utf8Path) -> Result<Option<McClient>, ClientError> {
        let Some(lock) = Lockfile::read(home).map_err(|e| ClientError::Lock(e.to_string()))? else {
            return Ok(None);
        };
        let client = McClient::from_lockfile(&lock)?;
        match client.health().await {
            Ok(_) => Ok(Some(client)),
            // A dead daemon behind a stale lockfile is "not running", not
            // an error: callers decide whether to start one.
            Err(ClientError::Transport(_)) => Ok(None),
            Err(other) => Err(other),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}{}", self.base, fractality_core::api::API_PREFIX, path)
    }

    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        verb: &str,
        req: reqwest::RequestBuilder,
    ) -> Result<T, ClientError> {
        let resp = req
            .header("Authorization", format!("Bearer {}", self.bearer))
            .send()
            .await?;
        let status = resp.status();
        if status.is_success() {
            return Ok(resp.json::<T>().await?);
        }
        let message = match resp.json::<ErrorResponse>().await {
            Ok(body) => match body.hint {
                Some(hint) => format!("{} ({hint})", body.error),
                None => body.error,
            },
            Err(_) => "unparseable error body".to_owned(),
        };
        Err(ClientError::Api {
            verb: verb.to_owned(),
            status: status.as_u16(),
            message,
        })
    }

    // ------------------------------------------------------------ client leg

    pub async fn health(&self) -> Result<HealthResponse, ClientError> {
        self.request("GET /v0/health", self.http.get(self.url("/health")))
            .await
    }

    pub async fn node(&self) -> Result<NodeResponse, ClientError> {
        self.request("GET /v0/node", self.http.get(self.url("/node")))
            .await
    }

    pub async fn register_run(&self, req: &RegisterRunRequest) -> Result<RunRecord, ClientError> {
        self.request("POST /v0/runs", self.http.post(self.url("/runs")).json(req))
            .await
    }

    pub async fn runs(
        &self,
        state: Option<RunState>,
        limit: Option<usize>,
    ) -> Result<Vec<RunRecord>, ClientError> {
        // Both values are URL-safe by construction (state names are
        // snake_case idents, limit is a number) — no encoder needed.
        let mut path = String::from("/runs");
        let mut sep = '?';
        if let Some(s) = state {
            path.push(sep);
            path.push_str("state=");
            path.push_str(s.as_str());
            sep = '&';
        }
        if let Some(n) = limit {
            path.push(sep);
            path.push_str("limit=");
            path.push_str(&n.to_string());
        }
        let resp: RunListResponse = self
            .request("GET /v0/runs", self.http.get(self.url(&path)))
            .await?;
        Ok(resp.runs)
    }

    pub async fn run(&self, id: RunId) -> Result<RunRecord, ClientError> {
        self.request(
            "GET /v0/runs/:id",
            self.http.get(self.url(&format!("/runs/{id}"))),
        )
        .await
    }

    pub async fn tree(&self, id: RunId) -> Result<TreeNode, ClientError> {
        self.request(
            "GET /v0/runs/:id/tree",
            self.http.get(self.url(&format!("/runs/{id}/tree"))),
        )
        .await
    }

    pub async fn shutdown(&self) -> Result<ShutdownResponse, ClientError> {
        self.request("POST /v0/shutdown", self.http.post(self.url("/shutdown")))
            .await
    }

    // --------------------------------------------------------------- pod leg

    pub async fn pod_register(
        &self,
        req: &PodRegisterRequest,
    ) -> Result<PodRegisterResponse, ClientError> {
        self.request(
            "POST /v0/pods/register",
            self.http.post(self.url("/pods/register")).json(req),
        )
        .await
    }

    pub async fn pod_heartbeat(
        &self,
        pod_id: PodId,
        hb: &PodHeartbeat,
    ) -> Result<PodHeartbeatResponse, ClientError> {
        self.request(
            "POST /v0/pods/:id/heartbeat",
            self.http
                .post(self.url(&format!("/pods/{pod_id}/heartbeat")))
                .json(hb),
        )
        .await
    }

    pub async fn pod_event(&self, pod_id: PodId, ev: &PodEventRequest) -> Result<Ack, ClientError> {
        self.request(
            "POST /v0/pods/:id/event",
            self.http
                .post(self.url(&format!("/pods/{pod_id}/event")))
                .json(ev),
        )
        .await
    }
}

/// Connects, auto-starting the daemon when absent (D3/D13: mission-control
/// auto-starts on first client call; the lockfile probe decides).
///
/// ```no_run
/// # async fn demo() -> Result<(), fractality_mc_client::ClientError> {
/// let home = camino::Utf8Path::new("C:/Users/me/.fractality");
/// let client = fractality_mc_client::connect_or_start(home).await?;
/// let health = client.health().await?;
/// assert_eq!(health.status, "ok");
/// # Ok(())
/// # }
/// ```
pub async fn connect_or_start(home: &camino::Utf8Path) -> Result<McClient, ClientError> {
    if let Some(client) = McClient::connect(home).await? {
        return Ok(client);
    }
    start_daemon(home)?;
    // The daemon needs a moment to bind, stamp scopes, and write the
    // lockfile; poll in small steps up to a hard deadline.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Some(client) = McClient::connect(home).await? {
            return Ok(client);
        }
        if std::time::Instant::now() >= deadline {
            return Err(ClientError::AutoStart {
                message: format!(
                    "daemon did not publish a live lockfile under `{home}` within 10s \
                     (see `{home}/mc.log`)"
                ),
            });
        }
    }
}

/// Resolves the daemon binary: `FRACTALITY_MC_BIN` override → sibling of
/// the current executable → PATH lookup.
fn resolve_mc_binary() -> Result<std::path::PathBuf, ClientError> {
    if let Some(explicit) = std::env::var_os("FRACTALITY_MC_BIN") {
        let p = std::path::PathBuf::from(explicit);
        if p.is_file() {
            return Ok(p);
        }
        return Err(ClientError::AutoStart {
            message: format!(
                "FRACTALITY_MC_BIN points at `{}` which is not a file",
                p.display()
            ),
        });
    }
    let name = if cfg!(windows) {
        "fractality-mission-control.exe"
    } else {
        "fractality-mission-control"
    };
    if let Ok(me) = std::env::current_exe()
        && let Some(dir) = me.parent()
    {
        let sibling = dir.join(name);
        if sibling.is_file() {
            return Ok(sibling);
        }
    }
    // Fall back to PATH: let the OS loader find it at spawn time.
    Ok(std::path::PathBuf::from(name))
}

/// Spawns the daemon detached, stdout/stderr appended to `<home>/mc.log`.
fn start_daemon(home: &camino::Utf8Path) -> Result<(), ClientError> {
    let bin = resolve_mc_binary()?;
    std::fs::create_dir_all(home.as_std_path()).map_err(|e| ClientError::AutoStart {
        message: format!("cannot create home `{home}`: {e}"),
    })?;
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.join("mc.log").as_std_path())
        .map_err(|e| ClientError::AutoStart {
            message: format!("cannot open `{home}/mc.log`: {e}"),
        })?;
    let log_err = log.try_clone().map_err(|e| ClientError::AutoStart {
        message: format!("cannot clone log handle: {e}"),
    })?;

    let mut cmd = std::process::Command::new(&bin);
    cmd.env("FRACTALITY_HOME", home.as_str())
        .stdin(std::process::Stdio::null())
        .stdout(log)
        .stderr(log_err);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW: the daemon detaches
        // from this console and survives the caller's exit.
        cmd.creation_flags(0x0000_0200 | 0x0800_0000);
    }
    cmd.spawn().map_err(|e| ClientError::AutoStart {
        message: format!("spawning `{}`: {e}", bin.display()),
    })?;
    tracing::debug!(bin = %bin.display(), %home, "mission-control spawn requested");
    Ok(())
}
