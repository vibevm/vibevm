//! Checked conversion for counts whose wire contract is JTD `uint32`.
//!
//! The in-memory domain stays `usize`; narrowing happens exactly once,
//! at the JSON-envelope boundary, and refuses instead of truncating.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CountOverflow {
    pub(crate) field: &'static str,
    pub(crate) value: usize,
}

impl fmt::Display for CountOverflow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "wire field `{}` value {} exceeds uint32 (violates \
             spec://org.vibevm.core/vibevm/common/PROP-044#machinery; \
             fix: reduce the result/page size or widen the field's schema and writer together)",
            self.field, self.value
        )
    }
}

pub(crate) fn checked_u32(field: &'static str, value: usize) -> Result<u32, CountOverflow> {
    u32::try_from(value).map_err(|_| CountOverflow { field, value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uint32_boundary_is_exact() {
        assert_eq!(checked_u32("hit_count", u32::MAX as usize), Ok(u32::MAX));
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn uint32_overflow_is_an_error_not_truncation() {
        let value = u32::MAX as usize + 1;
        let error = checked_u32("hit_count", value).expect_err("must refuse overflow");
        assert_eq!(
            error,
            CountOverflow {
                field: "hit_count",
                value
            }
        );
        assert!(error.to_string().contains("exceeds uint32"));
    }
}
