//! The `when` activation condition a `[boot_snippet]` can carry — the OS
//! gate of the loading model (PROP-009 §2.4 / §2.6).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#git-source");

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

use super::TargetOs;

/// A `[boot_snippet]` activation condition (PROP-009 §2.4 / §2.6).
///
/// A boot snippet carrying a `when` is a **conditional** contribution:
/// `vibe` renders it as a `dynamic` `INDEX.md` entry — irrespective of any
/// `link`, since a condition cannot be honoured by the verbatim `inline`
/// lane or a direct `static` read — and the agent reads the file at boot
/// only when the condition holds.
///
/// For v1 the sole condition is an operating-system match — enough to ship
/// OS-specific packages and subskills. The wire form is the string
/// `"os:<name>"`, `<name>` one of `windows` / `macos` / `linux`; it is
/// carried verbatim into the generated `INDEX.md`. The richer `[activation]`
/// probe vocabulary (PROP-003 §2.5) folds in when that engine is built;
/// `os:` is its first probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum WhenCondition {
    /// Activates only when the session's operating system is the named one.
    Os(TargetOs),
}

impl WhenCondition {
    /// `true` when this condition holds for the operating system the
    /// current process runs on.
    pub fn matches_current_os(self) -> bool {
        match self {
            WhenCondition::Os(os) => TargetOs::current() == Some(os),
        }
    }
}

impl std::fmt::Display for WhenCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WhenCondition::Os(os) => write!(f, "os:{os}"),
        }
    }
}

impl std::str::FromStr for WhenCondition {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let Some(os_name) = s.strip_prefix("os:") else {
            return Err(Error::BadWhenCondition {
                input: s.to_string(),
                reason: "unrecognised condition — expected `os:<name>`".to_string(),
            });
        };
        match os_name {
            "windows" => Ok(WhenCondition::Os(TargetOs::Windows)),
            "macos" => Ok(WhenCondition::Os(TargetOs::Macos)),
            "linux" => Ok(WhenCondition::Os(TargetOs::Linux)),
            other => Err(Error::BadWhenCondition {
                input: s.to_string(),
                reason: format!(
                    "unknown operating system `{other}` — expected `windows`, `macos`, or `linux`"
                ),
            }),
        }
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
    }

    #[test]
    fn when_condition_rejects_an_unrecognised_prefix() {
        // The `os:` namespace is the only one v1 understands — a bare
        // probe name from the wider `[activation]` vocabulary is rejected
        // until that engine lands.
        let err = "rust".parse::<WhenCondition>().unwrap_err();
        assert!(err.to_string().contains("expected `os:<name>`"), "{err}");
    }

    #[test]
    fn when_condition_rejects_an_unknown_os() {
        let err = "os:winows".parse::<WhenCondition>().unwrap_err();
        // The diagnostic names the full condition and the bad OS.
        assert!(err.to_string().contains("os:winows"), "{err}");
        assert!(err.to_string().contains("winows"), "{err}");
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
