//! The vibeterm control-plane client for `vibe aiui` (PROP-042 §4): read the
//! discovery file a `--control` vibeterm writes to `~/.vibevm/aiui/`, then drive
//! it over loopback HTTP+JSON (token-guarded) — open / send / snapshot / wait /
//! close. Blocking `reqwest`; no daemon, no ambient network.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-042#aiui-cli");

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};

use crate::cli::{
    AiuiOpenArgs, AiuiScrollbarArgs, AiuiSendArgs, AiuiSessionArgs, AiuiSnapshotArgs, AiuiWaitArgs,
    ScrollbarMode,
};

/// A running vibeterm control session's discovery info. `pub(super)` so the CDP
/// `inspect` sibling can read the same session's fields.
pub(super) struct Discovery {
    pub(super) port: u16,
    pub(super) token: String,
    pub(super) pid: u32,
    /// `Date.now()` (epoch ms) when vibeterm wrote the file — used to tell our
    /// freshly-spawned session apart from a stale `latest.json`.
    pub(super) started_at: u128,
    /// The Chrome DevTools Protocol port, when vibeterm opened a
    /// `--remote-debugging-port` endpoint for `vibe aiui inspect` (PROP-042 §4).
    /// `None` for a session whose vibeterm predates `--cdp-port`.
    pub(super) cdp_port: Option<u16>,
}

/// `vibe aiui open`: launch vibeterm with its control server, wait for the
/// discovery file, and print the session id (the vibeterm pid).
pub(super) fn open(a: AiuiOpenArgs) -> Result<()> {
    let exec = match a.exec {
        Some(cmd) => cmd,
        None => default_tree_exec(),
    };
    let (cols, rows) = match a.size.as_deref() {
        Some(s) => super::parse_size(s)?,
        // A headless session has no window to fit, so the grid size is fixed
        // here rather than measured. Default to a roomy grid so the tree's
        // columns and box-drawing read cleanly in a snapshot; override --size.
        None => (120, 40),
    };
    // Capture the wall clock *before* spawning: the discovery poll accepts
    // `latest.json` only once its `startedAt` is at or after this instant, so a
    // stale file from a previous session cannot be mistaken for ours. (The pid
    // vibeterm reports is not the pid we spawn — Electron's launcher forks the
    // real main process — so we cannot target a `<pid>.json` by the child id.)
    let since_ms = now_ms();
    // Control server always on; headless unless the caller asks to watch it live.
    let child =
        super::super::term::spawn_vibeterm(&exec, Some(cols), Some(rows), true, !a.visible)?;
    // The child handle is dropped (detached): vibeterm owns its own lifetime and
    // is torn down via `vibe aiui close`, not by this process.
    drop(child);
    let disc = wait_for_discovery(since_ms, a.timeout_ms)?;
    // The session id is the vibeterm pid; later verbs default to the latest.
    println!("{}", disc.pid);
    Ok(())
}

/// `vibe aiui send`: inject key names and/or literal text into the session.
pub(super) fn send(a: AiuiSendArgs) -> Result<()> {
    let disc = read_discovery(a.session)?;
    let mut body = json!({ "keys": a.keys });
    if let Some(text) = a.text {
        body["text"] = json!(text);
    }
    post(&disc, "/input", body)?;
    Ok(())
}

/// `vibe aiui snapshot`: print the running terminal's symbolic text grid, or —
/// with `--png <path>` — write a PNG screenshot of the live window (the visual
/// ground truth) to that path and print it.
pub(super) fn snapshot(a: AiuiSnapshotArgs) -> Result<()> {
    let disc = read_discovery(a.session)?;
    if let Some(mut path) = a.png {
        // The server writes the PNG to `path`; resolve it absolute first so it
        // lands where the caller expects regardless of vibeterm's own cwd.
        if !path.is_absolute() {
            path = std::env::current_dir()?.join(&path);
        }
        let body = post(&disc, "/capture", json!({ "path": path.to_string_lossy() }))?;
        println!(
            "{}",
            body["path"].as_str().unwrap_or(&path.to_string_lossy())
        );
        return Ok(());
    }
    let body = get(&disc, "/snapshot?format=text")?;
    print!("{}", body["text"].as_str().unwrap_or(""));
    Ok(())
}

/// `vibe aiui wait`: block until the session's PTY is quiet (deterministic
/// snapshots after driving input).
pub(super) fn wait(a: AiuiWaitArgs) -> Result<()> {
    let disc = read_discovery(a.session)?;
    post(
        &disc,
        "/wait",
        json!({ "idleMs": a.idle_ms, "timeoutMs": a.timeout_ms }),
    )?;
    Ok(())
}

/// `vibe aiui close`: tell the session to tear down.
pub(super) fn close(a: AiuiSessionArgs) -> Result<()> {
    let disc = read_discovery(a.session)?;
    // The server quits mid-response; a transport error on the reply is expected.
    let _ = post(&disc, "/close", json!({}));
    Ok(())
}

/// `vibe aiui pty-stop`: stop the hosted program only (NOT Electron). The
/// renderer + CDP endpoint stay live; the binary is freed for a rebuild.
pub(super) fn pty_stop(a: AiuiSessionArgs) -> Result<()> {
    let disc = read_discovery(a.session)?;
    post(&disc, "/pty-stop", json!({}))?;
    Ok(())
}

/// `vibe aiui pty-start`: (re)spawn the hosted program at the current grid.
/// Pairs with `pty-stop` around a rebuild for a live TUI preview loop.
pub(super) fn pty_start(a: AiuiSessionArgs) -> Result<()> {
    let disc = read_discovery(a.session)?;
    let v = post(&disc, "/pty-start", json!({}))?;
    println!(
        "pty started: cols={} rows={}",
        v.get("cols").and_then(|x| x.as_u64()).unwrap_or(0),
        v.get("rows").and_then(|x| x.as_u64()).unwrap_or(0),
    );
    Ok(())
}

/// `vibe aiui scrollbar <mode>`: flip the scrollbar policy live.
pub(super) fn scrollbar(a: AiuiScrollbarArgs) -> Result<()> {
    let disc = read_discovery(a.session)?;
    let mode = match a.mode {
        ScrollbarMode::Auto => "auto",
        ScrollbarMode::On => "on",
        ScrollbarMode::Off => "off",
    };
    post(&disc, "/scrollbar", json!({ "mode": mode }))?;
    Ok(())
}

/// The default `--exec` for `vibe aiui open`: the console `vibe tree` over the
/// directory `vibe aiui` was run from, by this same binary. The cwd is resolved
/// to an absolute path (vibeterm runs with its own cwd) and both the binary and
/// the path are quoted (vibeterm's `splitCommand` tokenises quoted arguments).
fn default_tree_exec() -> String {
    let exe = std::env::current_exe()
        .ok()
        .map(|p| super::super::term::quote_exe(&p.to_string_lossy()))
        .unwrap_or_else(|| "vibe".to_string());
    let cwd = std::env::current_dir()
        .ok()
        .map(|p| super::super::term::quote_exe(&p.to_string_lossy()))
        .unwrap_or_else(|| ".".to_string());
    format!("{exe} tree --path {cwd} -c")
}

/// `~/.vibevm/aiui/` — where vibeterm writes its per-session discovery files.
fn aiui_dir() -> PathBuf {
    home().join(".vibevm").join("aiui")
}

/// The user's home directory (`HOME`, then `USERPROFILE` on Windows).
fn home() -> PathBuf {
    if let Some(h) = std::env::var_os("HOME").filter(|s| !s.is_empty()) {
        return PathBuf::from(h);
    }
    if cfg!(windows)
        && let Some(p) = std::env::var_os("USERPROFILE").filter(|s| !s.is_empty())
    {
        return PathBuf::from(p);
    }
    PathBuf::from(".")
}

/// Read a session's discovery file — `<pid>.json` for an explicit session, else
/// `latest.json` (the most recently opened). `pub(super)` so the CDP `inspect`
/// sibling reads the same session.
pub(super) fn read_discovery(session: Option<u32>) -> Result<Discovery> {
    let file = match session {
        Some(pid) => aiui_dir().join(format!("{pid}.json")),
        None => aiui_dir().join("latest.json"),
    };
    let raw = std::fs::read_to_string(&file).map_err(|_| {
        anyhow!(
            "no vibeterm control session (`{}` not found) — run `vibe aiui open` first",
            file.display()
        )
    })?;
    parse_discovery(&raw)
}

/// Poll `latest.json` until a session whose `startedAt >= since_ms` appears (our
/// freshly-spawned vibeterm) or the deadline passes. Watching `latest.json`
/// rather than a `<pid>.json` sidesteps the Electron launcher's pid indirection;
/// the freshness gate rejects a stale pointer from an earlier session.
fn wait_for_discovery(since_ms: u128, timeout_ms: u64) -> Result<Discovery> {
    let file = aiui_dir().join("latest.json");
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if let Ok(raw) = std::fs::read_to_string(&file)
            && let Ok(disc) = parse_discovery(&raw)
            && disc.started_at >= since_ms
        {
            return Ok(disc);
        }
        if Instant::now() >= deadline {
            bail!("vibeterm's control server did not come up within {timeout_ms} ms");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Parse a discovery JSON blob into a [`Discovery`].
fn parse_discovery(raw: &str) -> Result<Discovery> {
    let v: Value = serde_json::from_str(raw).map_err(|e| anyhow!("bad discovery JSON: {e}"))?;
    let port = v["port"]
        .as_u64()
        .ok_or_else(|| anyhow!("discovery: missing `port`"))?;
    let token = v["token"]
        .as_str()
        .ok_or_else(|| anyhow!("discovery: missing `token`"))?
        .to_string();
    let pid = v["pid"].as_u64().unwrap_or(0);
    let started_at = v["startedAt"].as_u64().unwrap_or(0);
    let cdp_port = v.get("cdpPort").and_then(|x| x.as_u64()).map(|p| p as u16);
    Ok(Discovery {
        port: port as u16,
        token,
        pid: pid as u32,
        started_at: started_at as u128,
        cdp_port,
    })
}

/// Wall-clock milliseconds since the Unix epoch (a monotonic-enough stamp to
/// pair with vibeterm's `Date.now()` in the discovery file).
fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// A blocking HTTP client with a short timeout.
fn client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| anyhow!("building the HTTP client: {e}"))
}

/// `GET <path>` against the session, token-authenticated; JSON in, JSON out.
fn get(disc: &Discovery, path: &str) -> Result<Value> {
    let url = format!("http://127.0.0.1:{}{path}", disc.port);
    let resp = client()?.get(&url).bearer_auth(&disc.token).send()?;
    finish(path, resp)
}

/// `POST <path>` with a JSON body against the session, token-authenticated.
fn post(disc: &Discovery, path: &str, body: Value) -> Result<Value> {
    let url = format!("http://127.0.0.1:{}{path}", disc.port);
    let resp = client()?
        .post(&url)
        .bearer_auth(&disc.token)
        .json(&body)
        .send()?;
    finish(path, resp)
}

/// Turn a response into JSON, surfacing a non-2xx as an error with the body.
fn finish(path: &str, resp: reqwest::blocking::Response) -> Result<Value> {
    let status = resp.status();
    let body: Value = resp
        .json()
        .unwrap_or_else(|_| json!({ "error": "non-JSON response" }));
    if !status.is_success() {
        bail!("vibeterm control `{path}`: {status} — {body}");
    }
    Ok(body)
}
