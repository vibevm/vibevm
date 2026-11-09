//! The user's durable environment (`PROP-065#env-step`): the port an env
//! step applies its entries through, and its Linux / macOS implementation.
//! vibe owns `<settings>/env.sh` and `env.fish`, one line per applied entry
//! tagged with the installation key, and one managed block in the user's
//! shell startup files that sources them. The Windows implementation edits
//! `HKCU\Environment` and lives with the caller, which owns the process
//! boundary; this crate reads no environment itself.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#env-step");

pub mod blocks;

use std::fs;
use std::path::{Path, PathBuf};

use specmark::spec;

use crate::index::{EnvEntry, EnvOp, Index};
use crate::settings::{SettingsRoot, write_atomic};

/// Where an env step's entries are applied and removed. Implementations
/// never touch the system environment and edit PATH only by prepending.
///
/// ```
/// use std::cell::RefCell;
/// use vibe_app::{EnvEntry, EnvOp, UserEnvironment};
///
/// /// An environment that only remembers what it was given.
/// #[derive(Default)]
/// struct Recorded(RefCell<Vec<String>>);
///
/// impl UserEnvironment for Recorded {
///     fn apply(&self, key: &str, entries: &[EnvEntry]) -> Result<Vec<EnvEntry>, String> {
///         self.0.borrow_mut().extend(entries.iter().map(|e| format!("{key} +{}", e.name)));
///         Ok(entries.to_vec())
///     }
///     fn remove(&self, key: &str, entries: &[EnvEntry]) -> Result<(), String> {
///         self.0.borrow_mut().extend(entries.iter().map(|e| format!("{key} -{}", e.name)));
///         Ok(())
///     }
/// }
///
/// let env = Recorded::default();
/// let entry = EnvEntry {
///     step: "env".into(), op: EnvOp::Set, name: "A".into(),
///     value: Some("1".into()), previous: None, previous_kind: None,
/// };
/// let applied = env.apply("org.example/a@1.0.0", &[entry]).unwrap();
/// env.remove("org.example/a@1.0.0", &applied).unwrap();
/// assert_eq!(*env.0.borrow(), ["org.example/a@1.0.0 +A", "org.example/a@1.0.0 -A"]);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#env-step")]
pub trait UserEnvironment {
    /// Apply `entries` of installation `key`; return them with any value
    /// they replaced recorded (`previous`), so removal can restore it.
    fn apply(&self, key: &str, entries: &[EnvEntry]) -> Result<Vec<EnvEntry>, String>;

    /// Remove exactly `entries`, earlier applied for `key`.
    fn remove(&self, key: &str, entries: &[EnvEntry]) -> Result<(), String>;
}

/// The Linux / macOS user environment: `<settings>/env.sh` and `env.fish`
/// plus a managed block in `~/.profile`, `~/.bashrc`, `~/.zshrc` and the
/// fish configuration. The block goes into every startup file that exists
/// and always into `~/.profile`; it is removed again once no entry remains,
/// and no byte outside it is ever changed.
///
/// ```
/// use vibe_app::{EnvEntry, EnvOp, ProfileEnvironment, UserEnvironment};
///
/// let dir = tempfile::tempdir().unwrap();
/// let settings = dir.path().join("settings");
/// let home = dir.path().join("home");
/// std::fs::create_dir_all(&home).unwrap();
/// let env = ProfileEnvironment::new(&settings, &home);
/// let entry = EnvEntry {
///     step: "env".into(), op: EnvOp::Set, name: "TOOL_HOME".into(),
///     value: Some("/opt/tool".into()), previous: None, previous_kind: None,
/// };
/// env.apply("org.example/tool@1.0.0", &[entry.clone()]).unwrap();
/// let script = std::fs::read_to_string(settings.join("env.sh")).unwrap();
/// assert!(script.contains("export TOOL_HOME='/opt/tool'"));
/// assert!(std::fs::read_to_string(home.join(".profile")).unwrap().contains("env.sh"));
/// env.remove("org.example/tool@1.0.0", &[entry]).unwrap();
/// assert!(!std::fs::read_to_string(home.join(".profile")).unwrap().contains("env.sh"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#env-step")]
pub struct ProfileEnvironment {
    settings: PathBuf,
    home: PathBuf,
}

const HEADER: &str = "# Managed by vibe (PROP-065#env-step): one line per applied entry, tagged with its installation key. Do not edit.\n";

impl ProfileEnvironment {
    /// `settings` is the user settings root holding `env.sh`; `home` the
    /// user's home directory holding the shell startup files.
    pub fn new(settings: &Path, home: &Path) -> Self {
        Self {
            settings: settings.to_path_buf(),
            home: home.to_path_buf(),
        }
    }

    /// `<settings>/env.sh`, sourced by POSIX shells.
    pub fn script_path(&self) -> PathBuf {
        self.settings.join("env.sh")
    }

    /// `<settings>/env.fish`, sourced by fish.
    pub fn fish_path(&self) -> PathBuf {
        self.settings.join("env.fish")
    }

    fn rewrite(&self, key: &str, drop: &[EnvEntry], add: &[EnvEntry]) -> Result<(), String> {
        let mut remaining = 0;
        for (path, render) in [
            (self.script_path(), posix_line as fn(&EnvEntry) -> String),
            (self.fish_path(), fish_line as fn(&EnvEntry) -> String),
        ] {
            let current = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
                Err(error) => return Err(format!("reading `{}`: {error}", path.display())),
            };
            let dropped: Vec<String> = drop.iter().map(|entry| tag(key, entry)).collect();
            let mut lines: Vec<String> = current
                .lines()
                .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
                .filter(|line| !dropped.iter().any(|tag| line.ends_with(tag.as_str())))
                .map(str::to_owned)
                .collect();
            lines.extend(
                add.iter()
                    .map(|entry| format!("{}{}", render(entry), tag(key, entry))),
            );
            remaining = remaining.max(lines.len());
            if lines.is_empty() {
                match fs::remove_file(&path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(format!("removing `{}`: {error}", path.display())),
                }
                continue;
            }
            let mut text = HEADER.to_owned();
            for line in lines {
                text.push_str(&line);
                text.push('\n');
            }
            write_atomic(&path, text.as_bytes()).map_err(|error| error.to_string())?;
        }
        let script = self.script_path();
        let fish = self.fish_path();
        let startup = blocks::startup_files(&self.home);
        if remaining == 0 {
            for (file, _) in &startup {
                blocks::remove(file)?;
            }
        } else {
            for (file, fish_syntax) in &startup {
                let body = if *fish_syntax {
                    format!(
                        "test -f {0}; and source {0}",
                        quote(&fish.display().to_string())
                    )
                } else {
                    format!("[ -f {0} ] && . {0}", quote(&script.display().to_string()))
                };
                blocks::ensure(file, &body)?;
            }
        }
        Ok(())
    }
}

impl UserEnvironment for ProfileEnvironment {
    fn apply(&self, key: &str, entries: &[EnvEntry]) -> Result<Vec<EnvEntry>, String> {
        self.rewrite(key, entries, entries)?;
        Ok(entries.to_vec())
    }

    fn remove(&self, key: &str, entries: &[EnvEntry]) -> Result<(), String> {
        self.rewrite(key, entries, &[])
    }
}

const TAG: &str = " # vibe-app ";

/// The tag closing one line: installation key, step, operation and name.
fn tag(key: &str, entry: &EnvEntry) -> String {
    format!(
        "{TAG}{key} {} {} {}",
        entry.step,
        entry.op.as_str(),
        entry.name
    )
}

/// A POSIX single-quoted word.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

fn fish_quote(value: &str) -> String {
    format!("'{}'", value.replace('\\', r"\\").replace('\'', r"\'"))
}

fn posix_line(entry: &EnvEntry) -> String {
    let value = entry.value.as_deref().unwrap_or_default();
    match entry.op {
        EnvOp::Set => format!("export {}={}", entry.name, quote(value)),
        EnvOp::Unset => format!("unset {}", entry.name),
        EnvOp::PathPrepend => format!("export PATH={}\"${{PATH:+:$PATH}}\"", quote(value)),
    }
}

fn fish_line(entry: &EnvEntry) -> String {
    let value = entry.value.as_deref().unwrap_or_default();
    match entry.op {
        EnvOp::Set => format!("set -gx {} {}", entry.name, fish_quote(value)),
        EnvOp::Unset => format!("set -e {}", entry.name),
        EnvOp::PathPrepend => format!("set -gx PATH {} $PATH", fish_quote(value)),
    }
}

/// The recorded environment entries of every installation of a home, in
/// key order: what `vibe shell` applies to its session at once.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// assert!(vibe_app::recorded_environment(&settings).unwrap().is_empty());
/// ```
pub fn recorded_environment(settings: &SettingsRoot) -> Result<Vec<EnvEntry>, crate::AppError> {
    let index = Index::load(settings)?;
    Ok(index
        .installations
        .values()
        .flat_map(|record| record.lifecycle.env.iter().cloned())
        .collect())
}

#[cfg(test)]
#[path = "environment/tests.rs"]
mod tests;
