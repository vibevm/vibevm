//! The terminal-artifact vocabulary (PROP-043 §3.2).
//!
//! Which artifact kinds close a fact, and the non-empty, duplicate-free
//! set a requirement is written as. Closed exactly like the marker
//! vocabularies beside it in [`super`]: a value outside [`ArtifactKind`]
//! is a validation error, never a silent pass-through.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#terminal-artifacts");

use specmark::spec;
use std::fmt;

pub use crate::generated::artifact_requirements::{ArtifactKind, ArtifactRequirements};

/// One artifact kind whose presence contributes to closing a fact.
///
/// Declaration order is the canonical wire and rendering order.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#terminal-artifacts")]
impl fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A non-empty, duplicate-free artifact set stored in canonical order.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#terminal-artifacts")]
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
        Self::from_vec(kinds)
    }

    pub fn iter(&self) -> impl Iterator<Item = ArtifactKind> + '_ {
        self.as_slice().iter().copied()
    }

    pub fn contains(&self, kind: ArtifactKind) -> bool {
        self.as_slice().contains(&kind)
    }

    pub fn to_csv(&self) -> String {
        self.iter()
            .map(ArtifactKind::as_str)
            .collect::<Vec<_>>()
            .join(",")
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
        assert!(serde_json::from_str::<ArtifactRequirements>(r#"["spaceship"]"#).is_err());
        assert!(serde_json::from_str::<ArtifactRequirements>(r#"{"plan":true}"#).is_err());
    }
}
