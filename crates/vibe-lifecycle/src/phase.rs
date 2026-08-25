//! The closed vocabulary and canonical order of default lifecycle phases.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#LIFECYCLES");

use std::{fmt, str::FromStr};

use specmark::spec;
use thiserror::Error;

const LIFECYCLES_SPEC_URI: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#LIFECYCLES";
const PHASE_CHOICES: &str =
    "validate, install, generate, build, test, create, verify, package, deploy";

/// One phase of the default lifecycle.
///
/// Its textual form is the exact lowercase command vocabulary. `clean` is
/// deliberately absent: it is a separate lifecycle represented by
/// [`crate::LifecycleStep::Clean`].
///
/// ```
/// use std::str::FromStr;
/// use vibe_lifecycle::Phase;
///
/// assert_eq!(Phase::from_str("generate"), Ok(Phase::Generate));
/// assert_eq!(Phase::Package.as_str(), "package");
/// assert_eq!(Phase::Deploy.to_string(), "deploy");
/// assert!(Phase::from_str("clean").is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Cheap preflight before work begins.
    Validate,
    /// Resolve and materialise the installed world.
    Install,
    /// Generate derived sources.
    Generate,
    /// Build deterministic artifacts.
    Build,
    /// Run deterministic tests.
    Test,
    /// Produce agentic outputs.
    Create,
    /// Gate build and create outputs.
    Verify,
    /// Assemble distributable artifacts.
    Package,
    /// Publish packaged artifacts.
    Deploy,
}

/// The canonical default lifecycle, from preflight through publication.
///
/// This array is the only ordering authority. Consumers must use its position,
/// rather than enum discriminants or an `Ord` implementation, when building a
/// phase chain.
///
/// ```
/// use vibe_lifecycle::{DEFAULT_PHASES, Phase};
///
/// assert_eq!(DEFAULT_PHASES.len(), 9);
/// assert_eq!(DEFAULT_PHASES[0], Phase::Validate);
/// assert_eq!(DEFAULT_PHASES[8], Phase::Deploy);
/// ```
pub const DEFAULT_PHASES: [Phase; 9] = [
    Phase::Validate,
    Phase::Install,
    Phase::Generate,
    Phase::Build,
    Phase::Test,
    Phase::Create,
    Phase::Verify,
    Phase::Package,
    Phase::Deploy,
];

impl Phase {
    /// Return the exact lowercase command spelling of this phase.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validate => "validate",
            Self::Install => "install",
            Self::Generate => "generate",
            Self::Build => "build",
            Self::Test => "test",
            Self::Create => "create",
            Self::Verify => "verify",
            Self::Package => "package",
            Self::Deploy => "deploy",
        }
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for Phase {
    type Err = PhaseParseError;

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#LIFECYCLES")]
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        DEFAULT_PHASES
            .into_iter()
            .find(|phase| phase.as_str() == input)
            .ok_or_else(|| PhaseParseError {
                input: input.to_owned(),
            })
    }
}

/// A value that is not one of the nine exact lowercase default phases.
///
/// The diagnostic retains the rejected input and names the normative spec plus
/// every valid choice, making it suitable for CLI and manifest adapters.
///
/// ```
/// use vibe_lifecycle::Phase;
///
/// let error = "BUILD".parse::<Phase>().expect_err("uppercase is not canonical");
/// assert_eq!(error.input(), "BUILD");
/// assert!(error.to_string().contains("validate, install, generate"));
/// assert!(error.to_string().contains("PROP-054#LIFECYCLES"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error(
    "unknown lifecycle phase `{input}`; expected one of: {PHASE_CHOICES} (see {LIFECYCLES_SPEC_URI})"
)]
pub struct PhaseParseError {
    input: String,
}

impl PhaseParseError {
    /// Return the exact value that failed to parse.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LIFECYCLES")]
    fn every_canonical_phase_round_trips() {
        let expected = [
            "validate", "install", "generate", "build", "test", "create", "verify", "package",
            "deploy",
        ];

        assert_eq!(DEFAULT_PHASES.map(Phase::as_str), expected);
        for (phase, spelling) in DEFAULT_PHASES.into_iter().zip(expected) {
            assert_eq!(phase.to_string(), spelling);
            assert_eq!(spelling.parse(), Ok(phase));
        }
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LIFECYCLES")]
    fn parser_is_exact_and_rejects_clean() {
        for invalid in ["", "clean", "BUILD", " build", "build ", "compile"] {
            let error = invalid.parse::<Phase>().expect_err("must reject");
            assert_eq!(error.input(), invalid);
        }
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LIFECYCLES")]
    fn parse_error_is_actionable_and_complete() {
        let message = "wat".parse::<Phase>().expect_err("must reject").to_string();

        assert!(message.contains("`wat`"));
        assert!(message.contains(LIFECYCLES_SPEC_URI));
        assert!(message.contains(PHASE_CHOICES));
    }
}
