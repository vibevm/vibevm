//! Conditional-dependency predicate evaluation — PROP-003 §2.6.1.
//!
//! Manifest entries `[target."context(<key>)".dependencies]` carry a
//! predicate string and a [`Requires`]-shape body. This module parses
//! the predicate and evaluates it against an [`ActivationContext`]
//! built from the resolved graph + project state.
//!
//! The grammar inside `context(...)`: keys compose with `and` / `or` /
//! `not`, parentheses group, precedence is `not` > `and` > `or`
//! (PROP-003 §2.6 composition, r2). A bare `<key>` probes
//! `ctx.present` (and `ctx.provides`). The richer §2.5.2 probe forms
//! (`if_files = '…'` inside `context(...)`) remain unimplemented and
//! surface as `PredicateError::Unsupported` so the manifest parses
//! but the unmatched runtime form is loud.

use specmark::spec;
use thiserror::Error;

use crate::ActivationContext;

/// Parsed conditional-dep predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(
    implements = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar",
    r = 1
)]
#[spec(
    implements = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-composition",
    r = 2
)]
pub enum ConditionalPredicate {
    /// `<key>` — matches if the key is in `ctx.present` (or
    /// `ctx.provides` for `interface:` tags).
    Present(String),
    /// `a and b [and c …]` — every operand matches.
    And(Vec<ConditionalPredicate>),
    /// `a or b [or c …]` — at least one operand matches.
    Or(Vec<ConditionalPredicate>),
    /// `not a` — the operand does not match.
    Not(Box<ConditionalPredicate>),
}

impl ConditionalPredicate {
    /// Parse a predicate string from the TOML key. Accepts
    /// `context(<expr>)` where `<expr>` is the boolean-composition
    /// grammar over context keys; leading / trailing whitespace is
    /// tolerated everywhere.
    #[spec(
        implements = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar",
        r = 1
    )]
    #[spec(
        deviates = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar",
        r = 1,
        reason = "the grammar unit names the full §2.5.2 probe set inside context(...); \
                  only present/provides keys (+ composition) are implemented — the \
                  `if_files = …` probe forms surface as PredicateError::Unsupported"
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
        // Probe forms (`if_files = '…'`) are not keys; loud, not wrong.
        if inner.contains('=') {
            return Err(PredicateError::Unsupported(raw.to_string()));
        }
        let tokens = tokenize(inner, raw)?;
        let mut parser = Parser {
            tokens: &tokens,
            pos: 0,
            raw,
        };
        let expr = parser.or_expr()?;
        if parser.pos != tokens.len() {
            return Err(PredicateError::Malformed(raw.to_string()));
        }
        Ok(expr)
    }

    /// Evaluate against an activation context. Returns `true` if the
    /// predicate matches. Pure over `(self, ctx)` — host state never
    /// enters, which is what keeps lockfiles host-invariant.
    #[spec(
        implements = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-host-invariance",
        r = 1
    )]
    pub fn evaluate(&self, ctx: &ActivationContext) -> bool {
        match self {
            ConditionalPredicate::Present(key) => {
                ctx.present.contains(key.as_str()) || ctx.provides.contains(key.as_str())
            }
            ConditionalPredicate::And(ops) => ops.iter().all(|p| p.evaluate(ctx)),
            ConditionalPredicate::Or(ops) => ops.iter().any(|p| p.evaluate(ctx)),
            ConditionalPredicate::Not(op) => !op.evaluate(ctx),
        }
    }
}

#[derive(Debug, PartialEq)]
enum Token {
    Open,
    Close,
    And,
    Or,
    Not,
    Key(String),
}

/// Split the expression into tokens: parentheses, the three operators
/// (word-matched, so a key like `org.vibevm/android` is never split),
/// and keys (any other whitespace-delimited word).
fn tokenize(inner: &str, raw: &str) -> Result<Vec<Token>, PredicateError> {
    let spaced = inner.replace('(', " ( ").replace(')', " ) ");
    let mut out = Vec::new();
    for word in spaced.split_whitespace() {
        out.push(match word {
            "(" => Token::Open,
            ")" => Token::Close,
            "and" => Token::And,
            "or" => Token::Or,
            "not" => Token::Not,
            key => Token::Key(key.to_string()),
        });
    }
    if out.is_empty() {
        return Err(PredicateError::Malformed(raw.to_string()));
    }
    Ok(out)
}

/// Recursive-descent over the token list:
/// `or := and (\"or\" and)*` · `and := unary (\"and\" unary)*` ·
/// `unary := \"not\" unary | \"(\" or \")\" | key`.
struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    raw: &'a str,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn or_expr(&mut self) -> Result<ConditionalPredicate, PredicateError> {
        // The single-operand case returns the operand directly — no vec,
        // no "len == 1 so pop succeeds" reasoning to carry.
        let first = self.and_expr()?;
        if !matches!(self.peek(), Some(Token::Or)) {
            return Ok(first);
        }
        let mut ops = vec![first];
        while matches!(self.peek(), Some(Token::Or)) {
            self.pos += 1;
            ops.push(self.and_expr()?);
        }
        Ok(ConditionalPredicate::Or(ops))
    }

    fn and_expr(&mut self) -> Result<ConditionalPredicate, PredicateError> {
        let first = self.unary()?;
        if !matches!(self.peek(), Some(Token::And)) {
            return Ok(first);
        }
        let mut ops = vec![first];
        while matches!(self.peek(), Some(Token::And)) {
            self.pos += 1;
            ops.push(self.unary()?);
        }
        Ok(ConditionalPredicate::And(ops))
    }

    fn unary(&mut self) -> Result<ConditionalPredicate, PredicateError> {
        match self.peek() {
            Some(Token::Not) => {
                self.pos += 1;
                Ok(ConditionalPredicate::Not(Box::new(self.unary()?)))
            }
            Some(Token::Open) => {
                self.pos += 1;
                let inner = self.or_expr()?;
                if !matches!(self.peek(), Some(Token::Close)) {
                    return Err(PredicateError::Malformed(self.raw.to_string()));
                }
                self.pos += 1;
                Ok(inner)
            }
            Some(Token::Key(_)) => {
                let Some(Token::Key(k)) = self.tokens.get(self.pos) else {
                    unreachable!("peeked a key")
                };
                self.pos += 1;
                Ok(ConditionalPredicate::Present(k.clone()))
            }
            _ => Err(PredicateError::Malformed(self.raw.to_string())),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[spec(
    implements = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar",
    r = 1
)]
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
    use crate::CapabilityTag;

    #[test]
    #[verifies(
        "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar",
        r = 1
    )]
    fn parses_simple_present_predicate() {
        let p = ConditionalPredicate::parse("context(stack:rust)").unwrap();
        assert_eq!(p, ConditionalPredicate::Present("stack:rust".into()));
    }

    #[test]
    #[verifies(
        "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar",
        r = 1
    )]
    fn parses_with_whitespace() {
        let p = ConditionalPredicate::parse("  context( interface:foo )  ").unwrap();
        assert_eq!(p, ConditionalPredicate::Present("interface:foo".into()));
    }

    #[test]
    #[verifies(
        "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar",
        r = 1
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
        r = 1
    )]
    fn flags_unsupported_probe_forms() {
        assert!(matches!(
            ConditionalPredicate::parse("context(if_files = '**/Cargo.toml')"),
            Err(PredicateError::Unsupported(_))
        ));
    }

    #[test]
    #[verifies(
        "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-composition",
        r = 2
    )]
    fn parses_composition_with_precedence() {
        use ConditionalPredicate as P;
        // `not` binds tightest, then `and`, then `or`.
        let p = P::parse("context(a:x or b:y and not c:z)").unwrap();
        assert_eq!(
            p,
            P::Or(vec![
                P::Present("a:x".into()),
                P::And(vec![
                    P::Present("b:y".into()),
                    P::Not(Box::new(P::Present("c:z".into()))),
                ]),
            ])
        );
        // Parentheses regroup.
        let p = P::parse("context((a:x or b:y) and c:z)").unwrap();
        assert_eq!(
            p,
            P::And(vec![
                P::Or(vec![P::Present("a:x".into()), P::Present("b:y".into())]),
                P::Present("c:z".into()),
            ])
        );
    }

    #[test]
    #[verifies(
        "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-composition",
        r = 2
    )]
    fn evaluates_composition() {
        let mut ctx = ActivationContext::default();
        ctx.add_present(CapabilityTag::parse("stack:rust").unwrap());
        let has = |s: &str| ConditionalPredicate::parse(s).unwrap().evaluate(&ctx);
        assert!(has("context(stack:rust and not stack:go)"));
        assert!(!has("context(stack:rust and stack:go)"));
        assert!(has("context(stack:go or stack:rust)"));
        assert!(!has("context(not stack:rust)"));
        assert!(has("context(not (stack:go and stack:rust))"));
    }

    #[test]
    #[verifies(
        "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-composition",
        r = 2
    )]
    fn rejects_malformed_composition() {
        for bad in [
            "context(and)",
            "context(a:x and)",
            "context(or a:x)",
            "context((a:x)",
            "context(a:x))",
            "context(not)",
        ] {
            assert!(
                matches!(
                    ConditionalPredicate::parse(bad),
                    Err(PredicateError::Malformed(_))
                ),
                "`{bad}` must be malformed"
            );
        }
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
        ctx.add_present(CapabilityTag::parse("stack:rust").unwrap());
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
        ctx.add_provides(CapabilityTag::parse("interface:build-system").unwrap());
        assert!(p.evaluate(&ctx));
    }
}
