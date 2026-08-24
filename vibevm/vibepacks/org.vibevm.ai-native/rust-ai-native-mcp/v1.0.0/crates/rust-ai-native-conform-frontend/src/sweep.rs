//! The swept-test-matrix detection (R-060) — the loop-walk helpers for
//! the rust-syn frontend, kept out-of-line under the file-length budget.
//! The [`syn::visit::Visit`] loop methods themselves stay in `lib.rs`
//! (a trait impl cannot be split across files); this module holds the
//! shared bookkeeping and the bit-mask bound check they call.

specmark::scope!(
    "spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#frontends"
);

use conform_core::Fact;
use quote::ToTokens;

use crate::Extractor;

impl Extractor {
    /// Bookkeeping on entry to a GENERATED-axis loop (a `for` over a range):
    /// bump the range-loop depth and, at the ≥ 3 threshold inside a test,
    /// emit the `nested-loops` swept-matrix fact. Emitted at the threshold
    /// crossing (depth 3), so a single deep nest reports once per 3rd-level
    /// loop, never per inner iteration. A DECLARED-axis loop (a `for` over a
    /// collection/array/path) never calls this — see [`is_range_iterable`].
    pub(crate) fn enter_loop(&mut self, line: u32) {
        self.range_loop_depth += 1;
        if self.range_loop_depth >= 3 && self.test_depth > 0 {
            self.facts.push(Fact::TestSweep {
                kind: "nested-loops".into(),
                line,
                detail: self.range_loop_depth.to_string(),
            });
        }
    }
}

/// Is this `for`-loop iterable a generated numeric RANGE — the only form
/// that counts toward the swept-matrix depth (R-060)? A `for x in 0..n` /
/// `0..=n` / `a..b` iterable is a generated axis (its bound is computed, not
/// written as data); a collection iterable (`[a, b]`, `Stage::ALL`,
/// `vec.iter()`) is a DECLARED axis and returns false. Parens/groups unwrap
/// to the range beneath, so `(0..n)` counts.
///
/// **Recorded limit:** a range wrapped in an adapter (`(0..n).step_by(2)`) is
/// syntactically a method-call iterable, not an `ExprRange`, so it reads as
/// declared and does not count — it under-counts a genuinely generated axis.
/// `while`/`loop` carry no iterable at all: a numeric `while i < n` IS a
/// generated axis, but the heuristic cannot tell it from a data-driven
/// `while let Some(x) = iter.next()`, so neither counts (the Visit overrides
/// for them are gone). Both err toward silence — no false positive — the safe
/// direction for a narrowing.
pub(crate) fn is_range_iterable(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Range(_) => true,
        syn::Expr::Group(g) => is_range_iterable(&g.expr),
        syn::Expr::Paren(p) => is_range_iterable(&p.expr),
        _ => false,
    }
}

/// The `2^n` bit-mask signal (R-060): does this expression — a `for`-loop's
/// iterable — contain a `1 << n` shift (the canonical Rust power-of-two
/// mask generator)? Returns the rendered bound when it does, so the fact's
/// `detail` names exactly what sweeps (`"1 << n"`). Walks through ranges,
/// parens, and groups to the shift; a `1 << n` whose left operand is the
/// integer literal `1` is the signal, so `2 << n` or `n << 1` are not
/// (they are not power-of-two masks). Structural, not textual, so an
/// identifier ending in a digit (`buf1 << n`) never false-fires.
pub(crate) fn power_of_two_bound(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Binary(b) => {
            if let syn::BinOp::Shl(_) = b.op
                && let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(i),
                    ..
                }) = &*b.left
                && i.base10_parse::<u64>().ok() == Some(1)
            {
                return Some(expr.to_token_stream().to_string());
            }
            power_of_two_bound(&b.left).or_else(|| power_of_two_bound(&b.right))
        }
        syn::Expr::Group(g) => power_of_two_bound(&g.expr),
        syn::Expr::Paren(p) => power_of_two_bound(&p.expr),
        syn::Expr::Range(r) => r
            .start
            .as_deref()
            .and_then(power_of_two_bound)
            .or_else(|| r.end.as_deref().and_then(power_of_two_bound)),
        _ => None,
    }
}
