//! The reader-capability registry (PROP-044 §4.5): what THIS build can
//! honour when a record declares `must_understand`.
//!
//! A record's `must_understand` names capabilities a reader must
//! understand to act on it; a reader that lacks one must quarantine the
//! record, and the refusal surfaces at the point of application. The
//! predicate lives HERE — the lowest crate every reader sees — so every
//! reader (the index's answer path, the registry client's version
//! selector) asks the same list and the copies cannot drift. Before
//! this module the predicate lived in `vibe-index` alone and the
//! registry-side reader was capability-blind by construction (B-080).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#machinery");

/// The reader capabilities this build understands.
///
/// Empty today: no capability has been built yet, so any non-empty
/// `must_understand` names something this reader cannot honour. The
/// list grows as capabilities land. This is the single home —
/// `vibe-index`'s quarantine module re-exports it rather than keeping
/// a second copy.
///
/// NOT the same vocabulary as a package's `provides.capabilities` —
/// these are capabilities of the READER (PROP-044 §4.5), not of the
/// package.
pub const UNDERSTOOD: &[&str] = &[];

/// Does this build understand the named reader capability?
pub fn reader_understands(cap: &str) -> bool {
    UNDERSTOOD.contains(&cap)
}

/// The capabilities of `must_understand` this build does not
/// understand. Empty result = the record may be acted on.
pub fn missing_capabilities(must_understand: &[String]) -> Vec<String> {
    must_understand
        .iter()
        .filter(|cap| !reader_understands(cap))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_must_understand_needs_nothing() {
        assert!(missing_capabilities(&[]).is_empty());
    }

    #[test]
    fn no_capability_is_understood_yet() {
        // The list is empty by measurement (B-080): every named
        // capability today is one this build lacks. The moment a real
        // capability lands, this test flips with it — and that is the
        // point of stating it.
        assert!(UNDERSTOOD.is_empty());
        assert!(!reader_understands("b080-test-capability"));
        assert!(!reader_understands("org.vibevm/wal/tombstone@1"));
    }

    #[test]
    fn missing_capabilities_reports_every_unknown_in_order() {
        let caps: Vec<String> = vec!["x".into(), "y".into()];
        assert_eq!(
            missing_capabilities(&caps),
            vec!["x".to_string(), "y".to_string()]
        );
    }
}
