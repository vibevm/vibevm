//! The reader's side of `must_understand` (PROP-044 §4.5): what this
//! build can honour, and the quarantine record it keeps of catalog
//! entries it refused to act on.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#entry");

use semver::Version;
use vibe_core::Group;

/// The reader capabilities this build understands.
///
/// Empty today: no capability has been built yet, so any non-empty
/// `must_understand` names something this reader cannot honour. The list
/// grows as capabilities land.
///
/// NOT the same vocabulary as a package's `provides.capabilities` — these are
/// capabilities of the READER (PROP-044 §4.5), not of the package.
pub const UNDERSTOOD: &[&str] = &[];

/// A catalog record the reader refused to act on, and why.
///
/// Lives in memory only — never written to any catalog file.
#[derive(Debug, Clone)]
pub struct Quarantined {
    pub group: Group,
    pub name: String,
    pub version: Version,
    pub missing: Vec<String>,
}

/// The capabilities of `must_understand` this build does not understand.
/// Empty result = the record may be acted on.
pub fn missing_capabilities(must_understand: &[String]) -> Vec<String> {
    must_understand
        .iter()
        .filter(|cap| !UNDERSTOOD.contains(&cap.as_str()))
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
    fn unknown_capability_is_reported_missing() {
        let caps: Vec<String> = vec!["x".into()];
        assert_eq!(missing_capabilities(&caps), vec!["x".to_string()]);
    }
}
