//! The Windows user environment (PROP-019 §2.6): raw `HKCU\Environment`
//! values read and written through the registry API with their kind kept,
//! and the `WM_SETTINGCHANGE` broadcast after every change. No child process
//! is involved: spawning a shell for each value cost hundreds of milliseconds,
//! depended on the execution policy and quoting, and failed outright when
//! PATH could not resolve the shell (B-226).

use std::path::Path;

use anyhow::Result;

use super::{EnvPersister, Persisted, path_with_prefix};

pub(crate) use native::{delete_user_var_raw, get_user_var_raw, set_user_var_raw};

/// The kind of a raw `HKCU\Environment` value; also used by application
/// env steps (PROP-065 `#env-step`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegistryValueKind {
    /// `REG_SZ`: stored and read verbatim.
    String,
    /// `REG_EXPAND_SZ`: `%VAR%` references expand when a process starts.
    ExpandString,
}

/// One raw `HKCU\Environment` value, read without expanding `%VAR%`.
#[derive(Debug)]
pub(crate) struct RawUserVar {
    pub(crate) value: String,
    pub(crate) kind: RegistryValueKind,
}

/// A Windows persister: edits the raw values in `HKCU\Environment`, preserving
/// `REG_SZ` versus `REG_EXPAND_SZ`, then broadcasts `WM_SETTINGCHANGE`.
/// Registry I/O is never exercised by gate tests because it mutates real user
/// state; the pure PATH transformation carries the test coverage, and an
/// opt-in test below round-trips a scratch value on a Windows host.
pub(crate) struct WindowsEnvPersister;

impl EnvPersister for WindowsEnvPersister {
    fn set_vibevm_home(&self, home: &Path) -> Result<Persisted> {
        let target = home.display().to_string();
        let current = get_user_var_raw("VIBEVM_HOME")?;
        if current.as_ref().is_some_and(|value| value.value == target) {
            return Ok(Persisted::Unchanged);
        }
        let kind = current
            .map(|value| value.kind)
            .unwrap_or(RegistryValueKind::String);
        set_user_var_raw("VIBEVM_HOME", &target, kind)?;
        Ok(Persisted::Changed)
    }

    fn ensure_on_path(&self, dir: &Path) -> Result<Persisted> {
        let current = get_user_var_raw("Path")?;
        let raw = current
            .as_ref()
            .map(|value| value.value.as_str())
            .unwrap_or_default();
        match path_with_prefix(raw, &dir.display().to_string()) {
            None => Ok(Persisted::Unchanged),
            Some(next) => {
                // A user PATH conventionally carries `%USERPROFILE%`-style
                // entries, so a fresh one is born expandable.
                let kind = current
                    .map(|value| value.kind)
                    .unwrap_or(RegistryValueKind::ExpandString);
                set_user_var_raw("Path", &next, kind)?;
                Ok(Persisted::Changed)
            }
        }
    }

    fn activation_hint(&self) -> String {
        "open a new terminal (the registry change reaches new processes)".to_string()
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
#[path = "env_registry_windows.rs"]
mod native;

/// Other hosts have no user registry. The callers pick the rc-file path
/// there, so these exist only to keep one call surface on every target.
#[cfg(not(windows))]
mod native {
    use anyhow::{Result, bail};

    use super::{RawUserVar, RegistryValueKind};

    const NO_REGISTRY: &str = "HKCU\\Environment exists only on Windows";

    pub(crate) fn get_user_var_raw(_name: &str) -> Result<Option<RawUserVar>> {
        bail!(NO_REGISTRY)
    }

    pub(crate) fn set_user_var_raw(
        _name: &str,
        _value: &str,
        _kind: RegistryValueKind,
    ) -> Result<()> {
        bail!(NO_REGISTRY)
    }

    pub(crate) fn delete_user_var_raw(_name: &str) -> Result<()> {
        bail!(NO_REGISTRY)
    }
}
