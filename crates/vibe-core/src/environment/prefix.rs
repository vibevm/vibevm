//! Declarative prefix checks, not relocation/native/evidence verification.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#PAYLOAD-PREFIX-COMPATIBILITY");
use super::*;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrefixCompatibility {
    DeclaredRelocatable {
        producer: Reference,
        evidence: Option<Reference>,
        conditions: Vec<Reference>,
    },
    PrefixBound {
        exact_target_prefix: AbsoluteTargetPath,
    },
    Unsupported {
        reason: Reference,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrefixAssessment {
    DeclarativeMatch,
    PrefixMismatch,
    NeedsEvidence,
    NeedsConditions(Vec<Reference>),
    UnresolvedComparison,
    Unsupported(Reference),
}
impl PrefixCompatibility {
    /// Supplied condition/evidence references remain unverified claims. There
    /// is no binary rewrite or effect admission hidden in a declarative match.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#PAYLOAD-PREFIX-COMPATIBILITY",
        r = 1
    )]
    pub fn assess(
        &self,
        source: &AbsoluteTargetPath,
        destination: &AbsoluteTargetPath,
        comparison: CaseComparison,
        supplied_conditions: &[Reference],
    ) -> PrefixAssessment {
        match self {
            Self::Unsupported { reason } => PrefixAssessment::Unsupported(reason.clone()),
            Self::PrefixBound {
                exact_target_prefix,
            } => {
                let a = source.lexical_relation(exact_target_prefix, comparison);
                let b = destination.lexical_relation(exact_target_prefix, comparison);
                if a == LexicalRelation::Unresolved || b == LexicalRelation::Unresolved {
                    PrefixAssessment::UnresolvedComparison
                } else if a == LexicalRelation::Equal && b == LexicalRelation::Equal {
                    PrefixAssessment::DeclarativeMatch
                } else {
                    PrefixAssessment::PrefixMismatch
                }
            }
            Self::DeclaredRelocatable {
                evidence,
                conditions,
                ..
            } => {
                if evidence.is_none() {
                    return PrefixAssessment::NeedsEvidence;
                }
                // Unknown comparison cannot support a changed-prefix claim,
                // including nonASCII paths under the ASCII-only lexical model.
                if source.lexical_relation(destination, comparison) == LexicalRelation::Unresolved {
                    return PrefixAssessment::UnresolvedComparison;
                }
                let missing = conditions
                    .iter()
                    .filter(|c| !supplied_conditions.contains(c))
                    .cloned()
                    .collect::<Vec<_>>();
                if missing.is_empty() {
                    PrefixAssessment::DeclarativeMatch
                } else {
                    PrefixAssessment::NeedsConditions(missing)
                }
            }
        }
    }
    pub fn evidence_verification(&self) -> ClaimVerification {
        ClaimVerification::Unverified
    }
}
