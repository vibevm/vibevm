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

use super::store::BINARY_NAME;

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

    /// The `eval`-able line that sets `VIBEVM_HOME` for this shell.
    pub(crate) fn export_line(self, home: &Path) -> String {
        let h = home.display();
        match self {
            Shell::Fish => format!("set -gx VIBEVM_HOME \"{h}\""),
            Shell::Pwsh => format!("$env:VIBEVM_HOME = \"{h}\""),
            _ => format!("export VIBEVM_HOME=\"{h}\""),
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
// shims
// ---------------------------------------------------------------------------

fn posix_shim() -> String {
    // Read the live `current` pointer (instant switch, no reload); fall back
    // to the advisory $VIBEVM_HOME (PROP-019 §2.5).
    format!(
        "#!/bin/sh\n\
         # vibevm (VVM) shim — execs the active instance from ../vibevm/current.\n\
         self=\"$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\"\n\
         home=\"$(cat \"$self/../vibevm/current\" 2>/dev/null)\"\n\
         [ -z \"$home\" ] && home=\"$VIBEVM_HOME\"\n\
         if [ -z \"$home\" ]; then\n\
         \x20 echo 'vibe: no active version — run: vibe self use <selector>' >&2\n\
         \x20 exit 1\n\
         fi\n\
         exec \"$home/{BINARY_NAME}\" \"$@\"\n"
    )
}

fn cmd_shim() -> String {
    format!(
        "@echo off\r\n\
         set \"VVM_CUR=%~dp0..\\vibevm\\current\"\r\n\
         set \"VVM_HOME=\"\r\n\
         if exist \"%VVM_CUR%\" set /p VVM_HOME=<\"%VVM_CUR%\"\r\n\
         if \"%VVM_HOME%\"==\"\" set \"VVM_HOME=%VIBEVM_HOME%\"\r\n\
         if \"%VVM_HOME%\"==\"\" (\r\n\
         echo vibe: no active version - run: vibe self use ^<selector^> 1>&2\r\n\
         exit /b 1\r\n\
         )\r\n\
         \"%VVM_HOME%\\{BINARY_NAME}\" %*\r\n"
    )
}

/// Write the stable shims into `bin_dir` (PROP-019 §2.5). They read
/// `$VIBEVM_HOME` and exec the active binary. Idempotent.
pub(crate) fn write_shims(bin_dir: &Path) -> Result<()> {
    fs::create_dir_all(bin_dir).with_context(|| format!("creating `{}`", bin_dir.display()))?;
    let posix = bin_dir.join("vibe");
    fs::write(&posix, posix_shim()).with_context(|| format!("writing `{}`", posix.display()))?;
    #[cfg(unix)]
    set_executable(&posix)?;
    if cfg!(windows) {
        let cmd = bin_dir.join("vibe.cmd");
        fs::write(&cmd, cmd_shim()).with_context(|| format!("writing `{}`", cmd.display()))?;
    }
    Ok(())
}

#[cfg(unix)]
fn set_executable(p: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = fs::metadata(p)?.permissions();
    perm.set_mode(0o755);
    fs::set_permissions(p, perm).with_context(|| format!("chmod +x `{}`", p.display()))?;
    Ok(())
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

const BLOCK_BEGIN: &str = "# >>> vibevm (VVM) — managed, do not edit by hand >>>";
const BLOCK_END: &str = "# <<< vibevm (VVM) <<<";

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
        let text = fs::read_to_string(&self.rc_path).unwrap_or_default();
        let (pre, mut block, post) = split_block(&text);
        if !set_or_add(&mut block, prefix, line) {
            return Ok(Persisted::Unchanged);
        }
        if let Some(parent) = self.rc_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating `{}`", parent.display()))?;
        }
        fs::write(&self.rc_path, rebuild(&pre, &block, &post))
            .with_context(|| format!("writing `{}`", self.rc_path.display()))?;
        Ok(Persisted::Changed)
    }
}

impl EnvPersister for RcFilePersister {
    fn set_vibevm_home(&self, home: &Path) -> Result<Persisted> {
        let (prefix, line) = match self.shell {
            Shell::Fish => (
                "set -gx VIBEVM_HOME",
                format!("set -gx VIBEVM_HOME \"{}\"", home.display()),
            ),
            _ => (
                "export VIBEVM_HOME=",
                format!("export VIBEVM_HOME=\"{}\"", home.display()),
            ),
        };
        self.upsert(prefix, &line)
    }

    fn ensure_on_path(&self, dir: &Path) -> Result<Persisted> {
        let d = dir.display();
        let (prefix, line) = match self.shell {
            Shell::Fish => ("fish_add_path", format!("fish_add_path \"{d}\"")),
            _ => ("export PATH=", format!("export PATH=\"{d}:$PATH\"")),
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

// --- rc block helpers ------------------------------------------------------

/// Split a file into (text before the managed block, the block's inner
/// lines, text after the block). No block → (whole text, [], "").
fn split_block(text: &str) -> (String, Vec<String>, String) {
    if let (Some(b), Some(e)) = (text.find(BLOCK_BEGIN), text.find(BLOCK_END))
        && b < e
    {
        let pre = text[..b].to_string();
        let inner = &text[b + BLOCK_BEGIN.len()..e];
        let block = inner
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect();
        let post = text[e + BLOCK_END.len()..].to_string();
        return (pre, block, post);
    }
    (text.to_string(), Vec::new(), String::new())
}

/// Replace the block line beginning with `prefix`, or append `line`. Returns
/// whether anything changed.
fn set_or_add(block: &mut Vec<String>, prefix: &str, line: &str) -> bool {
    if let Some(slot) = block.iter_mut().find(|l| l.starts_with(prefix)) {
        if slot == line {
            return false;
        }
        *slot = line.to_string();
        true
    } else {
        block.push(line.to_string());
        true
    }
}

fn rebuild(pre: &str, block: &[String], post: &str) -> String {
    if block.is_empty() {
        return format!("{pre}{post}");
    }
    let pre = pre.trim_end_matches('\n');
    let post = post.trim_start_matches('\n');
    let mut out = String::new();
    if !pre.is_empty() {
        out.push_str(pre);
        out.push('\n');
    }
    out.push_str(BLOCK_BEGIN);
    out.push('\n');
    out.push_str(&block.join("\n"));
    out.push('\n');
    out.push_str(BLOCK_END);
    out.push('\n');
    if !post.is_empty() {
        out.push_str(post);
        if !post.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
#[path = "env_tests.rs"]
mod tests;
