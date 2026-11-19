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
    (
        "hermes_cli/gateway_chat.py",
        "f689d2471c2020e2b884375f45dc850332af934f958d2a633ec1275cf6ebdb02",
    ),
    (
        "hermes_cli/session_worktree.py",
        "575665b1fe68f2f06912fc39a4c72b461ca5e01ccc59058e9a46728489970428",
    ),
    (
        "hermes_cli/main_resume_args.py",
        "907c68545fe1b668e5bc606380c201f7ce45ec7bd938ee3c78e59caa19a9334c",
    ),
    (
        "gateway/session_finite.py",
        "a01c12ede67fad8d63a3e8054b7e76696e18250294b1676594d1031442639bad",
    ),
    (
        "hermes_cli/config.py",
        "f356ce0175ea466425798f9e206e617958ca85a826b00d0bb05b9fc33d5a761b",
    ),
    (
        "hermes_cli/main_tui_launch.py",
        "404b6da1ee01a0769f3e765b67e6fe887faf820246dfe6d99e1cbf6cea96044d",
    ),
    (
        "hermes_cli/_parser.py",
        "e9f0fbb367a32cbfc8f144b28c3f4ff6d0956003fbb1a0c09f3c487f4bf19311",
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
