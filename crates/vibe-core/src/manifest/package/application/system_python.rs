//! Selected-option system interpreter contract; observation belongs to the host.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#system-python");

use serde::{Deserialize, Serialize};
use specmark::spec;

/// Fixed Python capability, not an executable name or package-authored probe.
///
/// ```
/// use vibe_core::manifest::SystemPythonDecl;
///
/// let python: SystemPythonDecl = toml::from_str(
///     "implementation = 'cpython'\nmajor = 3\nminor = 14\nsoabi = 'cpython-314-x86_64-linux-gnu'",
/// )
/// .unwrap();
/// assert_eq!(python.minor, 14);
/// assert!(python.accepts("cpython", 3, 14, "cpython-314-x86_64-linux-gnu"));
/// assert!(!python.accepts("pypy", 3, 14, "cpython-314-x86_64-linux-gnu"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemPythonDecl {
    pub implementation: String,
    pub major: u16,
    pub minor: u16,
    pub soabi: String,
}

impl SystemPythonDecl {
    /// Initial admitted native archive target. Platform/architecture remain
    /// properties of the containing installer option, not another vocabulary.
    pub(crate) fn validate(&self, platform: &str, arch: &str) -> Result<(), String> {
        if self.implementation != "cpython" || self.major != 3 || self.minor != 14 {
            return Err("system_python currently requires CPython 3.14".into());
        }
        if platform != "linux" || arch != "x86_64" || self.soabi != "cpython-314-x86_64-linux-gnu" {
            return Err(
                "system_python currently supports only the Linux x64 CPython 3.14 ABI".into(),
            );
        }
        Ok(())
    }

    /// Judge an already observed typed result. No subprocess or ambient lookup.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#system-python")]
    pub fn accepts(&self, implementation: &str, major: u16, minor: u16, soabi: &str) -> bool {
        self.implementation == implementation
            && self.major == major
            && self.minor == minor
            && self.soabi == soabi
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_python_is_an_exact_capability_not_version_prose() {
        let declared: SystemPythonDecl = toml::from_str(
            "implementation = 'cpython'\nmajor = 3\nminor = 14\nsoabi = 'cpython-314-x86_64-linux-gnu'",
        )
        .unwrap();
        assert!(declared.validate("linux", "x86_64").is_ok());
        assert!(declared.accepts("cpython", 3, 14, "cpython-314-x86_64-linux-gnu"));
        assert!(!declared.accepts("pypy", 3, 14, "cpython-314-x86_64-linux-gnu"));
        assert!(!declared.accepts("cpython", 3, 13, "cpython-314-x86_64-linux-gnu"));
        assert!(!declared.accepts("cpython", 3, 14, "cpython-314t-x86_64-linux-gnu"));
        assert!(declared.validate("windows", "x64").is_err());
        assert!(declared.validate("linux", "arm64").is_err());
        assert!(
            toml::from_str::<SystemPythonDecl>(
                "implementation='cpython'\nmajor=3\nminor=14\nsoabi='x'\nexecutable='user-script'"
            )
            .is_err()
        );
    }
}
