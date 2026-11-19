//! Native YAML renders only on an owned stage; MCP PublicationBatch owns final writes.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION");

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::{Command, Stdio};
use vibe_safefs::Project;

const CONFIG_CAP: usize = 64 * 1024 * 1024;

/// Read an owned key or render its desired projection without touching live config.
pub(crate) fn project_yaml(
    settings: &Path,
    program: &Path,
    home: &Path,
    config: &Path,
    name: &str,
    desired: Option<Option<&serde_json::Value>>,
) -> Result<(Option<serde_json::Value>, String)> {
    let runtime = super::hermes_runtime::discover(program)?;
    let home_cap = Project::open(home).context("opening admitted Hermes profile home")?;
    if config != home.join("config.yaml") {
        bail!("Hermes MCP requires the selected profile config.yaml; refusing fallback");
    }
    let before = home_cap.read_file_snapshot_bounded("config.yaml", CONFIG_CAP)?;
    // An invocation-owned scratch home keeps read-only status/preview from
    // publishing helper assets or directories into the selected user profile.
    let temporary = tempfile::Builder::new()
        .prefix("vibe-hermes-mcp-")
        .tempdir()?;
    let stage_cap = Project::open(temporary.path())?;
    let directory = stage_cap.dir(&[], false)?;
    let rendered_dir = stage_cap.dir(&["render"], true)?;
    let bytes = before
        .as_ref()
        .map_or(&b"{}\n"[..], |snapshot| snapshot.bytes.as_slice());
    stage_cap.publish_new_in_with_mode(&rendered_dir, "config.yaml", bytes, private_mode())?;
    stage_cap.publish_new_in_with_mode(&directory, "result.json", b"", private_mode())?;
    stage_cap.publish_new_in_with_mode(
        &directory,
        "helper.py",
        include_bytes!("hermes_mcp.py"),
        private_mode(),
    )?;
    let helper = temporary.path().join("helper.py");
    let _ = settings; // Effective settings selected runtime; no persistent helper publication here.
    let request = serde_json::json!({
        "stage": rendered_dir.path().join("config.yaml"), "home": temporary.path(),
        "name": name, "operation": if desired.is_some() { "render" } else { "inspect" },
        "value": desired.flatten(), "result": temporary.path().join("result.json"),
    });
    stage_cap.publish_new_in_with_mode(
        &directory,
        "request.json",
        &serde_json::to_vec(&request)?,
        private_mode(),
    )?;
    let code = "import runpy,sys;root,helper,request=sys.argv[1:];sys.path.insert(0,root);sys.argv=[helper,request];runpy.run_path(helper,run_name='__main__')";
    let cancellation = vibe_registry::git_backend::transfers::invocation_control();
    cancellation.checkpoint()?;
    let child = Command::new(&runtime.python)
        .args(["-I", "-c", code])
        .arg(&runtime.root)
        .arg(helper)
        .arg(temporary.path().join("request.json"))
        .env("HERMES_HOME", temporary.path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("starting admitted Hermes YAML renderer")?;
    let mut child = OwnedChild(child);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let status = loop {
        cancellation.checkpoint()?;
        if let Some(status) = child.0.try_wait()? {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.0.kill();
            let _ = child.0.wait();
            bail!("Hermes YAML rendering timed out; live configuration was not published");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    if !status.success() {
        bail!(
            "Hermes native YAML rendering refused; preserve config and check selected profile/runtime"
        );
    }
    if home_cap.read_file_snapshot_bounded("config.yaml", CONFIG_CAP)? != before {
        bail!("Hermes configuration changed while rendering; newer bytes preserved");
    }
    let result = stage_cap
        .read_file_bounded("result.json", CONFIG_CAP)?
        .context("Hermes renderer result missing")?;
    let value: serde_json::Value =
        serde_json::from_slice(&result).context("invalid private Hermes owned-key result")?;
    let owned = value.get("entry").filter(|entry| !entry.is_null()).cloned();
    let rendered = stage_cap
        .read_file_bounded_in(&rendered_dir, "config.yaml", CONFIG_CAP)?
        .context("Hermes rendered stage missing")?;
    Ok((
        owned,
        String::from_utf8(rendered).context("Hermes YAML is not UTF-8")?,
    ))
}

/// Resolve the native process/profile home without reading model credentials.
pub(crate) fn profile_home(program: &Path) -> Result<std::path::PathBuf> {
    let runtime = super::hermes_runtime::discover(program)?;
    let temporary = tempfile::Builder::new()
        .prefix("vibe-hermes-home-")
        .tempdir()?;
    let capability = Project::open(temporary.path())?;
    let directory = capability.dir(&[], false)?;
    capability.publish_new_in_with_mode(
        &directory,
        "helper.py",
        include_bytes!("hermes_mcp.py"),
        private_mode(),
    )?;
    capability.publish_new_in_with_mode(&directory, "home.json", b"", private_mode())?;
    let code = "import runpy,sys;root,helper,result=sys.argv[1:];sys.path.insert(0,root);sys.argv=[helper,'--profile-home',result];runpy.run_path(helper,run_name='__main__')";
    let child = Command::new(&runtime.python)
        .args(["-I", "-c", code])
        .arg(&runtime.root)
        .arg(temporary.path().join("helper.py"))
        .arg(temporary.path().join("home.json"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut child = OwnedChild(child);
    let control = vibe_registry::git_backend::transfers::invocation_control();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        control.checkpoint()?;
        if let Some(status) = child.0.try_wait()? {
            if !status.success() {
                bail!("native Hermes profile lookup refused; select HERMES_HOME explicitly");
            }
            break;
        }
        if std::time::Instant::now() >= deadline {
            bail!("native Hermes profile lookup timed out");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let bytes = capability
        .read_file_bounded("home.json", 32768)?
        .context("native Hermes profile result missing")?;
    let result: serde_json::Value = serde_json::from_slice(&bytes)?;
    let home = std::path::PathBuf::from(
        result["home"]
            .as_str()
            .context("native Hermes profile home missing")?,
    );
    if !home.is_absolute()
        || home
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        bail!("native Hermes profile home is not absolute and normalized");
    }
    Ok(home)
}

struct OwnedChild(std::process::Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn cancelled_owned_helper_is_reaped_before_guard_returns() {
        let control = vibe_core::cancellation::CancellationToken::new();
        let child = Command::new("sh")
            .args(["-c", "exec sleep 60"])
            .spawn()
            .unwrap();
        let pid = child.id();
        let owned = OwnedChild(child);
        control.cancel();
        assert!(control.checkpoint().is_err());
        drop(owned);
        assert!(!Path::new(&format!("/proc/{pid}")).exists());
    }
}

fn private_mode() -> Option<u32> {
    if cfg!(unix) { Some(0o600) } else { None }
}
