//! Closed terminal-artifact requirement vocabulary for XML fact metadata.
//! The specmap frontend keeps this helper local: it validates and projects
//! authored metadata, but derives no terminal verdict and adds no wire field.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#REQUIRED-ARTIFACT-KINDS"
);

use super::reader::Violation;

const CANONICAL_KINDS: [ArtifactKind; 9] = [
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ArtifactKind {
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
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "specification" => Self::Specification,
            "implementation" => Self::Implementation,
            "verification" => Self::Verification,
            "documentation" => Self::Documentation,
            "decision" => Self::Decision,
            "research" => Self::Research,
            "plan" => Self::Plan,
            "disposition" => Self::Disposition,
            "external" => Self::External,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Self::Specification => "specification",
            Self::Implementation => "implementation",
            Self::Verification => "verification",
            Self::Documentation => "documentation",
            Self::Decision => "decision",
            Self::Research => "research",
            Self::Plan => "plan",
            Self::Disposition => "disposition",
            Self::External => "external",
        }
    }
}

#[derive(Debug)]
pub(super) struct ArtifactRequirements(Vec<ArtifactKind>);

impl ArtifactRequirements {
    pub(super) fn parse(raw: &str, at: usize) -> Result<Self, Violation> {
        if raw.is_empty() {
            return Err(Violation::at(
                at,
                "the `requires` set is non-empty; absence is expressed by omitting the attribute",
            ));
        }

        let mut kinds = Vec::new();
        for member in raw.split(',') {
            if member.is_empty() {
                return Err(Violation::at(
                    at,
                    "the `requires` set has an empty member (leading, trailing, or repeated comma)",
                ));
            }
            let Some(kind) = ArtifactKind::parse(member) else {
                return Err(Violation::at(
                    at,
                    format!(
                        "unknown required artifact kind `{member}`; expected one of {}",
                        Self::all_names()
                    ),
                ));
            };
            if kinds.contains(&kind) {
                return Err(Violation::at(
                    at,
                    format!("duplicate required artifact kind `{member}`"),
                ));
            }
            kinds.push(kind);
        }
        kinds.sort();
        Ok(Self(kinds))
    }

    pub(super) fn contains_external(&self) -> bool {
        self.0.contains(&ArtifactKind::External)
    }

    pub(super) fn canonical_csv(&self) -> String {
        self.0
            .iter()
            .map(|kind| kind.name())
            .collect::<Vec<_>>()
            .join(",")
    }

    fn all_names() -> String {
        CANONICAL_KINDS
            .iter()
            .map(|kind| kind.name())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_closed_kind_parses_and_authored_order_canonicalizes() {
        let reverse = CANONICAL_KINDS
            .iter()
            .rev()
            .map(|kind| kind.name())
            .collect::<Vec<_>>()
            .join(",");
        let parsed = ArtifactRequirements::parse(&reverse, 7)
            .unwrap_or_else(|error| panic!("closed set: {}", error.message));
        assert_eq!(
            parsed.canonical_csv(),
            CANONICAL_KINDS
                .iter()
                .map(|kind| kind.name())
                .collect::<Vec<_>>()
                .join(",")
        );
        assert!(parsed.contains_external());
    }

    #[test]
    fn empty_unknown_and_duplicate_members_are_refused() {
        for (raw, needle) in [
            ("", "non-empty"),
            (",specification", "empty member"),
            ("specification,", "empty member"),
            ("specification,,plan", "empty member"),
            ("specification,other", "unknown"),
            ("plan,plan", "duplicate"),
        ] {
            let err = ArtifactRequirements::parse(raw, 9).expect_err(raw);
            assert_eq!(err.line, 9);
            assert!(err.message.contains(needle), "{}: {}", raw, err.message);
        }
    }
}
