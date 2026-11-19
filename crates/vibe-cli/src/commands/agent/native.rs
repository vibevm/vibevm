//! Immutable agent-native assets published by the Rust launcher.
//!
//! The selected agent supplies its own runtime. Ordinary Vibe remains a Rust
//! binary; helpers never run from project-authored paths or overwrite an asset
//! another invocation may still consume.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-EXECUTABLE");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use vibe_safefs::Project;

#[path = "native/hermes_mcp.rs"]
mod hermes_mcp;
#[path = "native/hermes_ready_session.rs"]
pub(super) mod hermes_ready_session;
#[path = "native/hermes_runtime.rs"]
mod hermes_runtime;
#[path = "native/hermes_selected.rs"]
pub(super) mod hermes_selected;
#[path = "native/pi_runtime.rs"]
mod pi_runtime;
pub(crate) use hermes_mcp::{profile_home as hermes_profile_home, project_yaml};

/// Pi's agent-native carrier; runtime/mode orchestration remains native.
const PI_HELPER: &[u8] = include_bytes!("native/pi_bootstrap.mjs");
const PI_ENTRY: &[u8] = include_bytes!("native/pi_entry.mjs");

/// Publish the supported Pi carrier from the binary, never a project file.
pub(super) fn publish_pi(settings: &Path) -> Result<PathBuf> {
    publish_asset(settings, "pi", "mjs", PI_HELPER)
}

/// The admitted runtime and immutable helpers for a native invocation.
pub(super) struct NativeLaunch {
    pub(super) command: Vec<String>,
}

pub(super) fn prepare_pi(
    settings: &Path,
    program: &Path,
    prefix: &Path,
    cwd: &Path,
    args: &[String],
) -> Result<NativeLaunch> {
    let runtime = pi_runtime::discover(program)?;
    let carrier = publish_pi(settings)?;
    let entry = publish_asset(settings, "pi", "mjs", PI_ENTRY)?;
    let mut command = vec![
        runtime.node.display().to_string(),
        entry.display().to_string(),
        runtime.package.display().to_string(),
        carrier.display().to_string(),
        prefix.display().to_string(),
        cwd.display().to_string(),
    ];
    command.extend_from_slice(args);
    Ok(NativeLaunch { command })
}

/// Prepare the exact installed Hermes interpreter and embedded native supervisor.
pub(super) fn prepare_hermes(
    settings: &Path,
    program: &Path,
    prefix: &Path,
    cwd: &Path,
    args: &[String],
) -> Result<NativeLaunch> {
    let runtime = hermes_runtime::discover(program)?;
    let carrier = publish_asset(
        settings,
        "hermes",
        "py",
        include_bytes!("native/hermes_bootstrap.py"),
    )?;
    let entry = publish_asset(
        settings,
        "hermes",
        "py",
        include_bytes!("native/hermes_entry.py"),
    )?;
    // -I excludes project/current-directory Python imports. Only this admitted
    // native installation is inserted before executing the immutable helper.
    let code = "import runpy,sys;root,entry,*args=sys.argv[1:];sys.path.insert(0,root);sys.argv=[entry,*args];runpy.run_path(entry,run_name='__main__')";
    let mut command = vec![
        runtime.python.display().to_string(),
        "-I".into(),
        "-c".into(),
        code.into(),
        runtime.root.display().to_string(),
        entry.display().to_string(),
        carrier.display().to_string(),
        prefix.display().to_string(),
        cwd.display().to_string(),
    ];
    command.extend_from_slice(args);
    Ok(NativeLaunch { command })
}

/// Publish one embedded helper under an already admitted settings root.
/// The whole digest is the immutable name; existing mismatched bytes refuse.
pub(super) fn publish_asset(
    settings: &Path,
    family: &str,
    extension: &str,
    bytes: &[u8],
) -> Result<PathBuf> {
    if !matches!(family, "hermes" | "pi") || !matches!(extension, "py" | "mjs") {
        bail!("unknown native helper family or extension");
    }
    let project = Project::open(settings).context("opening native helper settings root")?;
    let directory = project.dir(&["agent", "native", family], true)?;
    let digest = format!("{:x}", Sha256::digest(bytes));
    let name = format!("bootstrap-{digest}.{extension}");
    match project.read_file_bounded_in(&directory, &name, bytes.len())? {
        Some(existing) if existing == bytes => return Ok(directory.path().join(name)),
        Some(_) => bail!("native helper content-addressed path contains foreign bytes"),
        None => {}
    }
    let _lock = project.lock("native-helper-publication.lock")?;
    // A concurrent publisher may have won before our lock; verify, never replace.
    if let Some(existing) = project.read_file_bounded_in(&directory, &name, bytes.len())? {
        if existing != bytes {
            bail!("native helper publication collided with foreign bytes");
        }
    } else {
        project
            .publish_new_in_with_mode(&directory, &name, bytes, native_asset_mode())
            .map_err(anyhow::Error::new)?;
    }
    Ok(directory.path().join(name))
}

fn native_asset_mode() -> Option<u32> {
    #[cfg(unix)]
    {
        Some(0o600)
    }
    #[cfg(not(unix))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn hermes_proxy_ack_python_contract() {
        let script = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/commands/agent/native/test_hermes_proxy_ack.py");
        let result = std::process::Command::new("python3")
            .args(["-I", "-B"])
            .arg(script)
            .output()
            .expect("Python 3 is required for the Hermes ACK contract test");
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    #[test]
    fn assets_are_immutable_and_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let path = publish_asset(temp.path(), "pi", "mjs", b"native helper").unwrap();
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(
            publish_asset(temp.path(), "pi", "mjs", b"native helper").unwrap(),
            path
        );
        assert_eq!(
            std::fs::metadata(&path).unwrap().modified().unwrap(),
            modified
        );
        std::fs::write(&path, b"foreign").unwrap();
        assert!(publish_asset(temp.path(), "pi", "mjs", b"native helper").is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"foreign");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_helper_directory_is_refused() {
        let temp = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("agent")).unwrap();
        std::os::unix::fs::symlink(external.path(), temp.path().join("agent/native")).unwrap();
        assert!(publish_asset(temp.path(), "hermes", "py", b"helper").is_err());
        assert_eq!(std::fs::read_dir(external.path()).unwrap().count(), 0);
    }
}
