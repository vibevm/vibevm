//! Extension-point vocabulary for package-slot materialisation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR");

use std::{fmt, str::FromStr};

use specmark::spec;

use super::{POINT_GRAMMAR_SPEC_URI, write_choices};

/// One extension point in the `slot` family.
///
/// ```
/// use vibe_core::lifecycle::SlotPoint;
///
/// assert_eq!("slot:pre-install".parse(), Ok(SlotPoint::PreInstall));
/// assert_eq!(
///     "slot:post-install".parse(),
///     Ok(SlotPoint::PostInstall),
/// );
/// assert_eq!(SlotPoint::PostInstall.to_string(), "slot:post-install");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotPoint {
    /// After slot population and before vibe consumes the package.
    PreInstall,
    /// After the package install becomes durable.
    PostInstall,
}

impl SlotPoint {
    const fn as_str(self) -> &'static str {
        match self {
            Self::PreInstall => "slot:pre-install",
            Self::PostInstall => "slot:post-install",
        }
    }
}

pub(super) const SLOT_POINTS: [SlotPoint; 2] = [SlotPoint::PreInstall, SlotPoint::PostInstall];

impl fmt::Display for SlotPoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for SlotPoint {
    type Err = SlotPointParseError;

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        SLOT_POINTS
            .into_iter()
            .find(|point| point.as_str() == input)
            .ok_or_else(|| SlotPointParseError::new(input))
    }
}

/// A value that is not an exact lowercase point in the `slot` family.
///
/// ```
/// use vibe_core::lifecycle::SlotPoint;
///
/// let error = "phase:install"
///     .parse::<SlotPoint>()
///     .expect_err("this parser accepts slot points only");
/// assert_eq!(error.input(), "phase:install");
/// assert!(error.to_string().contains("slot:pre-install, slot:post-install"));
/// assert!(error.to_string().contains("PROP-054#POINT-GRAMMAR"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotPointParseError {
    input: String,
}

impl fmt::Display for SlotPointParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid slot extension point `{}`; this type accepts the slot family only; expected one of: ",
            self.input,
        )?;
        write_choices(formatter, SLOT_POINTS)?;
        write!(formatter, " (see {POINT_GRAMMAR_SPEC_URI})")
    }
}

impl std::error::Error for SlotPointParseError {}

impl SlotPointParseError {
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
    fn both_slot_points_parse_and_round_trip() {
        let expected = ["slot:pre-install", "slot:post-install"];

        assert_eq!(SLOT_POINTS.map(SlotPoint::as_str), expected);
        for (point, spelling) in SLOT_POINTS.into_iter().zip(expected) {
            assert_eq!(spelling.parse(), Ok(point));
            assert_eq!(point.to_string(), spelling);
        }
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn slot_parser_rejects_case_whitespace_family_and_colon_errors() {
        for invalid in [
            "",
            "pre-install",
            "slot:",
            "slot:PRE-INSTALL",
            " slot:pre-install",
            "slot:pre-install ",
            "slot::pre-install",
            "slot:pre-install:extra",
            "phase:install",
            "compile:source",
        ] {
            let error = invalid.parse::<SlotPoint>().expect_err("must reject");
            assert_eq!(error.input(), invalid);
        }
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR")]
    fn slot_error_lists_the_slot_ssot_and_exact_rule() {
        let message = "phase:install"
            .parse::<SlotPoint>()
            .expect_err("must reject")
            .to_string();
        let choices = SLOT_POINTS.map(|point| point.to_string()).join(", ");

        assert!(message.contains("`phase:install`"));
        assert!(message.contains(&choices));
        assert!(message.contains(POINT_GRAMMAR_SPEC_URI));
    }
}
