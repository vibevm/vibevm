//! The native PATH proxy (`spec://org.vibevm.core/vibevm/common/PROP-059#proxy`).
//!
//! Every reserved command in `<settings>/opt/bin` is a byte copy of this
//! binary. Invoked as `<command>`, it finds its home (the nearest ancestor of
//! its own directory holding `applications/index.json`), picks the slot's
//! provider from `VIBE_APP_USE` or the slot default, resolves that provider's
//! launcher for `<command>` and runs the native program directly, forwarding
//! arguments, the exit status and, on POSIX, signals. It never runs `vibe`
//! and depends only on std, serde, serde_json, toml and the generated
//! `vibe-wire` index types. Any failure is one stderr line and exit 127.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[path = "vibe-app-proxy/resolve.rs"]
mod resolve;
#[path = "vibe-app-proxy/supervision.rs"]
mod supervision;

const PROXY_PROTOCOL: &str = "vibe-app-proxy/2";
const ORIGINAL_NAME: &str = "vibe-app-proxy";
const SESSION_VARIABLE: &str = "VIBE_APP_USE";

fn main() {
    match run() {
        // `ExitCode` stores only one byte. Windows child processes can return
        // full DWORD status values (including negative exception codes).
        Ok(code) => std::process::exit(code),
        Err(reason) => {
            eprintln!("vibe-app-proxy: {reason}");
            std::process::exit(127)
        }
    }
}

fn run() -> Result<i32, String> {
    let caller: Vec<OsString> = std::env::args_os().skip(1).collect();
    let exe = own_path()?;
    let command = exe
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or_else(|| format!("own file name `{}` is not a command", exe.display()))?
        .to_owned();
    if command == ORIGINAL_NAME {
        return original(&caller);
    }
    let session = session_value()?;
    let launch = resolve::resolve(&exe, &command, session.as_deref())?;
    execute(&launch, &caller)
}

/// Capability queries answer only under the original image name.
fn original(caller: &[OsString]) -> Result<i32, String> {
    match caller {
        [flag] if flag == "--probe" => println!("{PROXY_PROTOCOL}"),
        [flag, protocol] if flag == "--supports-index" => match protocol.to_str() {
            Some(
                "vibe-user-applications/3"
                | "vibe-user-applications/4"
                | "vibe-user-applications/5"
                | "vibe-user-applications/6"
                | "vibe-user-applications/7"
                | "vibe-user-applications/8",
            ) => {
                println!("{}", protocol.to_string_lossy());
            }
            _ => return Err("unsupported application index protocol".into()),
        },
        [flag] if flag == "--version" => println!("vibe-app-proxy {}", env!("CARGO_PKG_VERSION")),
        _ => {
            return Err(
                "this is the PATH proxy image; run an application command from `opt/bin` instead"
                    .into(),
            );
        }
    }
    Ok(0)
}

fn own_path() -> Result<PathBuf, String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("cannot locate its own executable: {error}"))?;
    Ok(std::fs::canonicalize(&exe)
        .map(resolve::strip_verbatim)
        .unwrap_or(exe))
}

#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "The standalone PATH proxy reads the session choice VIBE_APP_USE once at its process boundary (spec://org.vibevm.core/vibevm/common/PROP-059#session-selection); resolution receives it as a value."
)]
fn session_value() -> Result<Option<String>, String> {
    std::env::var_os(SESSION_VARIABLE)
        .map(|value| {
            value
                .into_string()
                .map_err(|_| format!("{SESSION_VARIABLE} is not valid UTF-8"))
        })
        .transpose()
}

fn command_for(launch: &resolve::Launch, caller: &[OsString]) -> Command {
    let mut command = Command::new(&launch.executable);
    command
        .args(resolve::operands(&launch.args, caller))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(directory) = &launch.working_directory {
        command.current_dir(directory);
    }
    // A fall-through hands the chain it already walked to the child, so a
    // second proxy for the same switched-off slot keeps moving forward
    // along PATH (`PROP-059#slot-off`).
    if let Some(skip) = &launch.skip {
        command.env(resolve::SKIP_VARIABLE, skip);
    }
    command
}

fn execute(launch: &resolve::Launch, caller: &[OsString]) -> Result<i32, String> {
    // Register before spawning: a terminal SIGINT reaches both the child and
    // the proxy, and the proxy keeps supervising until the child exits; a
    // SIGTERM to the proxy is forwarded to the child. On Windows the console
    // broadcasts Ctrl+C/Break to both processes and the proxy's handler
    // consumes its own copy.
    let mut supervisor = supervision::Supervisor::install()
        .map_err(|error| format!("installing signal supervision failed: {error}"))?;
    let child = command_for(launch, caller)
        .spawn()
        .map_err(|error| format!("running `{}` failed: {error}", launch.executable.display()))?;
    let status = supervisor.wait(child).map_err(|error| {
        format!(
            "waiting for `{}` failed: {error}",
            launch.executable.display()
        )
    })?;
    Ok(status_code(status))
}

fn status_code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    if let Some(signal) = status.signal() {
        return 128 + signal;
    }
    1
}

#[cfg(test)]
#[path = "vibe-app-proxy/tests.rs"]
mod tests;
