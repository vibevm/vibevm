//! The terminal-artifact vocabulary (PROP-043 §3.2).
//!
//! Which artifact kinds close a fact, and the non-empty, duplicate-free
//! set a requirement is written as. Closed exactly like the marker
//! vocabularies beside it in [`super`]: a value outside [`ArtifactKind`]
//! is a validation error, never a silent pass-through.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#terminal-artifacts");

use serde::{Deserialize, Serialize};
use specmark::spec;
use std::fmt;

/// One artifact kind whose presence contributes to closing a fact.
///
/// Declaration order is the canonical wire and rendering order.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#terminal-artifacts")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactKind {
    Specification,
    Implementation,
    Verification,
    Documentation,
    Decision,
    Research,
    Plan,
    Disposition,
    External,
}

impl ArtifactKind {
    pub const ALL: [ArtifactKind; 9] = [
        ArtifactKind::Specification,
        ArtifactKind::Implementation,
        ArtifactKind::Verification,
        ArtifactKind::Documentation,
        ArtifactKind::Decision,
        ArtifactKind::Research,
        ArtifactKind::Plan,
        ArtifactKind::Disposition,
        ArtifactKind::External,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ArtifactKind::Specification => "specification",
            ArtifactKind::Implementation => "implementation",
            ArtifactKind::Verification => "verification",
            ArtifactKind::Documentation => "documentation",
            ArtifactKind::Decision => "decision",
            ArtifactKind::Research => "research",
            ArtifactKind::Plan => "plan",
            ArtifactKind::Disposition => "disposition",
            ArtifactKind::External => "external",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

impl fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A non-empty, duplicate-free artifact set stored in canonical order.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#terminal-artifacts")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRequirements(Vec<ArtifactKind>);

impl ArtifactRequirements {
    pub fn parse_csv(value: &str) -> Result<Self, String> {
        if value.is_empty() {
            return Err("requirements list is empty".into());
        }
        let mut kinds = Vec::new();
        for member in value.split(',') {
            if member.is_empty() {
                return Err("requirements list contains an empty member".into());
            }
            let Some(kind) = ArtifactKind::parse(member) else {
                return Err(format!("unknown required artifact kind `{member}`"));
            };
            if kinds.contains(&kind) {
                return Err(format!("duplicate required artifact kind `{member}`"));
            }
            kinds.push(kind);
        }
        kinds.sort();
        Ok(Self(kinds))
    }

    pub fn iter(&self) -> impl Iterator<Item = ArtifactKind> + '_ {
        self.0.iter().copied()
    }

    pub fn contains(&self, kind: ArtifactKind) -> bool {
        self.0.contains(&kind)
    }

    pub fn to_csv(&self) -> String {
        self.iter()
            .map(ArtifactKind::as_str)
            .collect::<Vec<_>>()
            .join(",")
    }
}

impl Serialize for ArtifactRequirements {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ArtifactRequirements {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = Vec::<ArtifactKind>::deserialize(deserializer)?;
        if raw.is_empty() {
            return Err(serde::de::Error::custom(
                "artifact requirements must not be empty",
            ));
        }
        let csv = raw
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join(",");
        Self::parse_csv(&csv).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_requirements_serde_preserves_the_non_empty_canonical_set() {
        let requirements =
            ArtifactRequirements::parse_csv("external,specification,verification").unwrap();
        assert_eq!(
            serde_json::to_string(&requirements).unwrap(),
            r#"["specification","verification","external"]"#
        );
        let decoded: ArtifactRequirements =
            serde_json::from_str(r#"["external","specification"]"#).unwrap();
        assert_eq!(decoded.to_csv(), "specification,external");
        assert!(serde_json::from_str::<ArtifactRequirements>("[]").is_err());
        assert!(serde_json::from_str::<ArtifactRequirements>(r#"["plan","plan"]"#).is_err());
    }
}
