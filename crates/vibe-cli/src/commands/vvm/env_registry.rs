//! The Windows user environment (PROP-019 §2.6): raw `HKCU\Environment`
//! values read and written through the registry API with their kind kept,
//! and the `WM_SETTINGCHANGE` broadcast after every change. No child process
//! is involved: spawning a shell for each value cost hundreds of milliseconds,
//! depended on the execution policy and quoting, and failed outright when
//! PATH could not resolve the shell (B-226).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3);
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawUserVar {
    pub(crate) value: String,
    pub(crate) kind: RegistryValueKind,
}

/// Raw access to the user's environment variables: the seam that lets the
/// Windows persister's logic run against an in-memory store on any host.
/// Every write keeps the kind the caller names; nothing expands `%VAR%`.
pub(crate) trait UserRegistry {
    /// Read one raw value; `None` when it is absent.
    fn get_raw(&self, name: &str) -> Result<Option<RawUserVar>>;
    /// Write one raw value with its kind and announce the change.
    fn set_raw(&self, name: &str, value: &str, kind: RegistryValueKind) -> Result<()>;
}

/// The real `HKCU\Environment`: direct registry API calls followed by the
/// `WM_SETTINGCHANGE` broadcast, with no child process.
pub(crate) struct Hkcu;

impl UserRegistry for Hkcu {
    fn get_raw(&self, name: &str) -> Result<Option<RawUserVar>> {
        get_user_var_raw(name)
    }
    fn set_raw(&self, name: &str, value: &str, kind: RegistryValueKind) -> Result<()> {
        set_user_var_raw(name, value, kind)
    }
}

/// The persister logic over any [`UserRegistry`]: prepend the shim directory
/// to the raw user `Path` (deduping normalized equivalents case-insensitively
/// and keeping every other entry byte-for-byte, `%USERPROFILE%` text
/// included), preserve `REG_SZ` versus `REG_EXPAND_SZ`, and write nothing
/// when the value is already canonical.
pub(crate) struct RegistryEnvPersister<R> {
    registry: R,
}

impl<R: UserRegistry> RegistryEnvPersister<R> {
    pub(crate) fn new(registry: R) -> Self {
        Self { registry }
    }

    #[cfg(test)]
    pub(crate) fn registry(&self) -> &R {
        &self.registry
    }

    /// Write one path-valued user variable, keeping the kind the existing
    /// value carries (`REG_EXPAND_SZ` stays expandable, so an operator's
    /// `%USERPROFILE%`-style value is not silently frozen) and writing
    /// nothing when it already reads exactly so.
    fn set_var(&self, name: &str, value: &Path) -> Result<Persisted> {
        let target = value.display().to_string();
        let current = self.registry.get_raw(name)?;
        if current.as_ref().is_some_and(|value| value.value == target) {
            return Ok(Persisted::Unchanged);
        }
        let kind = current
            .map(|value| value.kind)
            .unwrap_or(RegistryValueKind::String);
        self.registry.set_raw(name, &target, kind)?;
        Ok(Persisted::Changed)
    }
}

impl<R: UserRegistry> EnvPersister for RegistryEnvPersister<R> {
    fn inspect_path(&self, dir: &Path) -> super::EnvironmentObservation {
        super::profile::inspect_registry(self.registry.get_raw("Path"), dir)
    }

    fn set_vibevm_home(&self, home: &Path) -> Result<Persisted> {
        self.set_var("VIBEVM_HOME", home)
    }

    fn set_install_root(&self, root: &Path) -> Result<Persisted> {
        self.set_var("VIBEVM_INSTALL_ROOT", root)
    }

    fn ensure_on_path(&self, dir: &Path) -> Result<Persisted> {
        let current = self.registry.get_raw("Path")?;
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
                self.registry.set_raw("Path", &next, kind)?;
                Ok(Persisted::Changed)
            }
        }
    }

    fn activation_hint(&self) -> String {
        "open a new terminal (the registry change reaches new processes)".to_string()
    }
}

/// The Windows persister: [`RegistryEnvPersister`] over the real
/// [`Hkcu`]. Registry I/O is never exercised by gate tests because it
/// mutates real user state; the logic is covered through an in-memory
/// [`UserRegistry`], and an opt-in test round-trips a scratch value on a
/// Windows host.
pub(crate) struct WindowsEnvPersister;

impl WindowsEnvPersister {
    fn real() -> RegistryEnvPersister<Hkcu> {
        RegistryEnvPersister::new(Hkcu)
    }
}

impl EnvPersister for WindowsEnvPersister {
    fn inspect_path(&self, dir: &Path) -> super::EnvironmentObservation {
        Self::real().inspect_path(dir)
    }
    fn set_vibevm_home(&self, home: &Path) -> Result<Persisted> {
        Self::real().set_vibevm_home(home)
    }
    fn set_install_root(&self, root: &Path) -> Result<Persisted> {
        Self::real().set_install_root(root)
    }
    fn ensure_on_path(&self, dir: &Path) -> Result<Persisted> {
        Self::real().ensure_on_path(dir)
    }
    fn activation_hint(&self) -> String {
        Self::real().activation_hint()
    }
}

/// An in-memory user environment for tests: names compare
/// case-insensitively as on Windows, values and kinds are kept verbatim, and
/// every write is counted so idempotence is observable.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct MemoryRegistry {
    values: std::cell::RefCell<std::collections::BTreeMap<String, RawUserVar>>,
    writes: std::cell::Cell<usize>,
}

#[cfg(test)]
impl MemoryRegistry {
    pub(crate) fn with(values: &[(&str, &str, RegistryValueKind)]) -> Self {
        let registry = Self::default();
        for (name, value, kind) in values {
            registry.values.borrow_mut().insert(
                name.to_ascii_uppercase(),
                RawUserVar {
                    value: (*value).to_string(),
                    kind: *kind,
                },
            );
        }
        registry
    }

    pub(crate) fn value(&self, name: &str) -> Option<RawUserVar> {
        self.values
            .borrow()
            .get(&name.to_ascii_uppercase())
            .cloned()
    }

    pub(crate) fn writes(&self) -> usize {
        self.writes.get()
    }
}

#[cfg(test)]
impl UserRegistry for MemoryRegistry {
    fn get_raw(&self, name: &str) -> Result<Option<RawUserVar>> {
        Ok(self.value(name))
    }
    fn set_raw(&self, name: &str, value: &str, kind: RegistryValueKind) -> Result<()> {
        self.values.borrow_mut().insert(
            name.to_ascii_uppercase(),
            RawUserVar {
                value: value.to_string(),
                kind,
            },
        );
        self.writes.set(self.writes.get() + 1);
        Ok(())
    }
}

#[cfg(test)]
#[path = "env_registry_tests.rs"]
mod tests;

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
