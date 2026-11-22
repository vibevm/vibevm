//! Typed system interpreter observation and pure selected-option admission.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#system-python");
use specmark::spec;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use vibe_core::manifest::SystemPythonDecl;

/// Complete result of the host's fixed isolated probe. Never version prose.
///
/// ```
/// use vibe_app::SystemPythonObservation;
/// use vibe_core::manifest::SystemPythonDecl;
///
/// let required = SystemPythonDecl {
///     implementation: "cpython".into(),
///     major: 3,
///     minor: 14,
///     soabi: "cpython-314-x86_64-linux-gnu".into(),
/// };
/// let root = std::env::temp_dir();
/// let observed = SystemPythonObservation {
///     executable: root.join("bin").join("python3.14"),
///     implementation: "cpython".into(),
///     major: 3,
///     minor: 14,
///     soabi: required.soabi.clone(),
///     prefix: root.clone(),
///     base_prefix: root.clone(),
/// };
/// assert!(observed.admit(&required).is_ok());
/// // A virtual environment is not the system interpreter, whatever its ABI.
/// let virtual_env = SystemPythonObservation { prefix: root.join("venv"), ..observed.clone() };
/// assert!(virtual_env.admit(&required).is_err());
/// // Neither is a compatible-looking interpreter of another minor version.
/// let older = SystemPythonObservation { minor: 13, ..observed };
/// assert!(older.admit(&required).is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemPythonObservation {
    pub executable: PathBuf,
    pub implementation: String,
    pub major: u16,
    pub minor: u16,
    pub soabi: String,
    pub prefix: PathBuf,
    pub base_prefix: PathBuf,
}

pub type SystemPythonProbe<'a> = &'a dyn Fn() -> Result<SystemPythonObservation, String>;

impl SystemPythonObservation {
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#system-python")]
    pub fn admit(&self, required: &SystemPythonDecl) -> Result<(), String> {
        if !self.executable.is_absolute()
            || !self.prefix.is_absolute()
            || self.prefix != self.base_prefix
        {
            return Err(
                "system Python probe did not identify a non-virtual system interpreter".into(),
            );
        }
        if !required.accepts(&self.implementation, self.major, self.minor, &self.soabi) {
            return Err("system Python is incompatible; provide CPython 3.14 with the declared Linux x64 ABI, then retry (Vibe does not install Python)".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn system_python_observation_requires_exact_abi_and_system_prefix() {
        let required = SystemPythonDecl {
            implementation: "cpython".into(),
            major: 3,
            minor: 14,
            soabi: "cpython-314-x86_64-linux-gnu".into(),
        };
        let valid = SystemPythonObservation {
            executable: "/usr/bin/python3.14".into(),
            implementation: "cpython".into(),
            major: 3,
            minor: 14,
            soabi: required.soabi.clone(),
            prefix: "/usr".into(),
            base_prefix: "/usr".into(),
        };
        assert!(valid.admit(&required).is_ok());
        for invalid in [
            SystemPythonObservation {
                implementation: "pypy".into(),
                ..valid.clone()
            },
            SystemPythonObservation {
                soabi: "wrong".into(),
                ..valid.clone()
            },
            SystemPythonObservation {
                minor: 13,
                ..valid.clone()
            },
            SystemPythonObservation {
                prefix: "/venv".into(),
                ..valid.clone()
            },
            SystemPythonObservation {
                executable: "python3".into(),
                ..valid.clone()
            },
        ] {
            assert!(invalid.admit(&required).is_err());
        }
    }
}
