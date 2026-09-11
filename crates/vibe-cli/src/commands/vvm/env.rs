//! Activation plumbing (PROP-019 §2.5, §2.6): shell detection, the stable
//! shims, and durable environment persistence.
//!
//! The durable writes sit behind the [`EnvPersister`] seam so tests drive
//! the POSIX rc-file path in a temp file and never mutate the real machine;
//! the Windows registry path is only ever taken by a live `self use`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#activation");

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::store::VersionStore;

#[path = "env_rc.rs"]
mod rc_io;
#[cfg(test)]
use rc_io::BLOCK_BEGIN;
#[cfg(test)]
use rc_io::write_rc_atomic_at;
use rc_io::{read_rc_file, rebuild, set_or_add, split_block, write_rc_atomic};
#[path = "env_shim.rs"]
mod shim;
pub(crate) use shim::{shim_statuses, write_shims};
#[cfg(test)]
use shim::{write_shim_atomic, write_shim_atomic_at, write_shim_atomic_using};

fn posix_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn fish_quote(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

// ---------------------------------------------------------------------------
// shells
// ---------------------------------------------------------------------------

/// A login shell, for rc-file selection and `eval`-line syntax (PROP-019 §2.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shell {
    Bash,
    Zsh,
    Fish,
    Pwsh,
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
            "posix" | "sh" => Ok(Shell::Posix),
            other => bail!("unknown shell `{other}` (want bash|zsh|fish|powershell|posix)"),
        }
    }

    /// The `eval`-able line that selects an exact instance for this shell.
    /// The override wins in stable shims; HOME remains advisory compatibility.
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
            _ => format!(
                "export VIBEVM_SHELL_HOME={}; export VIBEVM_HOME={}",
                posix_quote(&home),
                posix_quote(&home)
            ),
        }
    }

    /// The rc file this shell sources at startup, under `home`.
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

// ---------------------------------------------------------------------------
// durable env persistence
// ---------------------------------------------------------------------------

/// Whether a durable edit changed anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Persisted {
    Changed,
    Unchanged,
}

/// Persists durable user-environment settings (PROP-019 §2.6). A
/// crate-internal seam (vibe-cli is a bin crate): tests exercise the POSIX
/// rc-file impl in a temp dir, never mutating the real machine.
pub(crate) trait EnvPersister {
    fn set_vibevm_home(&self, home: &Path) -> Result<Persisted>;
    fn ensure_on_path(&self, dir: &Path) -> Result<Persisted>;
    /// How to apply the change to the *current* shell now.
    fn activation_hint(&self) -> String;
}

pub(crate) struct ActivationOutcome {
    pub path: Persisted,
    pub advisory_home_warning: Option<String>,
}

/// The one activation transaction used by use/rollback/bootstrap/update:
/// stable shims and generic PATH first, journaled live pointer second, and
/// version-specific advisory HOME last.
pub(crate) fn activate_instance(
    store: &VersionStore,
    home: &Path,
    persister: &dyn EnvPersister,
) -> Result<ActivationOutcome> {
    write_shims(store)?;
    let path = persister.ensure_on_path(&store.shim_dir())?;
    store.write_current(home)?;
    let advisory_home_warning = persister
        .set_vibevm_home(home)
        .err()
        .map(|error| error.to_string());
    Ok(ActivationOutcome {
        path,
        advisory_home_warning,
    })
}

/// A POSIX rc-file persister: maintains one marked block in the rc file
/// (PROP-019 §2.6). Idempotent and testable in a temp dir.
pub(crate) struct RcFilePersister {
    rc_path: PathBuf,
    shell: Shell,
}

impl RcFilePersister {
    pub(crate) fn new(rc_path: PathBuf, shell: Shell) -> Self {
        RcFilePersister { rc_path, shell }
    }

    /// Upsert a managed line identified by `prefix`; returns whether the file
    /// changed.
    fn upsert(&self, prefix: &str, line: &str) -> Result<Persisted> {
        let (text, permissions) = read_rc_file(&self.rc_path)?;
        let (pre, mut block, post) = split_block(&text);
        if !set_or_add(&mut block, prefix, line) {
            return Ok(Persisted::Unchanged);
        }
        if let Some(parent) = self.rc_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating `{}`", parent.display()))?;
        }
        write_rc_atomic(
            &self.rc_path,
            rebuild(&pre, &block, &post).as_bytes(),
            permissions,
        )?;
        Ok(Persisted::Changed)
    }
}

impl EnvPersister for RcFilePersister {
    fn set_vibevm_home(&self, home: &Path) -> Result<Persisted> {
        let home = home.display().to_string();
        let (prefix, line) = match self.shell {
            Shell::Fish => (
                "set -gx VIBEVM_HOME",
                format!("set -gx VIBEVM_HOME {}", fish_quote(&home)),
            ),
            _ => (
                "export VIBEVM_HOME=",
                format!("export VIBEVM_HOME={}", posix_quote(&home)),
            ),
        };
        self.upsert(prefix, &line)
    }

    fn ensure_on_path(&self, dir: &Path) -> Result<Persisted> {
        let dir = dir.display().to_string();
        let (prefix, line) = match self.shell {
            Shell::Fish => (
                "fish_add_path",
                format!("fish_add_path {}", fish_quote(&dir)),
            ),
            _ => (
                "export PATH=",
                format!("export PATH={}:$PATH", posix_quote(&dir)),
            ),
        };
        self.upsert(prefix, &line)
    }

    fn activation_hint(&self) -> String {
        format!(
            "source `{}` (or open a new shell) to apply now",
            self.rc_path.display()
        )
    }
}

/// A Windows persister: edits the raw values in `HKCU\Environment`, preserving
/// `REG_SZ` versus `REG_EXPAND_SZ`, then broadcasts `WM_SETTINGCHANGE`.
/// Registry I/O is never exercised by gate tests because it mutates real user
/// state; the pure PATH transformation below carries the test coverage.
pub(crate) struct WindowsEnvPersister;

impl EnvPersister for WindowsEnvPersister {
    fn set_vibevm_home(&self, home: &Path) -> Result<Persisted> {
        let target = home.display().to_string();
        let current = ps_get_user_var_raw("VIBEVM_HOME")?;
        if current.as_ref().is_some_and(|value| value.value == target) {
            return Ok(Persisted::Unchanged);
        }
        let kind = current
            .map(|value| value.kind)
            .unwrap_or(RegistryValueKind::String);
        ps_set_user_var_raw("VIBEVM_HOME", &target, kind)?;
        Ok(Persisted::Changed)
    }

    fn ensure_on_path(&self, dir: &Path) -> Result<Persisted> {
        let current = ps_get_user_var_raw("Path")?;
        let raw = current
            .as_ref()
            .map(|value| value.value.as_str())
            .unwrap_or_default();
        match path_with_prefix(raw, &dir.display().to_string()) {
            None => Ok(Persisted::Unchanged),
            Some(next) => {
                let kind = current
                    .map(|value| value.kind)
                    .unwrap_or(RegistryValueKind::ExpandString);
                ps_set_user_var_raw("Path", &next, kind)?;
                Ok(Persisted::Changed)
            }
        }
    }

    fn activation_hint(&self) -> String {
        "open a new terminal (the registry change reaches new processes)".to_string()
    }
}

fn run_powershell(script: &str) -> Result<String> {
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .context("spawning powershell to edit the user environment")?;
    if !out.status.success() {
        bail!(
            "powershell env edit failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Single-quote-escape a value for a PowerShell single-quoted string.
fn ps_quote(value: &str) -> String {
    value.replace('\'', "''")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegistryValueKind {
    String,
    ExpandString,
}

impl RegistryValueKind {
    fn powershell_name(self) -> &'static str {
        match self {
            RegistryValueKind::String => "String",
            RegistryValueKind::ExpandString => "ExpandString",
        }
    }

    fn parse(raw: &str) -> Result<Self> {
        match raw {
            "String" => Ok(RegistryValueKind::String),
            "ExpandString" => Ok(RegistryValueKind::ExpandString),
            other => bail!("unsupported HKCU\\Environment registry value kind `{other}`"),
        }
    }
}

#[derive(Debug)]
struct RawUserVar {
    value: String,
    kind: RegistryValueKind,
}

#[derive(serde::Deserialize)]
struct RawUserVarJson {
    present: bool,
    value: Option<String>,
    kind: Option<String>,
}

fn ps_set_user_var_raw(name: &str, value: &str, kind: RegistryValueKind) -> Result<()> {
    let script = format!(
        r#"
$key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment', $true)
try {{
    $kind = [Microsoft.Win32.RegistryValueKind]::{kind}
    $key.SetValue('{name}', '{value}', $kind)
}} finally {{
    if ($null -ne $key) {{ $key.Dispose() }}
}}
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class VvmEnvironmentBroadcast {{
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr SendMessageTimeout(
        IntPtr hWnd, uint msg, UIntPtr wParam, string lParam,
        uint flags, uint timeout, out UIntPtr result);
}}
'@
[UIntPtr]$result = [UIntPtr]::Zero
[void][VvmEnvironmentBroadcast]::SendMessageTimeout(
    [IntPtr]0xffff, 0x001A, [UIntPtr]::Zero, 'Environment', 0x0002, 5000, [ref]$result)
"#,
        name = ps_quote(name),
        value = ps_quote(value),
        kind = kind.powershell_name(),
    );
    run_powershell(&script)?;
    Ok(())
}

fn ps_get_user_var_raw(name: &str) -> Result<Option<RawUserVar>> {
    let script = format!(
        r#"
$key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $false)
try {{
    if ($null -eq $key -or -not ($key.GetValueNames() -contains '{name}')) {{
        [pscustomobject]@{{ present = $false; value = $null; kind = $null }} |
            ConvertTo-Json -Compress
    }} else {{
        $value = $key.GetValue(
            '{name}', $null,
            [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        $kind = $key.GetValueKind('{name}').ToString()
        [pscustomobject]@{{ present = $true; value = [string]$value; kind = $kind }} |
            ConvertTo-Json -Compress
    }}
}} finally {{
    if ($null -ne $key) {{ $key.Dispose() }}
}}
"#,
        name = ps_quote(name),
    );
    let out = run_powershell(&script)?;
    let decoded: RawUserVarJson =
        serde_json::from_str(&out).context("decoding raw HKCU\\Environment value")?;
    if !decoded.present {
        return Ok(None);
    }
    let kind = RegistryValueKind::parse(
        decoded
            .kind
            .as_deref()
            .context("raw registry value omitted its kind")?,
    )?;
    Ok(Some(RawUserVar {
        value: decoded.value.unwrap_or_default(),
        kind,
    }))
}

/// Put `target` at the FRONT of a `;`-separated PATH, deduping any existing
/// normalized equivalent while preserving every unrelated entry byte-for-byte.
/// `None` means the raw value already has the canonical result. A version
/// manager's shim dir must take precedence over any other `vibe` on PATH, so
/// it is prepended, not appended.
fn path_with_prefix(current: &str, target: &str) -> Option<String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    path_with_prefix_core(current, target, &cwd, |name| std::env::var(name).ok())
}

fn path_with_prefix_core(
    current: &str,
    target: &str,
    cwd: &Path,
    lookup: impl Fn(&str) -> Option<String>,
) -> Option<String> {
    let target_normal = normalize_windows_path(target, cwd, &lookup);
    let mut kept = Vec::new();
    if !current.is_empty() {
        for part in current.split(';') {
            if !paths_equal(&normalize_windows_path(part, cwd, &lookup), &target_normal) {
                kept.push(part);
            }
        }
    }
    let next = if kept.is_empty() {
        target.to_string()
    } else {
        format!("{target};{}", kept.join(";"))
    };
    (next != current).then_some(next)
}

fn normalize_windows_path(
    raw: &str,
    cwd: &Path,
    lookup: &impl Fn(&str) -> Option<String>,
) -> String {
    let trimmed = raw.trim();
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(trimmed);
    let expanded = expand_percent_vars(unquoted, lookup);
    let path = PathBuf::from(expanded);
    let full = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };
    let mut clean = PathBuf::new();
    for component in full.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                clean.pop();
            }
            other => clean.push(other.as_os_str()),
        }
    }
    clean
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string()
}

fn expand_percent_vars(raw: &str, lookup: &impl Fn(&str) -> Option<String>) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut cursor = 0;
    while let Some(open_rel) = raw[cursor..].find('%') {
        let open = cursor + open_rel;
        result.push_str(&raw[cursor..open]);
        let after_open = open + 1;
        let Some(close_rel) = raw[after_open..].find('%') else {
            result.push_str(&raw[open..]);
            return result;
        };
        let close = after_open + close_rel;
        let name = &raw[after_open..close];
        if !name.is_empty()
            && let Some(value) = lookup(name)
        {
            result.push_str(&value);
        } else {
            result.push_str(&raw[open..=close]);
        }
        cursor = close + 1;
    }
    result.push_str(&raw[cursor..]);
    result
}

fn paths_equal(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

#[cfg(test)]
#[path = "env_tests.rs"]
mod tests;
