//! Extension-point vocabulary for the lifecycle phase family.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR");

use std::{fmt, str::FromStr};

use specmark::spec;

use crate::{DEFAULT_PHASES, Phase};

mod compile;
mod slot;

pub use compile::{CompilePoint, CompilePointParseError};
pub use slot::{SlotPoint, SlotPointParseError};

use compile::COMPILE_POINTS;
use slot::SLOT_POINTS;

pub(super) const POINT_GRAMMAR_SPEC_URI: &str =
    "spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR";

/// One extension point in the `phase` family.
///
/// Clean remains separate from the nine-phase default lifecycle. This type is
/// intentionally narrower than the complete extension-point grammar: compiler
/// and slot points will receive their own domain types.
///
/// ```
/// use vibe_lifecycle::{Phase, PhasePoint};
///
/// assert_eq!("phase:clean".parse(), Ok(PhasePoint::Clean));
/// assert_eq!(
///     "phase:build".parse(),
///     Ok(PhasePoint::Default(Phase::Build)),
/// );
/// assert_eq!(PhasePoint::Default(Phase::Deploy).to_string(), "phase:deploy");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhasePoint {
    /// The independent clean lifecycle's extension point.
    Clean,
    /// An extension point belonging to the default lifecycle.
    Default(Phase),
}

impl fmt::Display for PhasePoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clean => formatter.write_str("phase:clean"),
            Self::Default(phase) => write!(formatter, "phase:{phase}"),
        }
    }
}

impl FromStr for PhasePoint {
    type Err = PhasePointParseError;

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let Some((family, name)) = input.split_once(':') else {
            return Err(PhasePointParseError::new(input));
        };

        if family != "phase" {
            return Err(PhasePointParseError::new(input));
        }

        if name == "clean" {
            return Ok(Self::Clean);
        }

        name.parse::<Phase>()
            .map(Self::Default)
            .map_err(|_| PhasePointParseError::new(input))
    }
}

/// A value that is not an exact lowercase point in the `phase` family.
///
/// The diagnostic retains the rejected input, lists every accepted spelling,
/// identifies this type's family boundary, and points to the normative grammar.
///
/// ```
/// use vibe_lifecycle::PhasePoint;
///
/// let error = "compile:source"
///     .parse::<PhasePoint>()
///     .expect_err("this parser accepts phase points only");
/// assert_eq!(error.input(), "compile:source");
/// assert!(error.to_string().contains("phase family only"));
/// assert!(error.to_string().contains("PROP-054#POINT-GRAMMAR"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhasePointParseError {
    input: String,
}

impl fmt::Display for PhasePointParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid phase extension point `{}`; this type accepts the phase family only; expected one of: ",
            self.input,
        )?;
        write_choices(formatter, phase_points())?;
        write!(formatter, " (see {POINT_GRAMMAR_SPEC_URI})")
    }
}

impl std::error::Error for PhasePointParseError {}

impl PhasePointParseError {
    fn new(input: &str) -> Self {
        Self {
            input: input.to_owned(),
        }
    }

    /// Return the exact value that failed to parse.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
}

/// One point in the complete closed extension-point vocabulary.
///
/// The family-specific types keep invalid combinations unrepresentable while
/// this sum type gives manifest and registry adapters one parser for every
/// legal point.
///
/// ```
/// use vibe_lifecycle::{CompilePoint, ExtensionPoint, Phase, PhasePoint, SlotPoint};
///
/// assert_eq!(
///     "phase:build".parse(),
///     Ok(ExtensionPoint::Phase(PhasePoint::Default(Phase::Build))),
/// );
/// assert_eq!(
///     "slot:pre-install".parse(),
///     Ok(ExtensionPoint::Slot(SlotPoint::PreInstall)),
/// );
/// assert_eq!(
///     "compile:pass".parse(),
///     Ok(ExtensionPoint::Compile(CompilePoint::Pass)),
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtensionPoint {
    /// A default- or clean-lifecycle phase point.
    Phase(PhasePoint),
    /// A package-slot materialisation point.
    Slot(SlotPoint),
    /// A staged compiler or pass-tier point.
    Compile(CompilePoint),
}

impl fmt::Display for ExtensionPoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Phase(point) => point.fmt(formatter),
            Self::Slot(point) => point.fmt(formatter),
            Self::Compile(point) => point.fmt(formatter),
        }
    }
}

impl FromStr for ExtensionPoint {
    type Err = ExtensionPointParseError;

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.split_once(':').map(|(family, _)| family) {
            Some("phase") => input
                .parse::<PhasePoint>()
                .map(Self::Phase)
                .map_err(|_| ExtensionPointParseError::new(input)),
            Some("slot") => input
                .parse::<SlotPoint>()
                .map(Self::Slot)
                .map_err(|_| ExtensionPointParseError::new(input)),
            Some("compile") => input
                .parse::<CompilePoint>()
                .map(Self::Compile)
                .map_err(|_| ExtensionPointParseError::new(input)),
            _ => Err(ExtensionPointParseError::new(input)),
        }
    }
}

/// A value outside the complete closed extension-point vocabulary.
///
/// The diagnostic retains the rejected input and derives its accepted list
/// from the same family tables as the parsers.
///
/// ```
/// use vibe_lifecycle::ExtensionPoint;
///
/// let error = "unknown:build"
///     .parse::<ExtensionPoint>()
///     .expect_err("unknown families are rejected");
/// assert_eq!(error.input(), "unknown:build");
/// assert!(error.to_string().contains("phase:clean"));
/// assert!(error.to_string().contains("slot:pre-install"));
/// assert!(error.to_string().contains("compile:pass"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionPointParseError {
    input: String,
}

impl fmt::Display for ExtensionPointParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid extension point `{}`; expected one of: ",
            self.input,
        )?;
        write_choices(
            formatter,
            phase_points()
                .map(ExtensionPoint::Phase)
                .chain(SLOT_POINTS.map(ExtensionPoint::Slot))
                .chain(COMPILE_POINTS.map(ExtensionPoint::Compile)),
        )?;
        write!(formatter, " (see {POINT_GRAMMAR_SPEC_URI})")
    }
}

impl std::error::Error for ExtensionPointParseError {}

impl ExtensionPointParseError {
    fn new(input: &str) -> Self {
        Self {
            input: input.to_owned(),
        }
    }

    /// Return the exact value that failed to parse.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
}

fn phase_points() -> impl Iterator<Item = PhasePoint> {
    std::iter::once(PhasePoint::Clean).chain(DEFAULT_PHASES.map(PhasePoint::Default))
}

pub(super) fn write_choices<T>(
    formatter: &mut fmt::Formatter<'_>,
    choices: impl IntoIterator<Item = T>,
) -> fmt::Result
where
    T: fmt::Display,
{
    let mut separator = "";
    for choice in choices {
        write!(formatter, "{separator}{choice}")?;
        separator = ", ";
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    const LEGAL_PHASE_POINTS: [(PhasePoint, &str); 10] = [
        (PhasePoint::Clean, "phase:clean"),
        (PhasePoint::Default(Phase::Validate), "phase:validate"),
        (PhasePoint::Default(Phase::Install), "phase:install"),
        (PhasePoint::Default(Phase::Generate), "phase:generate"),
        (PhasePoint::Default(Phase::Build), "phase:build"),
        (PhasePoint::Default(Phase::Test), "phase:test"),
        (PhasePoint::Default(Phase::Create), "phase:create"),
        (PhasePoint::Default(Phase::Verify), "phase:verify"),
        (PhasePoint::Default(Phase::Package), "phase:package"),
        (PhasePoint::Default(Phase::Deploy), "phase:deploy"),
    ];

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn all_ten_legal_spellings_parse_and_round_trip() {
        for (point, spelling) in LEGAL_PHASE_POINTS {
            assert_eq!(spelling.parse(), Ok(point));
            assert_eq!(point.to_string(), spelling);
        }
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn clean_stays_outside_the_default_phase_vocabulary() {
        assert_eq!("phase:clean".parse(), Ok(PhasePoint::Clean));
        assert!("clean".parse::<Phase>().is_err());
        assert_eq!(DEFAULT_PHASES.len(), 9);
        assert!(
            DEFAULT_PHASES
                .into_iter()
                .all(|phase| phase.as_str() != "clean")
        );
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn parser_rejects_noncanonical_and_out_of_family_inputs() {
        for invalid in [
            "",
            "build",
            "clean",
            "phase:",
            "phase:compile",
            "phase:BUILD",
            "phase: build",
            "phase:build ",
            " phase:build",
            "compile:source",
            "slot:pre-install",
            "unknown:build",
            "phase:build:extra",
        ] {
            let error = invalid.parse::<PhasePoint>().expect_err("must reject");
            assert_eq!(error.input(), invalid);
        }
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn invalid_diagnostic_is_actionable_and_points_to_the_exact_rule() {
        let message = "compile:source"
            .parse::<PhasePoint>()
            .expect_err("must reject")
            .to_string();

        assert!(message.contains("`compile:source`"));
        assert!(message.contains("phase family only"));
        let choices = std::iter::once("phase:clean".to_string())
            .chain(DEFAULT_PHASES.map(|phase| format!("phase:{phase}")))
            .collect::<Vec<_>>()
            .join(", ");
        assert!(message.contains(&choices));
        assert!(message.contains(POINT_GRAMMAR_SPEC_URI));
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn aggregate_parser_round_trips_all_seventeen_legal_points() {
        let points = phase_points()
            .map(ExtensionPoint::Phase)
            .chain(SLOT_POINTS.map(ExtensionPoint::Slot))
            .chain(COMPILE_POINTS.map(ExtensionPoint::Compile))
            .collect::<Vec<_>>();

        assert_eq!(points.len(), 17);
        for point in points {
            let spelling = point.to_string();
            assert_eq!(spelling.parse(), Ok(point));
        }
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn aggregate_parser_rejects_case_whitespace_family_and_colon_errors() {
        for invalid in [
            "",
            ":",
            "phase",
            "slot",
            "compile",
            "unknown:build",
            "PHASE:build",
            "phase:BUILD",
            " phase:build",
            "phase:build ",
            "phase::build",
            "phase:build:extra",
            "slot::pre-install",
            "compile:source:extra",
        ] {
            let error = invalid.parse::<ExtensionPoint>().expect_err("must reject");
            assert_eq!(error.input(), invalid);
        }
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn aggregate_error_lists_the_family_ssots_and_exact_rule() {
        let message = "unknown:build"
            .parse::<ExtensionPoint>()
            .expect_err("must reject")
            .to_string();
        let choices = phase_points()
            .map(ExtensionPoint::Phase)
            .chain(SLOT_POINTS.map(ExtensionPoint::Slot))
            .chain(COMPILE_POINTS.map(ExtensionPoint::Compile))
            .map(|point| point.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        assert!(message.contains("`unknown:build`"));
        assert!(message.contains(&choices));
        assert!(message.contains(POINT_GRAMMAR_SPEC_URI));
    }
}
