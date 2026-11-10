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
/// false one is physically absent (PROP-049 §3). `target:<name>` is resolved
/// at generation time too, against the project's own declared
/// `[system_prompt] target` (PROP-076 `#SYS-HOST-AND-TARGET`).
///
/// ```
/// use vibe_core::manifest::WhenCondition;
///
/// let cond: WhenCondition = "os:linux".parse().unwrap();
/// assert_eq!(cond.to_string(), "os:linux");
/// let installed: WhenCondition = "installed:org.vibevm.world/wal".parse().unwrap();
/// assert_eq!(installed.to_string(), "installed:org.vibevm.world/wal");
/// let target: WhenCondition = "target:windows".parse().unwrap();
/// assert_eq!(target.to_string(), "target:windows");
/// assert!("os:beos".parse::<WhenCondition>().is_err()); // unknown OS
/// assert!("target:beos".parse::<WhenCondition>().is_err()); // likewise
/// ```
///
/// The two operating-system gates ask different questions and are never
/// interchangeable: `os:` is about the machine the session runs on, `target:`
/// about the platform the product addresses. A Linux workstation building
/// Windows software holds both at once.
///
/// ```
/// use vibe_core::manifest::{TargetOs, WhenCondition};
///
/// let host: WhenCondition = "os:linux".parse().unwrap();
/// let product: WhenCondition = "target:windows".parse().unwrap();
/// assert_eq!(host.session_os(), Some(TargetOs::Linux));
/// assert_eq!(host.target_os(), None);
/// assert_eq!(product.target_os(), Some(TargetOs::Windows));
/// assert_eq!(product.session_os(), None);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum WhenCondition {
    /// Activates only when the session's operating system is the named one.
    Os(TargetOs),
    /// Activates only when the resolved project contains this package.
    Installed { group: Group, name: PackageName },
    /// Activates only when the named operating system is one the **product**
    /// is for — the project's `[system_prompt] target`, never the session's
    /// own OS. It is a property of the project, so it is always resolved
    /// while artifacts are generated and never reaches `INDEX.md`
    /// (PROP-076 `#SYS-HOST-AND-TARGET`).
    Target(TargetOs),
}

impl WhenCondition {
    /// `true` when this condition holds for the operating system the
    /// current process runs on.
    ///
    /// Only a session gate can hold here. A `target:` gate is about the
    /// product's platform, not the running machine, so it never answers this
    /// question — the generator resolves it against the declared selection
    /// instead, and asking the host would be exactly the confusion the two
    /// spellings exist to prevent.
    pub fn matches_current_os(&self) -> bool {
        match self {
            WhenCondition::Os(os) => TargetOs::current() == Some(*os),
            WhenCondition::Installed { .. } | WhenCondition::Target(_) => false,
        }
    }

    /// The package identity named by an install-time predicate.
    pub fn installed_identity(&self) -> Option<(&Group, &str)> {
        match self {
            WhenCondition::Installed { group, name } => Some((group, name.as_str())),
            WhenCondition::Os(_) | WhenCondition::Target(_) => None,
        }
    }

    /// The operating system a **session** gate names — the axis an `os:`
    /// condition has always been about.
    pub fn session_os(&self) -> Option<TargetOs> {
        match self {
            WhenCondition::Os(os) => Some(*os),
            WhenCondition::Installed { .. } | WhenCondition::Target(_) => None,
        }
    }

    /// The operating system a **product** gate names — the axis a `target:`
    /// condition is about.
    pub fn target_os(&self) -> Option<TargetOs> {
        match self {
            WhenCondition::Target(os) => Some(*os),
            WhenCondition::Os(_) | WhenCondition::Installed { .. } => None,
        }
    }
}

impl std::fmt::Display for WhenCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WhenCondition::Os(os) => write!(f, "os:{os}"),
            WhenCondition::Installed { group, name } => write!(f, "installed:{group}/{name}"),
            WhenCondition::Target(os) => write!(f, "target:{os}"),
        }
    }
}

impl std::str::FromStr for WhenCondition {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        if let Some(os_name) = s.strip_prefix("os:") {
            return parse_os(s, os_name).map(WhenCondition::Os);
        }
        if let Some(os_name) = s.strip_prefix("target:") {
            return parse_os(s, os_name).map(WhenCondition::Target);
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
            reason: "unrecognised condition — expected `os:<name>`, `target:<name>`, or \
                     `installed:<group>/<name>`"
                .to_string(),
        })
    }
}

/// The one operating-system spelling both OS gates share, so `os:` and
/// `target:` cannot drift apart in the set they accept or in the refusal
/// they produce. The condition itself is named in full, because that is what
/// the author wrote.
fn parse_os(condition: &str, os_name: &str) -> Result<TargetOs> {
    match os_name {
        "windows" => Ok(TargetOs::Windows),
        "macos" => Ok(TargetOs::Macos),
        "linux" => Ok(TargetOs::Linux),
        other => Err(Error::BadWhenCondition {
            input: condition.to_string(),
            reason: format!(
                "unknown operating system `{other}` — expected `windows`, `macos`, or `linux`"
            ),
        }),
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
        let message = err.to_string();
        // The refusal names every accepted spelling, including the newest.
        for spelling in ["os:<name>", "target:<name>", "installed:<group>/<name>"] {
            assert!(message.contains(spelling), "{message}");
        }
    }

    // --- PROP-076 `#SYS-HOST-AND-TARGET` — the `target:` product gate -----

    #[test]
    fn when_condition_parses_each_supported_target_os() {
        use std::str::FromStr;
        for (input, os) in [
            ("target:windows", TargetOs::Windows),
            ("target:macos", TargetOs::Macos),
            ("target:linux", TargetOs::Linux),
        ] {
            let parsed = WhenCondition::from_str(input).unwrap();
            assert_eq!(parsed, WhenCondition::Target(os));
            assert_eq!(parsed.to_string(), input, "round-trips through Display");
        }
    }

    #[test]
    fn a_target_gate_rejects_an_unknown_os_exactly_as_the_session_gate_does() {
        // One spelling of the OS set for both gates: the two refusals differ
        // only in the condition they quote back.
        let session = "os:winows"
            .parse::<WhenCondition>()
            .unwrap_err()
            .to_string();
        let product = "target:winows"
            .parse::<WhenCondition>()
            .unwrap_err()
            .to_string();
        assert!(
            session.contains("`windows`, `macos`, or `linux`"),
            "{session}"
        );
        assert!(
            product.contains("`windows`, `macos`, or `linux`"),
            "{product}"
        );
        assert!(product.contains("target:winows"), "{product}");
    }

    #[test]
    fn a_target_gate_is_never_a_session_predicate() {
        // The whole point of two spellings: `target:` says nothing about the
        // machine the session runs on, so it must not answer the host
        // question even when the names coincide.
        let current = TargetOs::current().expect("test host is a supported OS");
        assert!(!WhenCondition::Target(current).matches_current_os());
        assert_eq!(WhenCondition::Target(current).session_os(), None);
        assert_eq!(WhenCondition::Target(current).installed_identity(), None);
    }

    #[test]
    fn the_two_os_axes_are_read_through_separate_accessors() {
        let session = WhenCondition::Os(TargetOs::Linux);
        let product = WhenCondition::Target(TargetOs::Windows);
        assert_eq!(session.session_os(), Some(TargetOs::Linux));
        assert_eq!(session.target_os(), None);
        assert_eq!(product.target_os(), Some(TargetOs::Windows));
        assert_eq!(product.session_os(), None);
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
