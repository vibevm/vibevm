//! Strict host-independent applicability value shared by package and deploy targets.

use serde::{Deserialize, Serialize};

use super::TargetOs;

/// The canonical first-epoch applicability guard shared by artifact-package
/// and deploy targets.
///
/// The list is an OR-set. Construction rejects empty, duplicate, and
/// all-supported-OS sets, then stores the remaining values in canonical
/// `windows`, `linux`, `macos` order so semantic equality and manifest
/// rewriting cannot disagree.
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
