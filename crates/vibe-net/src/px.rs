//! The vibe-managed px process: located, started on a free loopback port,
//! awaited, and always killed and reaped by its owner.
//!
//! On Unix px runs under a small `/bin/sh` supervisor in its own process
//! group. The supervisor holds the read end of a pipe whose write end only
//! vibe holds: when vibe exits by any route — return, error, `SIGINT`,
//! `SIGKILL` — the pipe closes and the supervisor kills px. On Windows px
//! runs in a job object that kills it when the job handle closes (that is,
//! when vibe exits), started in its own console process group so an
//! attended Ctrl-C question does not take px down before it is answered.

use command_group::{CommandGroup, GroupChild};
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use vibe_core::net::{PxSettings, SystemProxyError};
use vibe_core::user_config::PacSource;

/// How long px may take to accept connections after it was started.
pub const PX_READY_TIMEOUT: Duration = Duration::from_secs(20);

/// The executable name px has on this platform.
pub const PX_EXE: &str = if cfg!(windows) { "px.exe" } else { "px" };

/// Why the vibe-managed px is not available. Each message names the cause
/// and the fix; none carries a credential or a PAC body.
///
/// ```
/// use vibe_net::PxError;
/// let text = PxError::NotInstalled {
///     searched: vec!["/opt/vibe/tools/px/px".into()],
///     hint: "run `vibe self px install`".into(),
/// }
/// .to_string();
/// assert!(text.contains("px is not installed") && text.contains("px_path") && text.contains("fix:"));
/// assert!(text.contains("vibe self px install"));
/// ```
#[derive(Debug, Clone, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-068#proxy")]
pub enum PxError {
    #[error(
        "px is not installed: looked for {} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: {hint}; or set `[proxy] px_path` to a px executable, or put `px` on PATH \
          (`pip install px-proxy`); to stop using px, set `[proxy] mode = \"explicit\"` (with `https`) or `\"none\"`, or export `VIBEVM_PROXY_MODE=none`)",
        searched.iter().map(|path| format!("`{}`", path.display())).collect::<Vec<_>>().join(", ")
    )]
    NotInstalled {
        searched: Vec<PathBuf>,
        hint: String,
    },
    #[error(
        "px `{}` failed to start: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: check that the file is a px executable this machine can run; to stop using px, set `[proxy] mode = \"explicit\"` (with `https`) or `\"none\"`, or export `VIBEVM_PROXY_MODE=none`)",
        program.display()
    )]
    Spawn { program: PathBuf, reason: String },
    #[error(
        "px exited before it accepted connections ({status}){} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: run `vibe self doctor` — it runs `px --test` with the same settings; to stop using px, set `[proxy] mode = \"explicit\"` (with `https`) or `\"none\"`, or export `VIBEVM_PROXY_MODE=none`)",
        if log.is_empty() { String::new() } else { format!(": {log}") }
    )]
    Exited { status: String, log: String },
    #[error(
        "px did not accept connections on {address} within {waited:?} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: check that nothing blocks loopback ports, then run `vibe self doctor`)"
    )]
    NotReady {
        address: SocketAddr,
        waited: Duration,
    },
    #[error(
        "`[proxy] px_port` {port} is already in use{} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: stop what holds it, drop `px_port` so vibe picks a free port, or use a px \
          that already runs there as an explicit proxy: `https = \"http://127.0.0.1:{port}\"`)",
        holder.as_deref().map(|h| format!(" (by {h})")).unwrap_or_default()
    )]
    PortBusy { port: u16, holder: Option<String> },
    #[error(
        "no free loopback port for px: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: free a loopback port and retry)"
    )]
    Port { reason: String },
    #[error(
        "px was not started: the operation was stopped \
         (violates spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW; \
          fix: rerun the operation without requesting a stop)"
    )]
    Cancelled,
    #[error(
        "px is not available: the operation that owned it has ended \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: make the request inside the operation)"
    )]
    Closed,
    /// `pac = "system"` could not be turned into a route: px is never
    /// started to detect it itself, so the cause is reported instead.
    #[error(transparent)]
    SystemProxy(#[from] SystemProxyError),
}

/// Where vibe looks for a px it carries: `tools/px/` beside the running
/// executable and beside its parent directory (a `bin/` layout).
pub fn bundled_px_candidates() -> Vec<PathBuf> {
    let Ok(exe) = std::env::current_exe() else {
        return Vec::new();
    };
    let exe = std::fs::canonicalize(&exe).unwrap_or(exe);
    // Without the Windows verbatim prefix, as the user would spell it.
    let exe = match exe.to_str().and_then(|text| text.strip_prefix(r"\\?\")) {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => exe,
    };
    let mut out = Vec::new();
    if let Some(dir) = exe.parent() {
        out.push(dir.join("tools").join("px").join(PX_EXE));
        if let Some(parent) = dir.parent() {
            out.push(parent.join("tools").join("px").join(PX_EXE));
        }
    }
    out
}

/// The px to run: `px_path` when the configuration names one, else the
/// one this installation carries, else `px` on PATH.
pub(crate) fn locate(settings: &PxSettings) -> Result<PathBuf, PxError> {
    let mut searched = Vec::new();
    if let Some(path) = &settings.path {
        if path.is_file() {
            return Ok(path.clone());
        }
        return Err(PxError::NotInstalled {
            searched: vec![path.clone()],
            hint: "make `[proxy] px_path` name an existing px executable".into(),
        });
    }
    // Beside the running binary (older releases), then the versioned store
    // every installation kind shares (PROP-019 `#companion-tools`).
    for candidate in bundled_px_candidates()
        .into_iter()
        .chain(crate::px_store::stored_executable())
    {
        if candidate.is_file() {
            return Ok(candidate);
        }
        searched.push(candidate);
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(PX_EXE);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    searched.push(PathBuf::from(format!("{PX_EXE} on PATH")));
    Err(PxError::NotInstalled {
        searched,
        hint: crate::px_store::missing_hint(),
    })
}

/// The route arguments of a px run: `--pac` / `--proxy`, the bypass list
/// and the site's extra arguments.
///
/// `pac = "system"` is resolved here, per px start, and px is handed the
/// explicit `--pac=<url>` (or `--proxy=<host:port>`) that came out — px
/// never auto-detects, because on a managed Windows desktop its detection
/// finds nothing (`PROP-068#proxy-px`). The settings may have changed
/// since the last operation, so the answer is read now and not cached.
pub(crate) fn route_args(
    settings: &PxSettings,
    no_proxy: Option<&str>,
) -> Result<Vec<String>, PxError> {
    let mut args = Vec::new();
    let mut system_bypass = None;
    let mut system_upstream = None;
    match settings.pac.as_ref() {
        Some(PacSource::System) => {
            let resolved = crate::system_proxy::resolve()?;
            tracing::debug!("`pac = \"system\"` resolved: {}", resolved.describe());
            if let Some(url) = resolved.pac_url {
                args.push(format!("--pac={url}"));
            }
            system_upstream = resolved.upstream;
            system_bypass = resolved.bypass;
        }
        Some(source) => {
            if let Some(location) = source.px_location() {
                args.push(format!("--pac={location}"));
            }
        }
        None => {}
    }
    // A configured `upstream` is the user's own word and outranks the one
    // the Windows settings named.
    if let Some(upstream) = settings.upstream.as_deref().or(system_upstream.as_deref()) {
        args.push(format!("--proxy={upstream}"));
    }
    let list = no_proxy
        .filter(|list| !list.trim().is_empty())
        .map(str::to_owned)
        .or(system_bypass);
    if let Some(list) = list {
        args.push(format!("--noproxy={list}"));
    }
    args.extend(settings.args.iter().cloned());
    Ok(args)
}

/// A free loopback port, released for px to bind.
pub(crate) fn free_port() -> Result<u16, PxError> {
    TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|error| PxError::Port {
            reason: error.to_string(),
        })
}

/// The proxy environment a child must not inherit into px.
const PROXY_ENV: &[&str] = &[
    "HTTPS_PROXY",
    "https_proxy",
    "HTTP_PROXY",
    "http_proxy",
    "ALL_PROXY",
    "all_proxy",
    "NO_PROXY",
    "no_proxy",
];

/// A px child, owned until [`PxProcess::kill_and_reap`].
pub(crate) struct PxProcess {
    child: GroupChild,
    #[cfg(unix)]
    liveness: Option<std::process::ChildStdin>,
    address: SocketAddr,
    reaped: bool,
}

impl std::fmt::Debug for PxProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PxProcess")
            .field("address", &self.address)
            .field("pid", &self.child.id())
            .finish_non_exhaustive()
    }
}

#[cfg(unix)]
const SUPERVISOR: &str = "exec 3<&0\n\
\"$0\" \"$@\" </dev/null 3<&- &\n\
p=$!\n\
{ read _ <&3; kill \"$p\" 2>/dev/null; } &\n\
w=$!\n\
wait \"$p\"\n\
s=$?\n\
kill \"$w\" 2>/dev/null\n\
exit \"$s\"\n";

impl PxProcess {
    /// Start `program` listening on a free loopback port and wait until it
    /// accepts connections. `stopped` is polled while waiting.
    pub(crate) fn start(
        program: &Path,
        settings: &PxSettings,
        no_proxy: Option<&str>,
        scratch: &Path,
        stopped: &dyn Fn() -> bool,
    ) -> Result<Self, PxError> {
        let port = match settings.port {
            Some(port) => {
                // A fixed port is used as given or not at all.
                if TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)).is_err() {
                    return Err(PxError::PortBusy {
                        port,
                        holder: holder_of(port),
                    });
                }
                port
            }
            None => free_port()?,
        };
        let address = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port));
        let mut args = vec![
            "--listen=127.0.0.1".to_owned(),
            format!("--port={port}"),
            "--foreground=1".to_owned(),
        ];
        args.extend(route_args(settings, no_proxy)?);
        let log_path = scratch.join("px.log");
        let log = std::fs::File::create(&log_path).map_err(|error| PxError::Spawn {
            program: program.to_path_buf(),
            reason: format!("could not create its log: {error}"),
        })?;
        let log_err = log.try_clone().map_err(|error| PxError::Spawn {
            program: program.to_path_buf(),
            reason: format!("could not create its log: {error}"),
        })?;
        #[cfg(unix)]
        let mut command = {
            let mut command = Command::new("/bin/sh");
            command
                .arg("-c")
                .arg(SUPERVISOR)
                .arg(program)
                .args(&args)
                .stdin(Stdio::piped());
            command
        };
        #[cfg(windows)]
        let mut command = {
            let mut command = Command::new(program);
            command.args(&args).stdin(Stdio::null());
            command
        };
        command
            .current_dir(scratch)
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_err));
        for name in PROXY_ENV {
            command.env_remove(name);
        }
        #[cfg(unix)]
        let spawned = command.group_spawn();
        #[cfg(windows)]
        let spawned = command
            .group()
            // CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW.
            .creation_flags(0x0000_0200 | 0x0800_0000)
            .spawn();
        let child = spawned.map_err(|error| PxError::Spawn {
            program: program.to_path_buf(),
            reason: error.to_string(),
        })?;
        let mut process = Self {
            child,
            #[cfg(unix)]
            liveness: None,
            address,
            reaped: false,
        };
        #[cfg(unix)]
        {
            process.liveness = process.child.inner().stdin.take();
        }
        let started = Instant::now();
        loop {
            if stopped() {
                let _ = process.kill_and_reap();
                return Err(PxError::Cancelled);
            }
            match process.child.try_wait() {
                Ok(Some(status)) => {
                    process.reaped = true;
                    return Err(PxError::Exited {
                        status: status.to_string(),
                        log: log_tail(&log_path),
                    });
                }
                Ok(None) => {}
                Err(error) => {
                    let _ = process.kill_and_reap();
                    return Err(PxError::Spawn {
                        program: program.to_path_buf(),
                        reason: error.to_string(),
                    });
                }
            }
            if TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_ok() {
                tracing::debug!(%address, "vibe-managed px is ready");
                return Ok(process);
            }
            if started.elapsed() > PX_READY_TIMEOUT {
                let _ = process.kill_and_reap();
                return Err(PxError::NotReady {
                    address,
                    waited: PX_READY_TIMEOUT,
                });
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    pub(crate) fn address(&self) -> SocketAddr {
        self.address
    }

    pub(crate) fn id(&self) -> u32 {
        self.child.id()
    }

    /// Terminate the group / job and wait until its leader is reaped.
    pub(crate) fn kill_and_reap(&mut self) -> Result<(), String> {
        #[cfg(unix)]
        self.liveness.take();
        if self.reaped {
            return Ok(());
        }
        let killed = self.child.kill().map_err(|error| error.to_string());
        let waited = self.child.wait().map_err(|error| error.to_string());
        self.reaped = waited.is_ok();
        match (killed, waited) {
            (_, Ok(_)) => Ok(()),
            (Err(kill), Err(wait)) => Err(format!("kill failed: {kill}; reap failed: {wait}")),
            (Ok(()), Err(wait)) => Err(format!("reap failed: {wait}")),
        }
    }
}

impl Drop for PxProcess {
    fn drop(&mut self) {
        if let Err(error) = self.kill_and_reap() {
            tracing::warn!("could not stop the vibe-managed px: {error}");
        }
    }
}

/// The last lines px wrote, bounded, for a failure message.
pub(crate) fn log_tail(path: &Path) -> String {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let start = lines.len().saturating_sub(5);
    let tail = lines[start..].join(" | ");
    tail.chars().take(600).collect()
}

/// `px --test=<target>` with the route settings, bounded by `timeout`:
/// `Ok(())` when px reports success (exit 0), else the exit and output tail.
pub(crate) fn self_test(
    program: &Path,
    settings: &PxSettings,
    no_proxy: Option<&str>,
    target: &str,
    timeout: Duration,
    scratch: &Path,
) -> Result<(), String> {
    let port = free_port().map_err(|error| error.to_string())?;
    let log_path = scratch.join("px-test.log");
    let log = std::fs::File::create(&log_path).map_err(|error| error.to_string())?;
    let log_err = log.try_clone().map_err(|error| error.to_string())?;
    let mut command = Command::new(program);
    command
        .arg("--listen=127.0.0.1")
        .arg(format!("--port={port}"))
        .args(route_args(settings, no_proxy).map_err(|error| error.to_string())?)
        .arg(format!("--test={target}"))
        .current_dir(scratch)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));
    for name in PROXY_ENV {
        command.env_remove(name);
    }
    let mut child = command
        .group_spawn()
        .map_err(|error| format!("could not run px: {error}"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err(format!("{status}: {}", log_tail(&log_path))),
            Ok(None) if started.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("no answer within {timeout:?}"));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
        }
    }
}

/// Who listens on loopback `port`, when this platform lets us see it.
#[cfg(target_os = "linux")]
fn holder_of(port: u16) -> Option<String> {
    let wanted = format!(":{port:04X}");
    let mut inodes = Vec::new();
    for table in ["/proc/net/tcp", "/proc/net/tcp6"] {
        let Ok(text) = std::fs::read_to_string(table) else {
            continue;
        };
        for line in text.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            // local_address, state 0A = LISTEN, inode
            if fields.len() > 9 && fields[1].ends_with(&wanted) && fields[3] == "0A" {
                inodes.push(format!("socket:[{}]", fields[9]));
            }
        }
    }
    if inodes.is_empty() {
        return None;
    }
    for entry in std::fs::read_dir("/proc").ok()?.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let Ok(fds) = std::fs::read_dir(entry.path().join("fd")) else {
            continue;
        };
        for fd in fds.flatten() {
            if let Ok(target) = std::fs::read_link(fd.path())
                && inodes
                    .iter()
                    .any(|inode| target.as_os_str() == inode.as_str())
            {
                let name = std::fs::read_to_string(entry.path().join("comm")).unwrap_or_default();
                return Some(format!("pid {pid} {}", name.trim()));
            }
        }
    }
    Some("another process".to_owned())
}

#[cfg(not(target_os = "linux"))]
fn holder_of(_port: u16) -> Option<String> {
    None
}
