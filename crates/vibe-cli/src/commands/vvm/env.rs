//! Activation plumbing (PROP-019 §2.5, §2.6): shell detection, the stable
//! shims, and durable environment persistence.
//!
//! The durable writes sit behind the [`EnvPersister`] seam so tests drive
//! the POSIX rc-file path in a temp file and never mutate the real machine;
//! the Windows registry path is only ever taken by a live `self use`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#activation");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::store::VersionStore;

#[path = "env_profile.rs"]
mod profile;
pub(crate) use profile::EnvironmentObservation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
pub(crate) enum DurableEnvironmentPolicy {
    Preserve,
    Update,
}

#[path = "env_rc.rs"]
mod rc_io;
#[cfg(test)]
use rc_io::BLOCK_BEGIN;
#[cfg(test)]
use rc_io::write_rc_atomic_at;
use rc_io::{read_rc_file, rebuild, set_or_add, split_block, write_rc_atomic};
#[path = "env_registry.rs"]
mod registry;
pub(crate) use registry::{
    RawUserVar, RegistryValueKind, WindowsEnvPersister, delete_user_var_raw, get_user_var_raw,
    set_user_var_raw,
};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
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
    /// Observe the same durable target without changing it or executing a shell.
    fn inspect_path(&self, _dir: &Path) -> EnvironmentObservation {
        EnvironmentObservation::unavailable("this persister does not expose inspection".into())
    }
}

pub(crate) struct ActivationOutcome {
    pub path: Persisted,
    pub advisory_home_warning: Option<String>,
}

/// Activate only managed shims and the live pointer; no durable env target is opened.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
pub(crate) fn activate_without_environment(
    store: &VersionStore,
    home: &Path,
) -> Result<ActivationOutcome> {
    activate(store, home, None)
}

/// The one activation transaction used by use/rollback/bootstrap/update:
/// stable shims and generic PATH first, journaled live pointer second, and
/// version-specific advisory HOME last.
pub(crate) fn activate_instance(
    store: &VersionStore,
    home: &Path,
    persister: &dyn EnvPersister,
) -> Result<ActivationOutcome> {
    activate(store, home, Some(persister))
}

fn activate(
    store: &VersionStore,
    home: &Path,
    persister: Option<&dyn EnvPersister>,
) -> Result<ActivationOutcome> {
    write_shims(store)?;
    let path = match persister {
        Some(persister) => persister.ensure_on_path(&store.shim_dir())?,
        None => Persisted::Unchanged,
    };
    store.write_current(home)?;
    let advisory_home_warning = persister
        .and_then(|persister| persister.set_vibevm_home(home).err())
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
        let (pre, mut block, post) = split_block(&text)?;
        if block.iter().filter(|line| line.starts_with(prefix)).count() > 1 {
            bail!(
                "violates REQ spec://org.vibevm.core/vibevm/common/PROP-019#path: duplicate managed environment assignments; fix surface: inspect the existing profile block before requesting a repair"
            );
        }
        if !set_or_add(&mut block, prefix, line) {
            return Ok(Persisted::Unchanged);
        }
        if let Some(parent) = self.rc_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating `{}`", parent.display()))?;
        }
        write_rc_atomic(
            &self.rc_path,
            rebuild(
                &pre,
                &block,
                &post,
                if text.contains("\r\n") { "\r\n" } else { "\n" },
            )
            .as_bytes(),
            permissions,
        )?;
        Ok(Persisted::Changed)
    }
}

impl EnvPersister for RcFilePersister {
    fn inspect_path(&self, dir: &Path) -> EnvironmentObservation {
        profile::inspect_rc(&self.rc_path, self.shell, dir)
    }
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
        let (prefix, line) = managed_path_line(self.shell, dir);
        self.upsert(prefix, &line)
    }

    fn activation_hint(&self) -> String {
        format!(
            "source `{}` (or open a new shell) to apply now",
            self.rc_path.display()
        )
    }
}

fn managed_path_line(shell: Shell, dir: &Path) -> (&'static str, String) {
    let dir = dir.display().to_string();
    match shell {
        Shell::Fish => (
            "fish_add_path",
            format!("fish_add_path {}", fish_quote(&dir)),
        ),
        _ => (
            "export PATH=",
            format!("export PATH={}:$PATH", posix_quote(&dir)),
        ),
    }
}

/// Put `target` at the FRONT of a `;`-separated PATH, deduping any existing
/// normalized equivalent while preserving every unrelated entry byte-for-byte.
/// `None` means the raw value already has the canonical result. A version
/// manager's shim dir must take precedence over any other `vibe` on PATH, so
/// it is prepended, not appended.
pub(crate) fn path_with_prefix(current: &str, target: &str) -> Option<String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    path_with_prefix_core(current, target, &cwd, |name| std::env::var(name).ok())
}

/// Remove every normalized equivalent of `target` from a `;`-separated
/// PATH, keeping every other entry byte-for-byte; `None` when nothing
/// matches. The exact inverse of [`path_with_prefix`] for application env
/// steps (PROP-065 `#env-step`).
pub(crate) fn path_without(current: &str, target: &str) -> Option<String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    path_without_core(current, target, &cwd, |name| std::env::var(name).ok())
}

fn path_without_core(
    current: &str,
    target: &str,
    cwd: &Path,
    lookup: impl Fn(&str) -> Option<String>,
) -> Option<String> {
    let target_normal = normalize_windows_path(target, cwd, &lookup);
    let kept: Vec<&str> = current
        .split(';')
        .filter(|part| !paths_equal(&normalize_windows_path(part, cwd, &lookup), &target_normal))
        .collect();
    let next = kept.join(";");
    (next != current).then_some(next)
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
