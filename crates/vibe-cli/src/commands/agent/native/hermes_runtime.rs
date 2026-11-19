//! Resolve only the selected Hermes installation and its declared interpreter.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-EXECUTABLE");

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub(super) struct HermesRuntime {
    pub(super) python: PathBuf,
    pub(super) root: PathBuf,
}

// Admission is bound to measured native modules, not Hermes' placeholder 0.0.0 version.
const MODULES: &[(&str, &str)] = &[
    (
        "gateway/run_turn.py",
        "834c924b1b5b83816ba228800f4b2daff39d10e9ef046ba32f28f2133752284b",
    ),
    (
        "gateway/session_execution.py",
        "8605e87e8f31fca8f0a982a481fd673f722ebdedcb8f2f83d051850b79a9585b",
    ),
    (
        "agent/system_prompt.py",
        "d9b4289c45144ad193fd179cc8c2b9f8053b41edfdc657e1768c061c3593e891",
    ),
    (
        "gateway/run_api.py",
        "f3c0f65027021b043e57b2fb42ec7bf8f6a79d3685753f9abf02aad1acc8fda8",
    ),
    (
        "gateway/session_authority.py",
        "0c63dfc2b7bde9a8392373cc91a67518d8e91956be96f9b01c31f3bc514678c4",
    ),
    (
        "hermes_cli/main.py",
        "8d986cd8de3ea7802e04a9d58543b19e7b03436299be5d5e0b6b65e409e22705",
    ),
];

pub(super) fn discover(program: &Path) -> Result<HermesRuntime> {
    let resolved = std::fs::canonicalize(program)?;
    let header = std::fs::read(&resolved)?;
    let first = header
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default();
    let interpreter = std::str::from_utf8(first)
        .ok()
        .and_then(|line| line.strip_prefix("#!"))
        .map(str::trim)
        .filter(|value| !value.contains(char::is_whitespace))
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.is_file())
        .context("selected Hermes launcher must declare its installed Python interpreter; shell wrappers are not admitted")?;
    for ancestor in resolved.ancestors().take(5) {
        for root in [ancestor.to_path_buf(), ancestor.join("hermes-agent")] {
            if MODULES.iter().all(|(relative, expected)| {
                std::fs::read(root.join(relative))
                    .is_ok_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == *expected)
            }) {
                return Ok(HermesRuntime {
                    python: interpreter,
                    root,
                });
            }
        }
    }
    bail!(
        "selected Hermes installation does not match the admitted native module identities; use read delivery without a carrier"
    )
}
