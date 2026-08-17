//! CLI-side re-export of the canonical `kinds` types, plus the
//! argument-boundary parse the open vocabulary needs.
//!
//! The actual definitions live in [`crate::types::kinds`] (a re-export
//! of the generated wire vocabulary). CLI subcommand modules still
//! reach for them through `crate::cli::kinds::*` for path-stability.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::str::FromStr;

use crate::error::{Error, Result};

pub use crate::types::kinds::{NamingConvention, PackageKind};

/// Parse a `--kind` flag value into the wire vocabulary. The parse
/// itself is total — an open vocabulary accepts every string, which
/// is exactly what the wire reader does — so the REFUSAL lives here,
/// at the argument boundary: a kind this build does not know gets a
/// message naming it and the known set, never a filter that answers
/// zero rows in silence. Unfamiliar on the wire is normal life;
/// unfamiliar in an argument is a user to tell (Б.1).
pub fn parse_kind_flag(value: &str) -> Result<PackageKind> {
    let kind = match PackageKind::from_str(value) {
        Ok(kind) => kind,
        Err(uninhabited) => match uninhabited {},
    };
    if let PackageKind::Unknown(_) = &kind {
        let known: Vec<&str> = PackageKind::known().iter().map(|k| k.as_str()).collect();
        return Err(Error::InvalidInput(format!(
            "package kind `{value}` is unknown to this build — known kinds: {}",
            known.join(", ")
        )));
    }
    Ok(kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_kind_parses_to_its_variant() {
        assert_eq!(parse_kind_flag("flow").unwrap(), PackageKind::Flow);
        assert_eq!(parse_kind_flag("lang").unwrap(), PackageKind::Lang);
    }

    #[test]
    fn unfamiliar_kind_speaks_instead_of_filtering() {
        // Б.1 — the argument must SAY the kind is unknown to this
        // build; a total parse that flowed into the filter would
        // answer zero rows and say nothing.
        let err = parse_kind_flag("plugin").unwrap_err().to_string();
        assert!(err.contains("unknown to this build"), "{err}");
        assert!(err.contains("plugin"), "{err}");
        assert!(err.contains("flow"), "{err}");
    }
}
