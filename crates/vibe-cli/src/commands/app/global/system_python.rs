//! Fixed, isolated system-Python capability observation at the CLI boundary.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#system-python");

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use vibe_app::{CommandSearch, SystemPythonObservation};

const PROBE: &str = "import json,os,sys,sysconfig; print(json.dumps(dict(executable=os.path.realpath(sys.executable),implementation=sys.implementation.name,major=sys.version_info.major,minor=sys.version_info.minor,soabi=sysconfig.get_config_var('SOABI'),prefix=sys.prefix,base_prefix=sys.base_prefix)))";
const LIMIT: u64 = 8192;

pub(super) fn observe() -> Result<SystemPythonObservation, String> {
    if !cfg!(target_os = "linux") {
        return Err("system Python archive admission currently supports Linux only".into());
    }
    // A closed system search, not the invoking project's PATH, a venv, pyenv,
    // an app proxy or a package-authored executable. Reuse the command owner.
    let program = CommandSearch {
        path: Some("/usr/bin".into()),
        ..Default::default()
    }
    .locate("python3")
    .map_err(|_| "system Python missing at /usr/bin/python3; provide CPython 3.14 and retry")?;
    trusted(&program)?;
    probe(&program, Duration::from_secs(3))
}

fn trusted(program: &Path) -> Result<(), String> {
    if program.parent() != Some(Path::new("/usr/bin")) {
        return Err("system Python resolves outside the admitted system binary directory".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        for path in [
            Path::new("/"),
            Path::new("/usr"),
            Path::new("/usr/bin"),
            program,
        ] {
            let meta = std::fs::symlink_metadata(path)
                .map_err(|_| "cannot inspect system Python ownership")?;
            if meta.uid() != 0 || meta.mode() & 0o022 != 0 || meta.file_type().is_symlink() {
                return Err("system Python path is not a protected system-owned executable".into());
            }
        }
    }
    let mut magic = [0; 4];
    std::fs::File::open(program)
        .and_then(|mut file| file.read_exact(&mut magic))
        .map_err(|_| "cannot read system Python executable")?;
    if magic != *b"\x7fELF" {
        return Err("system Python must be a native executable, not a script or shim".into());
    }
    Ok(())
}

fn probe(program: &Path, timeout: Duration) -> Result<SystemPythonObservation, String> {
    let output = tempfile::tempfile().map_err(|_| "cannot allocate Python probe output")?;
    let errors = tempfile::tempfile().map_err(|_| "cannot allocate Python probe diagnostics")?;
    let mut child = Command::new(program)
        .args(["-I", "-S", "-c", PROBE])
        .env_clear()
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(
            output
                .try_clone()
                .map_err(|_| "cannot attach Python probe output")?,
        )
        .stderr(
            errors
                .try_clone()
                .map_err(|_| "cannot attach Python probe diagnostics")?,
        )
        .spawn()
        .map_err(|_| "cannot execute the admitted system Python")?;
    let started = Instant::now();
    let status = loop {
        let exceeded = output.metadata().map_or(true, |m| m.len() > LIMIT)
            || errors.metadata().map_or(true, |m| m.len() > LIMIT);
        if exceeded || started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err("system Python probe exceeded its time/output budget".into());
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("cannot observe system Python probe completion".into());
            }
        }
    };
    if !status.success() || errors.metadata().map_or(true, |m| m.len() != 0) {
        return Err("system Python isolated capability probe failed".into());
    }
    use std::io::{Seek, SeekFrom};
    let mut output = output;
    output
        .seek(SeekFrom::Start(0))
        .map_err(|_| "cannot read Python probe")?;
    let mut bytes = Vec::new();
    output
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "cannot read Python probe")?;
    if bytes.len() as u64 > LIMIT {
        return Err("system Python probe output is too large".into());
    }
    let observed: SystemPythonObservation = serde_json::from_slice(&bytes)
        .map_err(|_| "system Python probe returned an incomplete/invalid capability record")?;
    if observed.executable != program
        || observed.prefix != Path::new("/usr")
        || observed.base_prefix != observed.prefix
    {
        return Err("system Python probe does not identify the admitted system interpreter".into());
    }
    Ok(observed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_probe_refuses_non_system_paths_before_execution() {
        let root = tempfile::tempdir().unwrap();
        let shim = root.path().join("python3");
        std::fs::write(&shim, "#!/bin/sh\nexit 0\n").unwrap();
        assert!(trusted(&shim).is_err());
        assert!(trusted(Path::new("python3")).is_err());
    }
}
