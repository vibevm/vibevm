//! The `when` activation condition a `[boot_snippet]` contribution can carry.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#git-source");

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::package_ref::{Group, PackageName};

use super::TargetOs;

/// A `[boot_snippet]` activation condition (PROP-009 §2.4 / §2.6).
///
/// A boot snippet carrying a `when` is a **conditional** contribution:
/// `vibe` renders it as a `dynamic` `INDEX.md` entry — irrespective of any
/// `link`, since a condition cannot be honoured by the verbatim `inline`
/// lane or a direct `static` read — and the agent reads the file at boot
/// only when the condition holds.
///
/// `os:<name>` is preserved in generated `INDEX.md` for the reading session.
/// `installed:<group>/<name>` is resolved while artifacts are generated and
/// never reaches `INDEX.md`: a true contribution becomes unconditional and a
/// false one is physically absent (PROP-049 §3).
///
/// ```
/// use vibe_core::manifest::WhenCondition;
///
/// let cond: WhenCondition = "os:linux".parse().unwrap();
/// assert_eq!(cond.to_string(), "os:linux");
/// let installed: WhenCondition = "installed:org.vibevm.world/wal".parse().unwrap();
/// assert_eq!(installed.to_string(), "installed:org.vibevm.world/wal");
/// assert!("os:beos".parse::<WhenCondition>().is_err()); // unknown OS
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum WhenCondition {
    /// Activates only when the session's operating system is the named one.
    Os(TargetOs),
    /// Activates only when the resolved project contains this package.
    Installed { group: Group, name: PackageName },
}

impl WhenCondition {
    /// `true` when this condition holds for the operating system the
    /// current process runs on.
    pub fn matches_current_os(&self) -> bool {
        match self {
            WhenCondition::Os(os) => TargetOs::current() == Some(*os),
            WhenCondition::Installed { .. } => false,
        }
    }

    /// The package identity named by an install-time predicate.
    pub fn installed_identity(&self) -> Option<(&Group, &str)> {
        match self {
            WhenCondition::Installed { group, name } => Some((group, name.as_str())),
            WhenCondition::Os(_) => None,
        }
    }
}

impl std::fmt::Display for WhenCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WhenCondition::Os(os) => write!(f, "os:{os}"),
            WhenCondition::Installed { group, name } => write!(f, "installed:{group}/{name}"),
        }
    }
}

impl std::str::FromStr for WhenCondition {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        if let Some(os_name) = s.strip_prefix("os:") {
            return match os_name {
                "windows" => Ok(WhenCondition::Os(TargetOs::Windows)),
                "macos" => Ok(WhenCondition::Os(TargetOs::Macos)),
                "linux" => Ok(WhenCondition::Os(TargetOs::Linux)),
                other => Err(Error::BadWhenCondition {
                    input: s.to_string(),
                    reason: format!(
                        "unknown operating system `{other}` — expected `windows`, `macos`, or `linux`"
                    ),
                }),
            };
        }
        if let Some(identity) = s.strip_prefix("installed:") {
            let Some((group, name)) = identity.split_once('/') else {
                return Err(Error::BadWhenCondition {
                    input: s.to_string(),
                    reason: "invalid installed-package identity — expected `<group>/<name>`"
                        .to_string(),
                });
            };
            let group = Group::parse(group).map_err(|error| Error::BadWhenCondition {
                input: s.to_string(),
                reason: format!("invalid installed-package group: {error}"),
            })?;
            let name = PackageName::parse(name).map_err(|error| Error::BadWhenCondition {
                input: s.to_string(),
                reason: format!("invalid installed-package name: {error}"),
            })?;
            return Ok(WhenCondition::Installed { group, name });
        }
        Err(Error::BadWhenCondition {
            input: s.to_string(),
            reason: "unrecognised condition — expected `os:<name>` or `installed:<group>/<name>`"
                .to_string(),
        })
    }
}

impl TryFrom<String> for WhenCondition {
    type Error = String;

    fn try_from(s: String) -> std::result::Result<Self, String> {
        s.parse().map_err(|e: Error| e.to_string())
    }
}

impl From<WhenCondition> for String {
    fn from(w: WhenCondition) -> String {
        w.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- PROP-009 §2.4 / §2.6 — the `when` OS gate ----------------------

    #[test]
    fn when_condition_parses_each_supported_os() {
        use std::str::FromStr;
        assert_eq!(
            WhenCondition::from_str("os:windows").unwrap(),
            WhenCondition::Os(TargetOs::Windows)
        );
        assert_eq!(
            WhenCondition::from_str("os:macos").unwrap(),
            WhenCondition::Os(TargetOs::Macos)
        );
        assert_eq!(
            WhenCondition::from_str("os:linux").unwrap(),
            WhenCondition::Os(TargetOs::Linux)
        );
    }

    #[test]
    fn when_condition_display_round_trips_through_from_str() {
        use std::str::FromStr;
        for cond in [
            WhenCondition::Os(TargetOs::Windows),
            WhenCondition::Os(TargetOs::Macos),
            WhenCondition::Os(TargetOs::Linux),
        ] {
            assert_eq!(WhenCondition::from_str(&cond.to_string()).unwrap(), cond);
        }
        assert_eq!(WhenCondition::Os(TargetOs::Linux).to_string(), "os:linux");
        let installed = WhenCondition::from_str("installed:org.vibevm.world/wal").unwrap();
        assert_eq!(
            WhenCondition::from_str(&installed.to_string()).unwrap(),
            installed
        );
    }

    #[test]
    fn when_condition_rejects_an_unrecognised_prefix() {
        let err = "rust".parse::<WhenCondition>().unwrap_err();
        assert!(
            err.to_string().contains("installed:<group>/<name>"),
            "{err}"
        );
    }

    #[test]
    fn when_condition_rejects_an_unknown_os() {
        let err = "os:winows".parse::<WhenCondition>().unwrap_err();
        // The diagnostic names the full condition and the bad OS.
        assert!(err.to_string().contains("os:winows"), "{err}");
        assert!(err.to_string().contains("winows"), "{err}");
    }

    #[test]
    fn when_condition_rejects_malformed_installed_identity_with_recipe() {
        for input in [
            "installed:wal",
            "installed:Org.Vibevm/wal",
            "installed:org.vibevm/Not-Kebab",
            "installed:org.vibevm/wal/extra",
        ] {
            let err = input.parse::<WhenCondition>().unwrap_err();
            let message = err.to_string();
            assert!(message.contains(input), "{message}");
            assert!(message.contains("installed:<group>/<name>"), "{message}");
        }
    }

    #[test]
    fn when_condition_matches_the_running_os() {
        // The test process runs on one of the supported OSes (CI: linux,
        // dev: windows); the matching condition holds and a different one
        // does not.
        let current = TargetOs::current().expect("test host is a supported OS");
        assert!(WhenCondition::Os(current).matches_current_os());
        let other = match current {
            TargetOs::Linux => TargetOs::Windows,
            TargetOs::Windows | TargetOs::Macos => TargetOs::Linux,
        };
        assert!(!WhenCondition::Os(other).matches_current_os());
    }
}
