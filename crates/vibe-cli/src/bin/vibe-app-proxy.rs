//! Small native PATH proxy. It never loads the application index or runs `vibe`.

use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[path = "vibe-app-proxy/supervision.rs"]
mod supervision;

const PROXY_PROTOCOL: &str = "vibe-app-proxy/1";
const CURRENT_PROTOCOL: &str = "vibe-app-current/1";
const MAX_SIDECAR_BYTES: u64 = 16 * 1024;
const MAX_CURRENT_BYTES: u64 = 1024;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
enum Proxy {
    Direct {
        protocol: String,
        target: PathBuf,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        cwd: Option<PathBuf>,
        lease: PathBuf,
    },
    Unavailable {
        protocol: String,
        reason: String,
    },
    Slot {
        protocol: String,
        slot: String,
        command: String,
        current: PathBuf,
    },
    View {
        protocol: String,
        slot: String,
        installation: String,
        command: String,
        current: PathBuf,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Current {
    protocol: String,
    projection: String,
}

struct Resolved {
    descriptor_path: PathBuf,
    descriptor: Proxy,
    selector: Option<(PathBuf, String)>,
}

fn main() {
    match run() {
        // `ExitCode` stores only one byte. Windows child processes can return
        // full DWORD status values (including negative exception codes).
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("vibe-app-proxy: {error:#}");
            std::process::exit(127)
        }
    }
}

fn run() -> Result<i32> {
    let mut args = std::env::args_os();
    let invoked = args.next().context("missing invocation name")?;
    let caller: Vec<OsString> = args.collect();
    if is_original_name(&invoked) && caller.len() == 1 && caller[0] == "--probe" {
        println!("{PROXY_PROTOCOL}");
        return Ok(0);
    }
    if is_original_name(&invoked) && caller.len() == 1 && caller[0] == "--version" {
        println!("vibe-app-proxy {}", env!("CARGO_PKG_VERSION"));
        return Ok(0);
    }
    let executable = invoked_path(&invoked)?;
    let sidecar = sidecar_path(&executable)?;
    let resolved = resolve(&sidecar)?;
    execute(&resolved, &caller)
}

fn is_original_name(name: &OsStr) -> bool {
    Path::new(name)
        .file_stem()
        .and_then(OsStr::to_str)
        .is_some_and(|stem| stem == "vibe-app-proxy")
}

#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "The standalone native proxy resolves its own argv0 against the caller's PATH at the process boundary; no application domain module receives ambient environment access."
)]
fn invoked_path(name: &OsStr) -> Result<PathBuf> {
    let path = Path::new(name);
    if path.components().count() > 1 || path.is_absolute() {
        let invoked = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };
        if !invoked.is_file() {
            bail!("invoked proxy does not exist: `{}`", invoked.display());
        }
        return Ok(invoked);
    }
    let search = std::env::var_os("PATH").context("PATH is unset")?;
    for directory in std::env::split_paths(&search) {
        let candidate = directory.join(name);
        if candidate.is_file() {
            // Keep the invoked filename: canonicalizing an alias symlink loses its sidecar.
            return Ok(candidate);
        }
        #[cfg(windows)]
        if candidate.extension().is_none() {
            let exe = candidate.with_extension("exe");
            if exe.is_file() {
                return Ok(exe);
            }
        }
    }
    bail!("cannot locate invoked proxy `{}` on PATH", path.display())
}

fn sidecar_path(executable: &Path) -> Result<PathBuf> {
    let filename = executable.file_name().context("proxy has no filename")?;
    let mut sidecar = filename.to_os_string();
    sidecar.push(".proxy.json");
    Ok(executable.with_file_name(sidecar))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path, limit: u64) -> Result<T> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("reading proxy descriptor `{}`", path.display()))?;
    if !metadata.file_type().is_file() || metadata.len() > limit {
        bail!(
            "proxy descriptor is not a bounded regular file: `{}`",
            path.display()
        );
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("proxy descriptor exceeds size limit: `{}`", path.display());
    }
    serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing proxy descriptor `{}`", path.display()))
}

fn resolve(alias_sidecar: &Path) -> Result<Resolved> {
    let alias: Proxy = read_json(alias_sidecar, MAX_SIDECAR_BYTES)?;
    check_protocol(&alias)?;
    match alias {
        Proxy::Slot {
            slot,
            command,
            current,
            ..
        } => {
            if !portable_token(&slot) || !portable_token(&command) {
                bail!("slot proxy has an invalid slot or command name");
            }
            let slot_dir = slot_directory(&current, &slot)?;
            resolve_pointer(&current, &slot_dir, &command)
        }
        Proxy::View {
            slot,
            installation,
            command,
            current,
            ..
        } => {
            if !portable_token(&slot) || !portable_token(&command) || installation.is_empty() {
                bail!("view proxy has an invalid slot, installation, or command");
            }
            let view_dir = view_directory(&current, &slot, &installation)?;
            resolve_pointer(&current, &view_dir, &command)
        }
        descriptor => Ok(Resolved {
            descriptor_path: alias_sidecar.to_path_buf(),
            descriptor,
            selector: None,
        }),
    }
}

fn resolve_pointer(current: &Path, base: &Path, command: &str) -> Result<Resolved> {
    let pointer: Current = read_json(current, MAX_CURRENT_BYTES)?;
    if pointer.protocol != CURRENT_PROTOCOL || !projection_id(&pointer.projection) {
        bail!("proxy current pointer is invalid");
    }
    let selected_projection = pointer.projection;
    let descriptor_path = base
        .join("projections")
        .join(&selected_projection)
        .join("commands")
        .join(format!("{command}.json"));
    let descriptor: Proxy = read_json(&descriptor_path, MAX_SIDECAR_BYTES)?;
    check_protocol(&descriptor)?;
    if !matches!(descriptor, Proxy::Direct { .. } | Proxy::Unavailable { .. }) {
        bail!("projection must contain a direct or unavailable descriptor");
    }
    Ok(Resolved {
        descriptor_path,
        descriptor,
        selector: Some((current.to_path_buf(), selected_projection)),
    })
}

fn check_protocol(descriptor: &Proxy) -> Result<()> {
    let protocol = match descriptor {
        Proxy::Direct { protocol, .. }
        | Proxy::Unavailable { protocol, .. }
        | Proxy::Slot { protocol, .. }
        | Proxy::View { protocol, .. } => protocol,
    };
    if protocol != PROXY_PROTOCOL {
        bail!("unsupported proxy descriptor protocol");
    }
    Ok(())
}

fn slot_directory(current: &Path, slot: &str) -> Result<PathBuf> {
    if !clean_absolute(current) || current.file_name() != Some(OsStr::new("current.json")) {
        bail!("slot current pointer must be an absolute current.json path");
    }
    let slot_dir = current
        .parent()
        .context("slot current pointer has no parent")?;
    if slot_dir.file_name() != Some(OsStr::new(slot))
        || slot_dir.parent().and_then(Path::file_name) != Some(OsStr::new("alternatives"))
        || slot_dir
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            != Some(OsStr::new("opt"))
    {
        bail!("slot current pointer is outside its alternatives directory");
    }
    Ok(slot_dir.to_path_buf())
}

fn view_directory(current: &Path, slot: &str, installation: &str) -> Result<PathBuf> {
    if !clean_absolute(current) || current.file_name() != Some(OsStr::new("current.json")) {
        bail!("view current pointer must be an absolute current.json path");
    }
    let view = current
        .parent()
        .context("view current pointer has no parent")?;
    let encoded = format!("{:x}", Sha256::digest(installation.as_bytes()));
    if view.file_name() != Some(OsStr::new(&encoded))
        || view.parent().and_then(Path::file_name) != Some(OsStr::new("installations"))
    {
        bail!("view current pointer differs from its installation key");
    }
    let slot_dir = view
        .parent()
        .and_then(Path::parent)
        .context("view has no slot directory")?;
    let slot_current = slot_dir.join("current.json");
    slot_directory(&slot_current, slot)?;
    Ok(view.to_path_buf())
}

fn portable_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

fn projection_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn clean_absolute(path: &Path) -> bool {
    path.is_absolute()
        && !path
            .components()
            .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
}

fn validate_direct(target: &Path, cwd: Option<&Path>, lease: &Path) -> Result<()> {
    if !clean_absolute(target)
        || !clean_absolute(lease)
        || cwd.is_some_and(|cwd| !clean_absolute(cwd))
    {
        bail!("direct proxy paths must be absolute without traversal");
    }
    if target
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "cmd" | "bat" | "ps1"))
    {
        bail!("script targets are unsupported by native application proxies");
    }
    if !target.is_file() {
        bail!("direct proxy target is not a file: `{}`", target.display());
    }
    let name = lease.file_name().and_then(OsStr::to_str).unwrap_or("");
    let digest = name.strip_suffix(".lock").unwrap_or("");
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || lease.parent().and_then(Path::file_name) != Some(OsStr::new("leases"))
        || lease
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            != Some(OsStr::new("applications"))
    {
        bail!("direct proxy lease path is invalid");
    }
    Ok(())
}

fn direct_command(
    target: &Path,
    args: &[String],
    cwd: Option<&Path>,
    caller: &[OsString],
) -> Command {
    let mut command = Command::new(target);
    command.args(args).args(caller);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command
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

fn execute(resolved: &Resolved, caller: &[OsString]) -> Result<i32> {
    match &resolved.descriptor {
        Proxy::Unavailable { reason, .. } => {
            if reason.trim().is_empty() {
                bail!("unavailable proxy has no reason");
            }
            eprintln!("vibe-app-proxy: {reason}");
            Ok(127)
        }
        Proxy::Direct {
            target,
            args,
            cwd,
            lease,
            ..
        } => {
            validate_direct(target, cwd.as_deref(), lease)?;
            let lock = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(lease)
                .with_context(|| format!("opening application lease `{}`", lease.display()))?;
            lock.lock_shared()
                .context("acquiring shared application lease")?;
            let latest: Proxy = read_json(&resolved.descriptor_path, MAX_SIDECAR_BYTES)?;
            if latest != resolved.descriptor {
                bail!("proxy descriptor changed while acquiring its lease");
            }
            if let Some((current, expected)) = &resolved.selector {
                let selected: Current = read_json(current, MAX_CURRENT_BYTES)?;
                if selected.protocol != CURRENT_PROTOCOL || selected.projection != *expected {
                    bail!("selected application alternative changed while acquiring its lease");
                }
            }
            let mut child = direct_command(target, args, cwd.as_deref(), caller);
            child
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            // Register before spawning: a terminal SIGINT reaches the child
            // and the proxy, but the proxy must retain its lease until the
            // child actually exits. A direct SIGTERM to the proxy terminates
            // the child instead of abandoning a live payload. On Windows the
            // console broadcasts Ctrl+C/Break to both processes; the proxy's
            // handler consumes its own copy and keeps supervising the child.
            #[cfg(windows)]
            let mut supervisor =
                supervision::Supervisor::install().context("installing console control handler")?;
            #[cfg(not(windows))]
            let mut supervisor = supervision::Supervisor::install()?;
            let child = child
                .spawn()
                .with_context(|| format!("running `{}`", target.display()))?;
            let status = supervisor.wait(child)?;
            // `lock` remains alive until the child has exited.
            drop(lock);
            Ok(status_code(status))
        }
        Proxy::Slot { .. } | Proxy::View { .. } => bail!("unresolved pointer proxy"),
    }
}

#[cfg(test)]
#[path = "vibe-app-proxy/tests.rs"]
mod tests;
