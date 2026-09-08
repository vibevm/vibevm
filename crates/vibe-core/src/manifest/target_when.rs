//! Strict host-independent applicability value shared by package and deploy targets.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#R8-PLATFORM-APPLICABILITY");

use serde::{Deserialize, Serialize};

use super::TargetOs;

/// The canonical first-epoch applicability guard shared by artifact-package
/// and deploy targets.
///
/// The list is an OR-set. Construction rejects empty, duplicate, and
/// all-supported-OS sets, then stores the remaining values in canonical
/// `windows`, `linux`, `macos` order so semantic equality and manifest
/// rewriting cannot disagree.
///
/// ```
/// use vibe_core::manifest::{TargetOs, TargetWhen};
///
/// let guard = TargetWhen::new(vec![TargetOs::Macos, TargetOs::Windows]).unwrap();
/// assert_eq!(guard.os(), [TargetOs::Windows, TargetOs::Macos]);
/// assert!(guard.applies_to(TargetOs::Macos));
/// assert!(!guard.applies_to(TargetOs::Linux));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TargetWhenWire", into = "TargetWhenWire")]
pub struct TargetWhen {
    os: Vec<TargetOs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetWhenWire {
    os: Vec<TargetOs>,
}

impl TargetWhen {
    /// Build one canonical guard.
    pub fn new(mut os: Vec<TargetOs>) -> Result<Self, String> {
        if os.is_empty() {
            return Err(
                "field `when.os` must name at least one of `windows`, `linux`, or `macos`"
                    .to_owned(),
            );
        }
        os.sort_unstable();
        if os.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err("field `when.os` lists an operating system more than once".to_owned());
        }
        if os.len() == 3 {
            return Err(
                "field `when.os` names every supported operating system; remove the redundant `when` guard"
                    .to_owned(),
            );
        }
        Ok(Self { os })
    }

    /// The canonical non-empty OS set.
    #[must_use]
    pub fn os(&self) -> &[TargetOs] {
        &self.os
    }

    /// Evaluate this value against an explicitly injected host OS.
    #[must_use]
    pub fn applies_to(&self, os: TargetOs) -> bool {
        self.os.contains(&os)
    }
}

/// One authored target's pure applicability decision.
///
/// ```
/// use vibe_core::manifest::{TargetApplicability, TargetOs, TargetWhen};
///
/// let guard = TargetWhen::new(vec![TargetOs::Windows]).unwrap();
/// let decision = TargetApplicability::decide("installer", Some(&guard), TargetOs::Linux);
/// assert_eq!(decision.target(), "installer");
/// assert_eq!(decision.status(), "skipped");
/// assert!(decision.reason().contains("linux"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetApplicability {
    target: String,
    when: Option<TargetWhen>,
    observed_os: TargetOs,
    active: bool,
}

impl TargetApplicability {
    /// Decide one target without reading ambient process state.
    #[must_use]
    pub fn decide(target: impl Into<String>, when: Option<&TargetWhen>, os: TargetOs) -> Self {
        Self {
            target: target.into(),
            when: when.cloned(),
            observed_os: os,
            active: when.is_none_or(|guard| guard.applies_to(os)),
        }
    }

    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    #[must_use]
    pub const fn when(&self) -> Option<&TargetWhen> {
        self.when.as_ref()
    }

    #[must_use]
    pub const fn observed_os(&self) -> TargetOs {
        self.observed_os
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    #[must_use]
    pub const fn status(&self) -> &'static str {
        if self.active { "active" } else { "skipped" }
    }

    /// Stable explanation for human and machine plans.
    #[must_use]
    pub fn reason(&self) -> String {
        if self.active {
            return match &self.when {
                Some(_) => format!("the when.os guard includes `{}`", self.observed_os),
                None => "the target is unconditional".to_owned(),
            };
        }
        format!(
            "the when.os guard excludes observed host OS `{}`",
            self.observed_os
        )
    }
}

impl TryFrom<TargetWhenWire> for TargetWhen {
    type Error = String;

    fn try_from(wire: TargetWhenWire) -> Result<Self, Self::Error> {
        Self::new(wire.os)
    }
}

impl From<TargetWhen> for TargetWhenWire {
    fn from(guard: TargetWhen) -> Self {
        Self { os: guard.os }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_when_is_stored_in_canonical_order() {
        let guard = TargetWhen::new(vec![TargetOs::Macos, TargetOs::Windows]).unwrap();
        assert_eq!(guard.os(), [TargetOs::Windows, TargetOs::Macos]);
        assert!(TargetApplicability::decide("x", Some(&guard), TargetOs::Windows).is_active());
        assert_eq!(
            TargetApplicability::decide("x", Some(&guard), TargetOs::Linux).status(),
            "skipped"
        );
    }

    #[test]
    fn target_when_rejects_non_conditions() {
        assert!(
            TargetWhen::new(Vec::new())
                .unwrap_err()
                .contains("at least one")
        );
        assert!(
            TargetWhen::new(vec![TargetOs::Linux, TargetOs::Linux])
                .unwrap_err()
                .contains("more than once")
        );
        assert!(
            TargetWhen::new(vec![TargetOs::Windows, TargetOs::Linux, TargetOs::Macos])
                .unwrap_err()
                .contains("redundant")
        );
    }
}
