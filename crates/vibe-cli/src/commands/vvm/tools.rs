//! The required build toolchain as a single source of truth (PROP-019 §2.8),
//! which doubles as the runnable form of "how to update the stack" (§7):
//! change [`REQUIRED_TOOLS`] and both `man doctor` and the docs follow.

specmark::scope!("spec://vibevm/common/PROP-019#tools");

use std::process::Command;

/// One tool a from-source build needs (PROP-019 §2.8).
pub(crate) struct ToolSpec {
    pub name: &'static str,
    /// The command that prints a version, e.g. `["git", "--version"]`.
    pub check: &'static [&'static str],
    pub min_version: &'static str,
    pub help_url: &'static str,
}

/// The tools a `vibe self install` build requires. The platform linker / C
/// toolchain (MSVC Build Tools / Xcode CLT / build-essential) is also
/// needed but is not version-checkable here — `self doctor` names it
/// separately. OpenSSL is deliberately absent (vibevm uses rustls).
pub(crate) const REQUIRED_TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "git",
        check: &["git", "--version"],
        min_version: "2.0.0",
        help_url: "https://git-scm.com/downloads",
    },
    ToolSpec {
        name: "cargo",
        check: &["cargo", "--version"],
        min_version: "1.93.0",
        help_url: "https://rustup.rs",
    },
    ToolSpec {
        name: "rustc",
        check: &["rustc", "--version"],
        min_version: "1.93.0",
        help_url: "https://rustup.rs",
    },
];

/// Tools `vibe self install` uses ONLY to package vibeterm (node, npm) —
/// **advisory, not required**: a Rust-only box skips vibeterm and still builds
/// (PROP-019 §2.7). Surfaced by `self doctor` but never counted as a problem.
pub(crate) const OPTIONAL_TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "node",
        check: &["node", "--version"],
        min_version: "22.6.0",
        help_url: "https://nodejs.org/",
    },
    ToolSpec {
        name: "npm",
        check: &["npm", "--version"],
        min_version: "11.0.0",
        help_url: "https://docs.npmjs.com/downloading-and-installing-node-js-and-npm",
    },
];

/// Probe the optional (vibeterm-packaging) tools.
pub(crate) fn check_optional() -> Vec<ToolStatus> {
    OPTIONAL_TOOLS.iter().map(check_one).collect()
}

/// The platform linker / C toolchain hint for `man doctor` (PROP-019 §2.8).
pub(crate) fn linker_hint() -> (&'static str, &'static str) {
    if cfg!(windows) {
        (
            "MSVC Build Tools (Desktop development with C++)",
            "https://visualstudio.microsoft.com/visual-cpp-build-tools/",
        )
    } else if cfg!(target_os = "macos") {
        (
            "Xcode Command Line Tools (`xcode-select --install`)",
            "https://developer.apple.com/xcode/resources/",
        )
    } else {
        (
            "a C toolchain (e.g. build-essential)",
            "https://gcc.gnu.org/install/",
        )
    }
}

/// The outcome of probing one tool.
pub(crate) struct ToolStatus {
    pub name: &'static str,
    pub version: Option<String>,
    pub ok: bool,
    pub min_version: &'static str,
    pub help_url: &'static str,
}

/// Probe every required tool.
pub(crate) fn check_all() -> Vec<ToolStatus> {
    REQUIRED_TOOLS.iter().map(check_one).collect()
}

fn check_one(spec: &ToolSpec) -> ToolStatus {
    let version = run_version(spec.check);
    let ok = version
        .as_deref()
        .map(|v| version_ok(v, spec.min_version))
        .unwrap_or(false);
    ToolStatus {
        name: spec.name,
        version,
        ok,
        min_version: spec.min_version,
        help_url: spec.help_url,
    }
}

fn run_version(cmd: &[&str]) -> Option<String> {
    let out = tool_command(cmd[0]).args(&cmd[1..]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    extract_semver(&String::from_utf8_lossy(&out.stdout))
}

/// Whether a tool is invocable by name (presence only; version floors are
/// enforced elsewhere — `engines` for npm, [`check_one`] for the required set).
/// `pub(crate)` so the vibeterm packager's availability gate reuses one probe.
pub(crate) fn has_tool(name: &str) -> bool {
    // Gate on the tool's own exit code, not merely that a process spawned: on
    // Windows the spawn is always `cmd.exe` (which exists unconditionally), so
    // `.status().is_ok()` returned `true` for EVERY name — a Rust-only box then
    // failed `self update` at `Command::new("node")` instead of gracefully
    // skipping vibeterm. A missing tool makes `cmd /C <name> --version` exit
    // non-zero; a real one exits 0.
    tool_command(name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Build a [`Command`] for a bare tool name, cross-platform (PROP-019 §2.8).
/// On Windows a PATH-resolved tool is often a `.cmd`/`.bat` shim (`npm`,
/// `npx`), but Rust's `Command::new` does **not** consult `PATHEXT`, so it
/// cannot find `npm.cmd`. Routing through `cmd /C` lets `cmd.exe` do the
/// `PATHEXT` resolution; elsewhere the bare name is correct.
fn tool_command(name: &str) -> Command {
    if cfg!(windows) {
        let mut c = Command::new("cmd.exe");
        c.arg("/C").arg(name);
        c
    } else {
        Command::new(name)
    }
}

/// Pull the first `X.Y.Z` out of a `--version` line, tolerating vendor
/// suffixes (e.g. git's `2.43.0.windows.1`).
fn extract_semver(text: &str) -> Option<String> {
    for tok in text.split(|c: char| !(c.is_ascii_digit() || c == '.')) {
        let parts: Vec<&str> = tok.split('.').filter(|p| !p.is_empty()).collect();
        if parts.len() >= 3 && parts[..3].iter().all(|p| p.parse::<u64>().is_ok()) {
            return Some(format!("{}.{}.{}", parts[0], parts[1], parts[2]));
        }
    }
    None
}

/// `found >= min`. If either fails to parse, do not fault the user.
fn version_ok(found: &str, min: &str) -> bool {
    match (semver::Version::parse(found), semver::Version::parse(min)) {
        (Ok(f), Ok(m)) => f >= m,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#tools", r = 1)]
    fn required_tools_table_is_well_formed() {
        assert!(!REQUIRED_TOOLS.is_empty());
        for t in REQUIRED_TOOLS {
            assert!(!t.check.is_empty(), "{} has a check command", t.name);
            assert!(
                semver::Version::parse(t.min_version).is_ok(),
                "{} min_version parses",
                t.name
            );
            assert!(
                t.help_url.starts_with("https://"),
                "{} has a help URL",
                t.name
            );
        }
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#tools", r = 1)]
    fn extract_semver_tolerates_vendor_suffixes() {
        assert_eq!(
            extract_semver("git version 2.43.0").as_deref(),
            Some("2.43.0")
        );
        assert_eq!(
            extract_semver("git version 2.43.0.windows.1").as_deref(),
            Some("2.43.0")
        );
        assert_eq!(
            extract_semver("cargo 1.93.0 (abc 2026-02-11)").as_deref(),
            Some("1.93.0")
        );
        assert_eq!(extract_semver("no version here").as_deref(), None);
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#tools", r = 1)]
    fn version_ok_compares_semver() {
        assert!(version_ok("1.93.1", "1.93.0"));
        assert!(version_ok("2.0.0", "1.93.0"));
        assert!(!version_ok("1.92.0", "1.93.0"));
        // Unparseable → not faulted.
        assert!(version_ok("weird", "1.0.0"));
    }
}
