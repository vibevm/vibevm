//! Text normalisation — the one core utility the cells share.

specmark::scope!("spec://rust-demo/PROP-001#cell-text");

use specmark::spec;

/// Collapse internal whitespace runs to single spaces and trim the
/// ends.
///
/// ```
/// use rust_demo::core::text::normalise;
/// assert_eq!(normalise("  Ada   Lovelace "), "Ada Lovelace");
/// assert_eq!(normalise("\t\n"), "");
/// ```
#[spec(implements = "spec://rust-demo/PROP-001#cell-text")]
pub fn normalise(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::normalise;

    #[test]
    fn collapses_runs_and_trims() {
        assert_eq!(normalise("  a \t b\n\nc "), "a b c");
    }

    #[test]
    fn empty_and_blank_normalise_to_empty() {
        assert_eq!(normalise(""), "");
        assert_eq!(normalise("   "), "");
    }
}
