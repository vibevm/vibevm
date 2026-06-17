//! Activation plumbing (PROP-019 §2.5, §2.6): shell detection, the stable
//! shims, and durable environment persistence.
//!
//! The durable writes sit behind the [`EnvPersister`] seam so tests drive
//! the POSIX rc-file path in a temp file and never mutate the real machine;
//! the Windows registry path is only ever taken by a live `man use`.

specmark::scope!("spec://vibevm/common/PROP-019#activation");

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
         \x20 echo 'vibe: no active version — run: vibe man use <selector>' >&2\n\
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
         echo vibe: no active version - run: vibe man use ^<selector^> 1>&2\r\n\
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

/// A Windows persister: edits the user environment in the registry via
/// PowerShell's `[Environment]` API, which broadcasts the change to new
/// processes (PROP-019 §2.6). Not exercised by gate tests (it mutates the
/// real user env); only a live `man use` takes this path.
pub(crate) struct WindowsEnvPersister;

impl EnvPersister for WindowsEnvPersister {
    fn set_vibevm_home(&self, home: &Path) -> Result<Persisted> {
        ps_set_user_var("VIBEVM_HOME", &home.display().to_string())?;
        Ok(Persisted::Changed)
    }

    fn ensure_on_path(&self, dir: &Path) -> Result<Persisted> {
        let current = ps_get_user_var("Path")?.unwrap_or_default();
        let target = dir.display().to_string();
        if current.split(';').any(|p| p.eq_ignore_ascii_case(&target)) {
            return Ok(Persisted::Unchanged);
        }
        let next = if current.is_empty() {
            target
        } else {
            format!("{};{}", current.trim_end_matches(';'), target)
        };
        ps_set_user_var("Path", &next)?;
        Ok(Persisted::Changed)
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

fn ps_set_user_var(name: &str, value: &str) -> Result<()> {
    run_powershell(&format!(
        "[Environment]::SetEnvironmentVariable('{}', '{}', 'User')",
        ps_quote(name),
        ps_quote(value)
    ))?;
    Ok(())
}

fn ps_get_user_var(name: &str) -> Result<Option<String>> {
    let out = run_powershell(&format!(
        "[Environment]::GetEnvironmentVariable('{}', 'User')",
        ps_quote(name)
    ))?;
    Ok(if out.is_empty() { None } else { Some(out) })
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
mod tests {
    use super::*;
    use specmark::verifies;

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#activation", r = 1)]
    fn shell_detect_and_export_line() {
        assert_eq!(Shell::detect(Some("/usr/bin/zsh")), Shell::Zsh);
        assert_eq!(Shell::detect(Some("/bin/bash")), Shell::Bash);
        assert_eq!(Shell::detect(Some("/usr/local/bin/fish")), Shell::Fish);
        let home = Path::new("/opt/vibevm/versions/branch/main");
        assert!(
            Shell::Bash
                .export_line(home)
                .starts_with("export VIBEVM_HOME=")
        );
        assert!(
            Shell::Fish
                .export_line(home)
                .starts_with("set -gx VIBEVM_HOME")
        );
        assert!(
            Shell::Pwsh
                .export_line(home)
                .starts_with("$env:VIBEVM_HOME")
        );
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#activation", r = 1)]
    fn shims_read_the_current_pointer() {
        let tmp = tempfile::tempdir().unwrap();
        write_shims(tmp.path()).unwrap();
        let posix = fs::read_to_string(tmp.path().join("vibe")).unwrap();
        assert!(posix.contains("vibevm/current"), "reads the live pointer");
        assert!(
            posix.contains("$VIBEVM_HOME"),
            "falls back to the advisory env"
        );
        assert!(posix.contains(BINARY_NAME));
        if cfg!(windows) {
            let cmd = fs::read_to_string(tmp.path().join("vibe.cmd")).unwrap();
            assert!(cmd.contains("current"));
            assert!(cmd.contains(BINARY_NAME));
        }
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#path", r = 1)]
    fn rc_persister_is_idempotent_and_repoints() {
        let tmp = tempfile::tempdir().unwrap();
        let rc = tmp.path().join(".bashrc");
        fs::write(&rc, "# user's own line\nexport EDITOR=vim\n").unwrap();
        let p = RcFilePersister::new(rc.clone(), Shell::Bash);

        let home_a = Path::new("/opt/vibevm/versions/tag/1.0.0");
        assert_eq!(p.set_vibevm_home(home_a).unwrap(), Persisted::Changed);
        assert_eq!(
            p.ensure_on_path(Path::new("/opt/bin")).unwrap(),
            Persisted::Changed
        );
        // Re-applying the same values is a no-op.
        assert_eq!(p.set_vibevm_home(home_a).unwrap(), Persisted::Unchanged);
        assert_eq!(
            p.ensure_on_path(Path::new("/opt/bin")).unwrap(),
            Persisted::Unchanged
        );

        // Switching versions repoints the same line, not a second one.
        let home_b = Path::new("/opt/vibevm/versions/branch/main");
        assert_eq!(p.set_vibevm_home(home_b).unwrap(), Persisted::Changed);
        let text = fs::read_to_string(&rc).unwrap();
        assert_eq!(text.matches("export VIBEVM_HOME=").count(), 1);
        assert_eq!(text.matches(BLOCK_BEGIN).count(), 1);
        assert!(text.contains("branch/main"));
        assert!(!text.contains("tag/1.0.0"));
        // The user's own lines survive.
        assert!(text.contains("export EDITOR=vim"));
    }
}
