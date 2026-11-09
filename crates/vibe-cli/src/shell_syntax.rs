//! The one place in the tree that writes shell syntax
//! (PROP-075 `#ACTIVATION-ENV-OUTPUT`).
//!
//! The enum and its three quoting helpers grew for `vibe self env`
//! (PROP-019 §2.6) and are promoted here unchanged, so `vibe self env` prints
//! exactly the bytes it always printed — a golden test in
//! `commands/vvm/env_tests.rs` pins them. What is new is the `Cmd` variant
//! and the generic [`Shell::set_line`] / [`Shell::unset_line`] pair that
//! `vibe app env` and `vibe hook` print, because cmd has no `eval` and a
//! quoted `set` line cannot be fed through a pipe.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-ENV-OUTPUT");

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

pub(crate) fn posix_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub(crate) fn fish_quote(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

pub(crate) fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// A shell, for rc-file selection and `eval`-line syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Shell {
    Bash,
    Zsh,
    Fish,
    Pwsh,
    /// Windows `cmd`: bare `NAME=VALUE` lines consumed by `for /f`, with no
    /// quoting and no `set` keyword (PROP-075 `#ACTIVATION-ENV-OUTPUT`).
    Cmd,
    /// A generic POSIX shell (`.profile`, `export` syntax).
    Posix,
}

impl Shell {
    /// Detect from a `$SHELL` value, falling back per-OS.
    pub(crate) fn detect(shell_env: Option<&str>) -> Shell {
        if let Some(s) = shell_env {
            let s = s.to_ascii_lowercase();
            if s.contains("fish") {
                return Shell::Fish;
            }
            if s.contains("zsh") {
                return Shell::Zsh;
            }
            if s.contains("bash") {
                return Shell::Bash;
            }
        }
        if cfg!(windows) {
            Shell::Pwsh
        } else {
            Shell::Posix
        }
    }

    /// Parse an explicit `--shell` value.
    pub(crate) fn parse(name: &str) -> Result<Shell> {
        match name.to_ascii_lowercase().as_str() {
            "bash" => Ok(Shell::Bash),
            "zsh" => Ok(Shell::Zsh),
            "fish" => Ok(Shell::Fish),
            "powershell" | "pwsh" => Ok(Shell::Pwsh),
            "cmd" => Ok(Shell::Cmd),
            "posix" | "sh" => Ok(Shell::Posix),
            other => {
                bail!("unknown shell `{other}` (want bash|zsh|fish|powershell|cmd|posix)")
            }
        }
    }

    /// The stable spelling, as `--shell` takes it and `--json` reports it.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
            Shell::Pwsh => "pwsh",
            Shell::Cmd => "cmd",
            Shell::Posix => "posix",
        }
    }

    /// The `eval`-able line that selects an exact vibevm instance for this
    /// shell (`vibe self env`, PROP-019 §2.6). The override wins in stable
    /// shims; HOME remains advisory compatibility.
    pub(crate) fn export_line(self, home: &Path) -> String {
        let home = home.display().to_string();
        match self {
            Shell::Fish => format!(
                "set -gx VIBEVM_SHELL_HOME {}; set -gx VIBEVM_HOME {}",
                fish_quote(&home),
                fish_quote(&home)
            ),
            Shell::Pwsh => format!(
                "$env:VIBEVM_SHELL_HOME = {}; $env:VIBEVM_HOME = {}",
                powershell_quote(&home),
                powershell_quote(&home)
            ),
            Shell::Cmd => format!("VIBEVM_SHELL_HOME={home}\nVIBEVM_HOME={home}"),
            _ => format!(
                "export VIBEVM_SHELL_HOME={}; export VIBEVM_HOME={}",
                posix_quote(&home),
                posix_quote(&home)
            ),
        }
    }

    /// The line that sets one variable in this shell.
    pub(crate) fn set_line(self, name: &str, value: &str) -> String {
        match self {
            Shell::Fish => format!("set -gx {name} {};", fish_quote(value)),
            Shell::Pwsh => format!("$env:{name} = {};", powershell_quote(value)),
            Shell::Cmd => format!("{name}={value}"),
            _ => format!("export {name}={};", posix_quote(value)),
        }
    }

    /// The line that clears one variable in this shell. cmd's `for /f` form
    /// consumes `NAME=` as an empty value, which is how `set "NAME="` clears
    /// it.
    pub(crate) fn unset_line(self, name: &str) -> String {
        match self {
            Shell::Fish => format!("set -e {name};"),
            Shell::Pwsh => {
                format!("Remove-Item -ErrorAction SilentlyContinue Env:\\{name};")
            }
            Shell::Cmd => format!("{name}="),
            _ => format!("unset {name};"),
        }
    }

    /// The managed rc target under `home`; this does not prove it is sourced.
    pub(crate) fn rc_path(self, home: &Path) -> PathBuf {
        match self {
            Shell::Fish => home.join(".config").join("fish").join("config.fish"),
            Shell::Zsh => home.join(".zshrc"),
            Shell::Bash => home.join(".bashrc"),
            // PowerShell has no $HOME rc we edit here (Windows uses the
            // registry persister); fall back to .profile for POSIX.
            _ => home.join(".profile"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[specmark::spec(
        verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-ENV-OUTPUT"
    )]
    fn one_golden_set_and_unset_line_per_shell() {
        // A value with a space and a single quote, so each shell's quoting
        // rule is exercised rather than asserted on a trivial string.
        let value = "/opt/it's here/bin:/usr/bin";
        for (shell, set, unset) in [
            (
                Shell::Bash,
                "export PATH='/opt/it'\"'\"'s here/bin:/usr/bin';",
                "unset PATH;",
            ),
            (
                Shell::Zsh,
                "export PATH='/opt/it'\"'\"'s here/bin:/usr/bin';",
                "unset PATH;",
            ),
            (
                Shell::Posix,
                "export PATH='/opt/it'\"'\"'s here/bin:/usr/bin';",
                "unset PATH;",
            ),
            (
                Shell::Fish,
                "set -gx PATH '/opt/it\\'s here/bin:/usr/bin';",
                "set -e PATH;",
            ),
            (
                Shell::Pwsh,
                "$env:PATH = '/opt/it''s here/bin:/usr/bin';",
                "Remove-Item -ErrorAction SilentlyContinue Env:\\PATH;",
            ),
            // cmd gets the `for /f`-usable form: no quoting, no keyword.
            (Shell::Cmd, "PATH=/opt/it's here/bin:/usr/bin", "PATH="),
        ] {
            assert_eq!(shell.set_line("PATH", value), set, "{shell:?}");
            assert_eq!(shell.unset_line("PATH"), unset, "{shell:?}");
        }
    }

    #[test]
    fn the_shell_spellings_round_trip() {
        for shell in [
            Shell::Bash,
            Shell::Zsh,
            Shell::Fish,
            Shell::Pwsh,
            Shell::Cmd,
            Shell::Posix,
        ] {
            assert_eq!(Shell::parse(shell.as_str()).unwrap(), shell);
        }
        assert_eq!(Shell::parse("PowerShell").unwrap(), Shell::Pwsh);
        assert_eq!(Shell::parse("sh").unwrap(), Shell::Posix);
        let error = Shell::parse("nushell").unwrap_err().to_string();
        assert!(error.contains("cmd"), "{error}");
    }
}
