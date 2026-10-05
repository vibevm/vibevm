//! A selected Rust implementation with ordinary boundary-case tests.
//! Its tests and the separate Lean model do not establish refinement.

#![feature(register_tool)]
#![register_tool(specmark)]

/// Return the supplied unsigned integer unchanged.
///
/// ```
/// assert_eq!(rust::identity(7), 7);
/// ```
#[specmark::spec(implements = "spec://org.example/same-subject/Protocol#identity")]
pub fn identity(value: u64) -> u64 {
    value
}

#[cfg(test)]
mod tests {
    use super::identity;

    #[test]
    fn zero_is_unchanged() {
        assert_eq!(identity(0), 0);
    }

    #[test]
    fn maximum_is_unchanged() {
        assert_eq!(identity(u64::MAX), u64::MAX);
    }
}
