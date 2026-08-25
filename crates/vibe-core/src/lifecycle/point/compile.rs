//! Extension-point vocabulary for staged compiler transforms and passes.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR");

use std::{fmt, str::FromStr};

use specmark::spec;

use super::{POINT_GRAMMAR_SPEC_URI, write_choices};

/// One extension point in the `compile` family.
///
/// ```
/// use vibe_core::lifecycle::CompilePoint;
///
/// assert_eq!("compile:source".parse(), Ok(CompilePoint::Source));
/// assert_eq!("compile:emitted".parse(), Ok(CompilePoint::Emitted));
/// assert_eq!("compile:pass".parse(), Ok(CompilePoint::Pass));
/// assert_eq!(CompilePoint::Document.to_string(), "compile:document");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilePoint {
    /// Raw document text before parsing.
    Source,
    /// One parsed document before closure construction.
    Document,
    /// The assembled lane before emission.
    Lane,
    /// Final emitted bytes.
    Emitted,
    /// The full pass tier; pass placement is declared separately.
    Pass,
}

impl CompilePoint {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "compile:source",
            Self::Document => "compile:document",
            Self::Lane => "compile:lane",
            Self::Emitted => "compile:emitted",
            Self::Pass => "compile:pass",
        }
    }
}

pub(super) const COMPILE_POINTS: [CompilePoint; 5] = [
    CompilePoint::Source,
    CompilePoint::Document,
    CompilePoint::Lane,
    CompilePoint::Emitted,
    CompilePoint::Pass,
];

impl fmt::Display for CompilePoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for CompilePoint {
    type Err = CompilePointParseError;

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        COMPILE_POINTS
            .into_iter()
            .find(|point| point.as_str() == input)
            .ok_or_else(|| CompilePointParseError::new(input))
    }
}

/// A value that is not an exact lowercase point in the `compile` family.
///
/// ```
/// use vibe_core::lifecycle::CompilePoint;
///
/// let error = "slot:pre-install"
///     .parse::<CompilePoint>()
///     .expect_err("this parser accepts compile points only");
/// assert_eq!(error.input(), "slot:pre-install");
/// assert!(error.to_string().contains("compile:source"));
/// assert!(error.to_string().contains("compile:pass"));
/// assert!(error.to_string().contains("PROP-054#POINT-GRAMMAR"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilePointParseError {
    input: String,
}

impl fmt::Display for CompilePointParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid compile extension point `{}`; this type accepts the compile family only; expected one of: ",
            self.input,
        )?;
        write_choices(formatter, COMPILE_POINTS)?;
        write!(formatter, " (see {POINT_GRAMMAR_SPEC_URI})")
    }
}

impl std::error::Error for CompilePointParseError {}

impl CompilePointParseError {
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

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn all_five_compile_points_parse_and_round_trip() {
        let expected = [
            "compile:source",
            "compile:document",
            "compile:lane",
            "compile:emitted",
            "compile:pass",
        ];

        assert_eq!(COMPILE_POINTS.map(CompilePoint::as_str), expected);
        for (point, spelling) in COMPILE_POINTS.into_iter().zip(expected) {
            assert_eq!(spelling.parse(), Ok(point));
            assert_eq!(point.to_string(), spelling);
        }
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn compile_parser_rejects_case_whitespace_family_and_colon_errors() {
        for invalid in [
            "",
            "source",
            "compile:",
            "compile:SOURCE",
            " compile:source",
            "compile:source ",
            "compile::source",
            "compile:source:extra",
            "phase:build",
            "slot:pre-install",
        ] {
            let error = invalid.parse::<CompilePoint>().expect_err("must reject");
            assert_eq!(error.input(), invalid);
        }
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn compile_error_lists_the_compile_ssot_and_exact_rule() {
        let message = "slot:pre-install"
            .parse::<CompilePoint>()
            .expect_err("must reject")
            .to_string();
        let choices = COMPILE_POINTS.map(|point| point.to_string()).join(", ");

        assert!(message.contains("`slot:pre-install`"));
        assert!(message.contains(&choices));
        assert!(message.contains(POINT_GRAMMAR_SPEC_URI));
    }
}
