//! Stage exact distribution bytes for a later native launch probe.
//!
//! Staging is not execution evidence. The held temporary directory keeps the
//! verified executable available without writing to the user's real home.

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use tempfile::TempDir;
use vibe_publish::{
    DISTRIBUTION_BUNDLE_MAX_BYTES, DistributionComponentName, PlatformDistributionFragment,
    SUPPORTED_DISTRIBUTION_TARGETS, sha256_digest,
};
use vibe_wire::generated::publication_subject::PublicationSmokeProbeStatus;
use vibe_wire::generated::publication_subject::PublicationTarget;

use super::build::verified_bundle_entries;

mod report_store;

/// A verified, locally staged bundle. No command has been run in this home.
#[allow(dead_code)]
pub(crate) struct StagedBundle {
    hold: TempDir,
    pub(crate) target: String,
    pub(crate) bundle_digest: String,
    pub(crate) home: PathBuf,
    pub(crate) executable: PathBuf,
    executable_size: u64,
    executable_digest: String,
}

/// Check one held bundle against both its fragment and publication target,
/// then copy only the two declared executables into a fresh temporary home.
/// The caller must still run the exact staged executable on a compatible
/// native host and produce a separate smoke result.
#[allow(dead_code)]
pub(crate) fn stage_verified_bundle(
    bundle_bytes: &[u8],
    expected_target: &str,
    fragment: &PlatformDistributionFragment,
    subject: &PublicationTarget,
) -> Result<StagedBundle> {
    fragment
        .validate()
        .context("validating platform fragment")?;
    ensure!(
        fragment.version == "1.1.0",
        "smoke stage requires mutable 1.1.0, found `{}`",
        fragment.version
    );
    ensure!(
        SUPPORTED_DISTRIBUTION_TARGETS.contains(&expected_target)
            && fragment.target == expected_target
            && subject.target == expected_target,
        "smoke target differs from the supported fragment/subject target"
    );
    let expected_name = format!("vibevm-1.1.0-{expected_target}.zip");
    ensure!(
        fragment.asset.name == expected_name
            && subject.bundle.name == expected_name
            && subject.bundle.media_type == "application/zip",
        "smoke bundle name or media type differs from the exact target"
    );
    ensure!(
        fragment.asset.size <= DISTRIBUTION_BUNDLE_MAX_BYTES
            && u64::try_from(bundle_bytes.len()).ok() == Some(fragment.asset.size)
            && subject.bundle.size_bytes == fragment.asset.size.to_string(),
        "smoke bundle size differs from fragment/subject"
    );
    let digest = sha256_digest(bundle_bytes);
    ensure!(
        digest == fragment.asset.digest && digest == subject.bundle.sha256,
        "smoke bundle SHA-256 differs from fragment/subject"
    );
    let bootstrap_name = if expected_target == "x86_64-pc-windows-msvc" {
        format!("vibe-bootstrap-{expected_target}.exe")
    } else {
        format!("vibe-bootstrap-{expected_target}")
    };
    ensure!(
        fragment.bootstrap.name == bootstrap_name
            && subject.bootstrap.name == bootstrap_name
            && subject.bootstrap.media_type == "application/octet-stream"
            && subject.bootstrap.size_bytes == fragment.bootstrap.size.to_string()
            && subject.bootstrap.sha256 == fragment.bootstrap.digest,
        "smoke bootstrap identity differs from fragment/subject"
    );
    let fragment_bytes = fragment
        .to_json_bytes()
        .context("serializing exact platform fragment")?;
    ensure!(
        subject.fragment.name == format!("vibevm-1.1.0-{expected_target}.fragment.json")
            && subject.fragment.media_type == "application/json"
            && subject.fragment.size_bytes == fragment_bytes.len().to_string()
            && subject.fragment.sha256 == sha256_digest(&fragment_bytes),
        "smoke fragment bytes differ from publication subject"
    );
    ensure!(
        subject.components.len() == fragment.bundle.components.len(),
        "smoke component set differs from publication subject"
    );
    for (actual, expected) in fragment.bundle.components.iter().zip(&subject.components) {
        ensure!(
            actual.name.as_str() == match expected.name {
                vibe_wire::generated::publication_subject::PublicationComponentName::Vibe => "vibe",
                vibe_wire::generated::publication_subject::PublicationComponentName::VibeIndex =>
                    "vibe-index",
            } && actual.path == expected.path
                && actual.size.to_string() == expected.size_bytes
                && actual.digest == expected.sha256,
            "smoke component differs from publication subject"
        );
    }
    let archive = &fragment.bundle.source_archive;
    ensure!(
        archive.path == subject.source_archive.path
            && archive.size.to_string() == subject.source_archive.size_bytes
            && archive.digest == subject.source_archive.sha256
            && archive.tree_oid == subject.source_archive.tree_oid,
        "smoke source archive differs from publication subject"
    );

    // This bounded reader rejects traversal, symlinks, undeclared files and
    // wrong embedded component/source/manifest digests before any file write.
    let entries = verified_bundle_entries(bundle_bytes, &fragment.bundle)
        .context("verifying closed distribution ZIP before staging")?;
    let vibe = fragment
        .bundle
        .components
        .iter()
        .find(|component| component.name == DistributionComponentName::Vibe)
        .context("validated bundle lacks vibe")?;
    let embedded_vibe = entries
        .iter()
        .find(|entry| entry.name == vibe.path)
        .context("verified ZIP lacks vibe")?;
    ensure!(
        embedded_vibe.bytes.len() as u64 == fragment.bootstrap.size
            && sha256_digest(&embedded_vibe.bytes) == fragment.bootstrap.digest,
        "embedded vibe differs from raw bootstrap identity"
    );

    let hold = tempfile::Builder::new()
        .prefix("vibevm-smoke-")
        .tempdir()
        .context("creating fresh isolated smoke home")?;
    let home = hold.path().to_path_buf();
    let bin = home.join("bin");
    fs::create_dir(&bin).context("creating isolated smoke bin")?;
    let mut executable = None;
    for component in &fragment.bundle.components {
        // Fragment validation constrains these paths to fixed basenames. The
        // explicit check keeps the staging boundary safe if that rule changes.
        let path = Path::new(&component.path);
        ensure!(
            path.file_name()
                .is_some_and(|name| name == path.as_os_str())
                && matches!(
                    component.path.as_str(),
                    "vibe" | "vibe.exe" | "vibe-index" | "vibe-index.exe"
                ),
            "smoke executable path is not a fixed basename"
        );
        let entry = entries
            .iter()
            .find(|entry| entry.name == component.path)
            .context("verified ZIP lacks declared executable")?;
        let destination = bin.join(path);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .with_context(|| format!("creating staged executable `{}`", destination.display()))?;
        file.write_all(&entry.bytes)
            .with_context(|| format!("staging executable `{}`", destination.display()))?;
        file.sync_all().context("sealing staged executable bytes")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&destination, fs::Permissions::from_mode(0o700))
                .context("marking staged executable runnable")?;
        }
        if component.name == DistributionComponentName::Vibe {
            executable = Some(destination);
        }
    }
    let executable = executable.ok_or_else(|| anyhow::anyhow!("staged bundle lacks vibe"))?;
    Ok(StagedBundle {
        hold,
        target: expected_target.to_string(),
        bundle_digest: digest,
        home,
        executable,
        executable_size: vibe.size,
        executable_digest: vibe.digest.clone(),
    })
}

const MAX_CAPTURE_BYTES: usize = 16 * 1024;
const MAX_PROBE_DURATION: Duration = Duration::from_secs(30);

/// Local probe data only. A publisher must persist/review this record before
/// deriving any subject `evidence_ref`; a unit-test fixture is never release
/// evidence merely because this method returns Passed.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct NativeProbeRecord {
    pub(crate) target: String,
    pub(crate) host: String,
    pub(crate) status: PublicationSmokeProbeStatus,
    pub(crate) bundle_digest: String,
    pub(crate) tested_sha256: Option<String>,
    pub(crate) runner_ref: Option<String>,
    pub(crate) executable_sha256: Option<String>,
    pub(crate) reason: String,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) elapsed_ms: u128,
    pub(crate) output_truncated: bool,
}

fn host_identity() -> String {
    let abi = if std::env::consts::OS == "linux" {
        if cfg!(target_env = "musl") {
            "/musl"
        } else {
            "/gnu"
        }
    } else {
        ""
    };
    format!("{}/{}{}", std::env::consts::OS, std::env::consts::ARCH, abi)
}

fn native_compatible(target: &str) -> bool {
    matches!(
        (std::env::consts::OS, std::env::consts::ARCH, target),
        ("windows", "x86_64", "x86_64-pc-windows-msvc")
            | ("linux", "x86_64", "x86_64-unknown-linux-musl")
            | ("linux", "x86_64", "x86_64-unknown-linux-gnu")
            | ("macos", "x86_64", "x86_64-apple-darwin")
            | ("macos", "aarch64", "aarch64-apple-darwin")
    )
}

fn probe_record(
    stage: &StagedBundle,
    status: PublicationSmokeProbeStatus,
    reason: &str,
) -> NativeProbeRecord {
    NativeProbeRecord {
        target: stage.target.clone(),
        host: host_identity(),
        status,
        bundle_digest: stage.bundle_digest.clone(),
        tested_sha256: None,
        runner_ref: None,
        executable_sha256: None,
        reason: reason.to_string(),
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
        elapsed_ms: 0,
        output_truncated: false,
    }
}

struct BoundedOutput {
    bytes: Vec<u8>,
    truncated: bool,
    read_error: bool,
}

fn drain_bounded<R: Read + Send + 'static>(
    mut reader: R,
    kind: &'static str,
    tx: mpsc::Sender<(&'static str, BoundedOutput)>,
) {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 4096];
        let mut truncated = false;
        let mut read_error = false;
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    let retain = n.min(MAX_CAPTURE_BYTES.saturating_sub(bytes.len()));
                    bytes.extend_from_slice(&chunk[..retain]);
                    truncated |= retain < n;
                }
                Err(_) => {
                    read_error = true;
                    break;
                }
            }
        }
        let _ = tx.send((
            kind,
            BoundedOutput {
                bytes,
                truncated,
                read_error,
            },
        ));
    });
}

/// Execute only on a compatible native host. The tested bytes are checked
/// immediately before spawning the exact staged path. The isolated home and
/// captured outputs are bounded; no source tests or publication effects run.
#[allow(dead_code)]
pub(crate) fn run_native_probe(stage: &StagedBundle, timeout: Duration) -> NativeProbeRecord {
    let mut result = probe_record(
        stage,
        PublicationSmokeProbeStatus::Unavailable,
        "native executor unavailable",
    );
    if !native_compatible(&stage.target) {
        result.reason = format!(
            "foreign native target {}; host {}",
            stage.target, result.host
        );
        return result;
    }
    let timeout = timeout.min(MAX_PROBE_DURATION);
    if timeout.is_zero() {
        result.reason = "zero probe timeout".into();
        return result;
    }
    let project = match vibe_safefs::Project::open(&stage.home) {
        Ok(project) => project,
        Err(error) => {
            result.reason = format!("isolated home unavailable: {error}");
            return result;
        }
    };
    let relative = match stage.executable.strip_prefix(&stage.home) {
        Ok(relative) => relative,
        Err(_) => {
            result.reason = "staged executable escaped isolated home".into();
            return result;
        }
    };
    let relative = relative.to_string_lossy().replace('\\', "/");
    let state = match project
        .stable_file_state_bounded(&relative, stage.executable_size.saturating_add(1))
    {
        Ok(Some(state)) => state,
        Ok(None) => {
            result.reason = "staged executable disappeared".into();
            return result;
        }
        Err(error) => {
            result.reason = format!("staged executable cannot be measured: {error}");
            return result;
        }
    };
    let measured = format!("sha256:{}", state.sha256);
    result.executable_sha256 = Some(measured.clone());
    if state.bytes != stage.executable_size || measured != stage.executable_digest {
        result.reason = "staged executable differs from verified bundle component".into();
        return result;
    }

    let mut command = Command::new(&stage.executable);
    command
        .arg("--version")
        .current_dir(&stage.home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear();
    for inherited in ["PATH", "SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(inherited) {
            command.env(inherited, value);
        }
    }
    for name in [
        "HOME",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "TMP",
        "TEMP",
        "TMPDIR",
        "VIBE_HOME",
        "VIBEVM_HOME",
    ] {
        command.env(name, &stage.home);
    }
    let started = Instant::now();
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            result.reason = format!("native process spawn failed: {error}");
            return result;
        }
    };
    result.status = PublicationSmokeProbeStatus::Failed;
    result.tested_sha256 = Some(stage.bundle_digest.clone());
    result.runner_ref = Some(format!("system-native/{}", result.host));
    let (tx, rx) = mpsc::channel();
    if let Some(stdout) = child.stdout.take() {
        drain_bounded(stdout, "stdout", tx.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        drain_bounded(stderr, "stderr", tx.clone());
    }
    drop(tx);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() >= timeout => {
                timed_out = true;
                let _ = child.kill();
                break child.wait().ok();
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(_) => {
                let _ = child.kill();
                break child.wait().ok();
            }
        }
    };
    result.elapsed_ms = started.elapsed().as_millis();
    result.exit_code = status.and_then(|value| value.code());
    let mut capture_incomplete = false;
    for _ in 0..2 {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(("stdout", output)) => {
                result.stdout = String::from_utf8_lossy(&output.bytes).into_owned();
                result.output_truncated |= output.truncated;
                capture_incomplete |= output.read_error;
            }
            Ok(("stderr", output)) => {
                result.stderr = String::from_utf8_lossy(&output.bytes).into_owned();
                result.output_truncated |= output.truncated;
                capture_incomplete |= output.read_error;
            }
            Ok(_) | Err(_) => capture_incomplete = true,
        }
    }
    result.reason = if timed_out {
        "native process timed out".into()
    } else if capture_incomplete {
        "native process output capture incomplete".into()
    } else if result.output_truncated {
        "native process output exceeded bounded capture".into()
    } else if !status.is_some_and(|value| value.success()) {
        "native process exited unsuccessfully".into()
    } else if result.stdout.trim() != "vibe 1.1.0" {
        "native process version output differs from vibe 1.1.0".into()
    } else {
        result.status = PublicationSmokeProbeStatus::Passed;
        "exact staged native executable returned vibe 1.1.0".into()
    };
    result
}

#[cfg(test)]
#[path = "smoke/tests.rs"]
mod tests;
