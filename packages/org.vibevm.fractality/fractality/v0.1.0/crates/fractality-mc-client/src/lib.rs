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
    Ack, ErrorResponse, HealthResponse, KillRequest, KillResponse, MetricsResponse, NodeResponse,
    PodEventRequest, PodHeartbeat, PodHeartbeatResponse, PodRegisterRequest, PodRegisterResponse,
    RegisterRunRequest, RunListResponse, ShutdownResponse, TreeNode,
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

    pub async fn kill(&self, id: RunId, recursive: bool) -> Result<KillResponse, ClientError> {
        self.request(
            "POST /v0/runs/:id/kill",
            self.http
                .post(self.url(&format!("/runs/{id}/kill")))
                .json(&KillRequest { recursive }),
        )
        .await
    }

    pub async fn metrics(&self) -> Result<MetricsResponse, ClientError> {
        self.request("GET /v0/metrics", self.http.get(self.url("/metrics")))
            .await
    }

    /// Parks a run on a question (D18); called by the ask_boss broker.
    pub async fn question(&self, id: RunId, question: &str) -> Result<RunRecord, ClientError> {
        self.request(
            "POST /v0/runs/:id/question",
            self.http
                .post(self.url(&format!("/runs/{id}/question")))
                .json(&fractality_core::api::QuestionRequest {
                    question: question.to_owned(),
                }),
        )
        .await
    }

    /// Answers a parked run (D18); the run resumes.
    pub async fn answer(&self, id: RunId, answer: &str) -> Result<RunRecord, ClientError> {
        self.request(
            "POST /v0/runs/:id/answer",
            self.http
                .post(self.url(&format!("/runs/{id}/answer")))
                .json(&fractality_core::api::AnswerRequest {
                    answer: answer.to_owned(),
                }),
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
    resolve_fractality_binary("FRACTALITY_MC_BIN", "fractality-mission-control")
}

/// Resolves the pod binary the daemon launches per run (D3):
/// `FRACTALITY_POD_BIN` override → sibling of the current executable →
/// PATH lookup. Lives beside the other process-discovery seams because
/// this file is the recorded composition surface for those env reads.
///
/// ```
/// let pod = fractality_mc_client::resolve_pod_binary().expect("resolves");
/// // Worst case is the PATH fallback: the name is always fractality-pod.
/// assert!(
///     pod.file_name()
///         .and_then(|n| n.to_str())
///         .is_some_and(|n| n.starts_with("fractality-pod"))
/// );
/// ```
pub fn resolve_pod_binary() -> Result<std::path::PathBuf, ClientError> {
    resolve_fractality_binary("FRACTALITY_POD_BIN", "fractality-pod")
}

fn resolve_fractality_binary(
    env_override: &str,
    stem: &str,
) -> Result<std::path::PathBuf, ClientError> {
    if let Some(explicit) = std::env::var_os(env_override) {
        let p = std::path::PathBuf::from(explicit);
        if p.is_file() {
            return Ok(p);
        }
        return Err(ClientError::AutoStart {
            message: format!(
                "{env_override} points at `{}` which is not a file",
                p.display()
            ),
        });
    }
    let name = if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_owned()
    };
    if let Ok(me) = std::env::current_exe()
        && let Some(dir) = me.parent()
    {
        let sibling = dir.join(&name);
        if sibling.is_file() {
            return Ok(sibling);
        }
    }
    // Fall back to PATH: let the OS loader find it at spawn time.
    Ok(std::path::PathBuf::from(name))
}

/// Strips `HANDLE_FLAG_INHERIT` from this process's own std handles for
/// the duration of a detached spawn, restoring the flags on drop (F17).
///
/// Why this exists: on Windows, `CreateProcess(bInheritHandles=TRUE)` —
/// which Rust std uses whenever stdio is redirected — copies EVERY
/// inheritable handle in our table into the child, not just the three
/// stdio slots. When a shell runs `id=$(fractality spawn …)`, our stdout
/// IS the substitution pipe's write end, marked inheritable by the
/// shell; the freshly auto-started daemon would inherit that write end
/// and, being a daemon, never close it — the shell then waits for an
/// EOF that cannot come and the substitution hangs forever. Observed
/// live in MT-02's first firing.
#[cfg(windows)]
mod inherit_guard {
    // Minimal kernel32 FFI — no new dependency for three calls at one
    // recorded OS seam.
    unsafe extern "system" {
        fn GetStdHandle(nstdhandle: u32) -> isize;
        fn GetHandleInformation(handle: isize, flags: *mut u32) -> i32;
        fn SetHandleInformation(handle: isize, mask: u32, flags: u32) -> i32;
    }
    const STD_INPUT_HANDLE: u32 = -10i32 as u32;
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;
    const HANDLE_FLAG_INHERIT: u32 = 1;
    const INVALID_HANDLE_VALUE: isize = -1;

    /// RAII: strip on construction, restore on drop.
    pub(crate) struct Guard {
        restore: Vec<isize>,
    }

    impl Guard {
        /// SAFETY envelope: queries/toggles one documented flag on our
        /// own std handles; no memory is transferred, failures are
        /// ignored (worst case the old hang persists for that slot).
        #[specmark::spec(
            deviates = "spec://fractality/PROP-001#architecture",
            reason = "three kernel32 calls at one recorded OS seam (F17): stripping HANDLE_FLAG_INHERIT from our own std handles around a detached spawn; an audit crate for this would be pure ceremony"
        )]
        pub(crate) fn strip_std_handles() -> Self {
            let mut restore = Vec::new();
            for slot in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
                unsafe {
                    let handle = GetStdHandle(slot);
                    if handle == 0 || handle == INVALID_HANDLE_VALUE {
                        continue;
                    }
                    let mut flags = 0u32;
                    if GetHandleInformation(handle, &mut flags) == 0 {
                        continue;
                    }
                    if flags & HANDLE_FLAG_INHERIT != 0
                        && SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) != 0
                    {
                        restore.push(handle);
                    }
                }
            }
            Self { restore }
        }

        /// SAFETY envelope: restores the exact flag [`Self::strip_std_handles`]
        /// cleared, on the same handles.
        #[specmark::spec(
            deviates = "spec://fractality/PROP-001#architecture",
            reason = "the restore half of the F17 guard: one SetHandleInformation call re-arming the flag strip_std_handles removed"
        )]
        fn restore_inherit(handle: isize) {
            unsafe {
                SetHandleInformation(handle, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT);
            }
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            for handle in &self.restore {
                Guard::restore_inherit(*handle);
            }
        }
    }
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
    // F17: never let the detached daemon capture our caller's pipes —
    // `id=$(fractality spawn …)` must reach EOF the moment we exit.
    #[cfg(windows)]
    let _inherit_guard = inherit_guard::Guard::strip_std_handles();
    cmd.spawn().map_err(|e| ClientError::AutoStart {
        message: format!("spawning `{}`: {e}", bin.display()),
    })?;
    tracing::debug!(bin = %bin.display(), %home, "mission-control spawn requested");
    Ok(())
}
