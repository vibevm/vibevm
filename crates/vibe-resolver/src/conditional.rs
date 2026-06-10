//! Conditional-dependency predicate evaluation — PROP-003 §2.6.1.
//!
//! Manifest entries `[target."context(<key>)".dependencies]` carry a
//! predicate string and a [`Requires`]-shape body. This module parses
//! the predicate and evaluates it against an [`ActivationContext`]
//! built from the resolved graph + project state.
//!
//! Today's grammar supports the simple-key form only: `context(<key>)`
//! where `<key>` is a `<kind>:<name>` pkgref/capability/interface tag
//! that probes `ctx.present` (and `ctx.provides` if it starts with
//! `interface:`). Richer probe forms — `if_files`, boolean
//! composition (`and`, `or`, `not`) — are reserved for follow-up
//! slices and surface here as `PredicateError::Unsupported` so the
//! manifest parses but the unmatched runtime form is loud.

use specmark::spec;
use thiserror::Error;

use crate::ActivationContext;

/// Parsed conditional-dep predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(
    implements = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar",
    r = 2
)]
pub enum ConditionalPredicate {
    /// `context(<key>)` — matches if `<key>` is in `ctx.present` (or
    /// `ctx.provides` for `interface:` tags).
    Present(String),
}

impl ConditionalPredicate {
    /// Parse a predicate string from the TOML key. Accepts:
    /// - `context(<key>)` exact match
    /// - leading / trailing whitespace
    #[spec(
        implements = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar",
        r = 2
    )]
    #[spec(
        deviates = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-composition",
        r = 1,
        reason = "boolean composition (`and`/`or`/`not`) intentionally unimplemented; \
                  every composition form surfaces as PredicateError::Unsupported, \
                  pending the PROP-014 pilot decision"
    )]
    pub fn parse(raw: &str) -> Result<Self, PredicateError> {
        let s = raw.trim();
        let inner = s
            .strip_prefix("context(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| PredicateError::Malformed(raw.to_string()))?
            .trim();
        if inner.is_empty() {
            return Err(PredicateError::Malformed(raw.to_string()));
        }
        // Future probe forms — `if_files = '...'`, boolean composition —
        // surface as `Unsupported` until we plumb them through.
        if inner.contains('=') || inner.contains(" and ") || inner.contains(" or ") {
            return Err(PredicateError::Unsupported(raw.to_string()));
        }
        Ok(ConditionalPredicate::Present(inner.to_string()))
    }

    /// Evaluate against an activation context. Returns `true` if the
    /// predicate matches.
    #[spec(
        implements = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-host-invariance",
        r = 1
    )]
    pub fn evaluate(&self, ctx: &ActivationContext) -> bool {
        match self {
            ConditionalPredicate::Present(key) => {
                ctx.present.contains(key) || ctx.provides.contains(key)
            }
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PredicateError {
    #[error("malformed conditional-dep predicate `{0}` (expected `context(<key>)`)")]
    Malformed(String),

    #[error(
        "conditional-dep predicate `{0}` uses an unsupported form. Today only `context(<key>)` (capability/pkgref/interface tag) is recognised."
    )]
    Unsupported(String),
}

#[cfg(test)]
mod tests {
    use specmark::verifies;

    use super::*;

    #[test]
    #[verifies(
        "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar",
        r = 2
    )]
    fn parses_simple_present_predicate() {
        let p = ConditionalPredicate::parse("context(stack:rust)").unwrap();
        assert_eq!(p, ConditionalPredicate::Present("stack:rust".into()));
    }

    #[test]
    #[verifies(
        "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar",
        r = 2
    )]
    fn parses_with_whitespace() {
        let p = ConditionalPredicate::parse("  context( interface:foo )  ").unwrap();
        assert_eq!(p, ConditionalPredicate::Present("interface:foo".into()));
    }

    #[test]
    #[verifies(
        "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar",
        r = 2
    )]
    fn rejects_malformed() {
        assert!(matches!(
            ConditionalPredicate::parse("stack:rust"),
            Err(PredicateError::Malformed(_))
        ));
        assert!(matches!(
            ConditionalPredicate::parse("context()"),
            Err(PredicateError::Malformed(_))
        ));
    }

    #[test]
    #[verifies(
        "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar",
        r = 2
    )]
    fn flags_unsupported_richer_forms() {
        assert!(matches!(
            ConditionalPredicate::parse("context(if_files = '**/Cargo.toml')"),
            Err(PredicateError::Unsupported(_))
        ));
        assert!(matches!(
            ConditionalPredicate::parse("context(stack:rust and interface:foo)"),
            Err(PredicateError::Unsupported(_))
        ));
    }

    #[test]
    #[verifies(
        "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-host-invariance",
        r = 1
    )]
    fn evaluates_against_present() {
        let p = ConditionalPredicate::Present("stack:rust".into());
        let mut ctx = ActivationContext::default();
        assert!(!p.evaluate(&ctx));
        ctx.add_present("stack:rust");
        assert!(p.evaluate(&ctx));
    }

    #[test]
    #[verifies(
        "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-host-invariance",
        r = 1
    )]
    fn evaluates_against_provides() {
        let p = ConditionalPredicate::Present("interface:build-system".into());
        let mut ctx = ActivationContext::default();
        assert!(!p.evaluate(&ctx));
        ctx.add_provides("interface:build-system");
        assert!(p.evaluate(&ctx));
    }
}
