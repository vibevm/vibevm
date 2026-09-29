//! The Rust T-syn frontend (ENGINE-CONFORM §2): `syn` in-process, the
//! one-page-AST path. Emits the facts the Phase 4 rules consume —
//! items with attribute text, `use` imports, `<Type>::new` construction
//! sites, and `unsafe` uses. B5: an unparseable file yields zero facts
//! rather than an error; the rest of the tree still extracts.

specmark::scope!(
    "spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#frontends"
);

use conform_core::{Fact, Frontend};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;

use deviation::DeviationStack;

mod deviation;
mod out_of_line;
mod sweep;
pub use out_of_line::{
    apply_out_of_line_test_context, out_of_line_test_bodies, stamp_test_context,
};

/// The Rust T-syn [`Frontend`](conform_core::Frontend): parse a source
/// string into conform facts in-process.
///
/// `RustFrontend` is a zero-sized seam — construct it directly and call
/// [`extract`](conform_core::Frontend::extract). Every extraction opens
/// with the file's line metrics, then the tagged items in source order;
/// an unparseable file yields zero such facts (B5), never an error.
///
/// ```
/// use conform_core::Frontend;
/// use rust_ai_native_conform_frontend::RustFrontend;
///
/// let facts = RustFrontend.extract(
///     "lib.rs",
///     "demo",
///     "demo",
///     "pub fn answer() -> u32 { 42 }\n",
/// );
/// assert!(!facts.is_empty()); // at least the FileMetrics fact
/// assert_eq!(RustFrontend.id(), "rust-syn");
///
/// // Unparseable input is tolerated: zero facts, no panic.
/// assert!(RustFrontend.extract("x.rs", "demo", "demo", "fn (").is_empty());
/// ```
pub struct RustFrontend;

impl Frontend for RustFrontend {
    fn id(&self) -> &'static str {
        "rust-syn"
    }

    fn version(&self) -> &'static str {
        // Bump when extraction changes shape — the store key includes
        // it, so old cached facts are simply never read again.
        // v2: is_pub + has_doctest on Item; ErrorVariant facts.
        // v3: FileMetrics per file; UnwrapUse with cfg(test) scoping.
        // v4: UnwrapUse with fn-grain spec(deviates) scoping.
        // v5: UnsafeUse with the same test/deviates scoping, and
        //     unsafe impl methods extracted (they were invisible).
        // v6: EnvRead facts (env::var/var_os/set_var/remove_var) for the
        //     ambient-env rule, with the same test/deviates scoping.
        // v7: InvariantComment facts — a raw-text scan of the marker
        //     vocabulary (`SAFETY:` / `INVARIANT:` / …), since syn drops
        //     plain `//` comments. Feeds invariant-comment-position.
        // v8: the vocabulary narrows to the five labeled (colon-bearing)
        //     tags — a marker is a labeled tag, not a prose word. `SAFETY:`
        //     is dropped (a block-local `unsafe` justification, not a file
        //     invariant), and the bare words gain a colon (`MUST:` ≠ bare
        //     `MUST`), so the cache must retire.
        // v9: TestSweep facts — a swept test matrix (a `1 << n` bit-mask
        //     loop bound, or a ≥3-deep Cartesian nest of loops) in test
        //     context only, feeding declared-test-matrices (R-060).
        // v10: the nested-loops signal narrows — only a `for` loop with a
        //     RANGE iterable (`0..n`) counts toward the sweep depth; a
        //     collection/array/path iterable is a declared axis and no
        //     longer counts, and `while`/`loop` do not count either.
        //     Exhausting a closed set by nesting collection loops is now
        //     compliant, so the emitted fact set shrinks and the cache
        //     must retire.
        // v11: the `#[spec(deviates = …, reason = "…")]` second key is now
        //     extracted — `UnsafeUse`/`UnwrapUse`/`EnvRead` carry the reason
        //     text in their `reason` field (the engine threads it to the SARIF
        //     `justification`), so the fact shape changes and the cache
        //     must retire.
        "11"
    }

    fn extract(&self, _file: &str, _crate_name: &str, module: &str, text: &str) -> Vec<Fact> {
        let Ok(ast) = syn::parse_file(text) else {
            return Vec::new();
        };
        let mut v = Extractor {
            module: module.to_string(),
            facts: vec![Fact::FileMetrics {
                lines: text.lines().count() as u32,
            }],
            test_depth: 0,
            deviating: DeviationStack::default(),
            test_ranges: Vec::new(),
            range_loop_depth: 0,
        };
        v.visit_file(&ast);
        // Plain `//` line comments are dropped by syn::parse_file (only
        // `#[doc]` doc comments survive the AST), so the invariant-marker
        // census walks the raw `text` the AST was parsed from. Runs after
        // the visit so it can reuse the test-context line ranges the
        // visit collected.
        v.scan_invariant_comments(text);
        v.facts.sort_by_key(|f| match f {
            Fact::FileMetrics { .. } => 0,
            Fact::Item { line, .. }
            | Fact::Import { line, .. }
            | Fact::Ctor { line, .. }
            | Fact::UnsafeUse { line, .. }
            | Fact::ErrorVariant { line, .. }
            | Fact::UnwrapUse { line, .. }
            | Fact::EnvRead { line, .. }
            // Never produced by rust-syn — the ts-tsc and go frontends own
            // these, and the comment-walking invariant pass is not wired
            // here yet — but the sort is total over the shared fact model.
            | Fact::TsUnsafe { line, .. }
            | Fact::TsEnvRead { line, .. }
            | Fact::TsSeamError { line, .. }
            | Fact::GoUnsafe { line, .. }
            | Fact::GoConformance { line, .. }
            | Fact::InvariantComment { line, .. }
            | Fact::TestSweep { line, .. }
            // B-026: a foreign-linter diagnosis (SARIF ingest), sorted by
            // its line like every located fact. Never produced by rust-syn
            // — it arrives via sarif::load_reports, merged into the fact
            // stream by the driver after extraction — but the sort is total
            // over the shared fact model.
            | Fact::LintDiagnosis { line, .. } => *line,
        });
        v.facts
    }
}

struct Extractor {
    module: String,
    facts: Vec<Fact>,
    /// Nonzero while visiting a `#[cfg(test)]` module or `#[test]`
    /// fn — `UnwrapUse` facts inside carry `in_test: true`.
    test_depth: u32,
    /// The fn-grain deviation scope: a stack of each enclosing
    /// `#[spec(deviates = …)]` fn's reason, so the three deviation facts
    /// (`UnwrapUse` / `UnsafeUse` / `EnvRead`) inside carry `in_deviation`
    /// and the nearest reason. See [`deviation`] — fn-grain only: a
    /// deviates edge on an impl, struct, or mod records a different
    /// deviation (the solver-choice edges on `Sat` / `NaiveDepSolver`
    /// are the live counter-examples) and grants no amnesty.
    deviating: DeviationStack,
    /// `[start, end]` line ranges of `#[cfg(test)]` modules and `#[test]`
    /// free fns — the line-grain twin of `test_depth`, collected during
    /// the visit so the post-visit raw-text comment scan can stamp
    /// `in_test` on an invariant comment the AST never sees.
    test_ranges: Vec<(u32, u32)>,
    /// Nesting depth of RANGE loops (`for x in 0..n`) currently being
    /// visited — nonzero while inside a generated-axis for-loop. The
    /// declared-test-matrices signal (R-060): only a range iterable is a
    /// generated axis, so a Cartesian product of ≥ 3 nested RANGE loops in
    /// test context is a swept matrix; a `for x in [a, b]` / `for x in
    /// Stage::ALL` collection loop is a declared axis and never increments
    /// this.
    range_loop_depth: u32,
}

/// `#[cfg(test)]` / `#[cfg(any(test, ...))]` — the same shape the specmap
/// ratchet skips. `pub(crate)` so out-of-line detection reuses it verbatim.
pub(crate) fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        if !a.path().is_ident("cfg") {
            return false;
        }
        match &a.meta {
            syn::Meta::List(list) => list.tokens.to_string().contains("test"),
            _ => false,
        }
    })
}

/// `#[test]`, `#[tokio::test]`, and friends — the last path segment
/// is `test`.
fn is_test_fn(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().segments.last().is_some_and(|s| s.ident == "test"))
}

fn attr_text(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter_map(|a| {
            let last = a.path().segments.last()?.ident.to_string();
            match (&a.meta, last.as_str()) {
                (syn::Meta::List(list), "spec" | "cell" | "verifies") => {
                    Some(format!("{last}({})", list.tokens))
                }
                _ => None,
            }
        })
        .collect()
}

/// True when the item's doc comment carries a fenced code block — the
/// compiled-doctest signal Class G consumes. rustdoc treats a fence
/// with no language (or `rust`) as a doctest; `text`/`ignore` fences
/// are prose, but distinguishing them is the rule's refinement, not
/// the fact's — the fact records "a fence exists".
fn has_doc_fence(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        if !a.path().is_ident("doc") {
            return false;
        }
        if let syn::Meta::NameValue(nv) = &a.meta
            && let syn::Expr::Lit(lit) = &nv.value
            && let syn::Lit::Str(s) = &lit.lit
        {
            return s.value().trim_start().starts_with("```");
        }
        false
    })
}

fn is_pub(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn line_of(spanned: &impl Spanned) -> u32 {
    spanned.span().start().line as u32
}

/// The fixed invariant-marker vocabulary the frontend emits. The rule
/// re-checks the active config vocabulary, so the extractor emits
/// generously; each entry is the canonical spelling the config
/// dictionary uses. All five are colon-bearing labeled tags — a marker
/// is a labeled tag, not a prose word — so each is self-anchoring. The
/// word-boundary guard in [`invariant_marker`] stays as a
/// forward-compatible check for any future bare marker.
const INVARIANT_MARKERS: &[&str] = &["INVARIANT:", "WARNING:", "PANICS:", "MUST:", "NEVER:"];

/// The canonical invariant marker a comment line LEADS with, if any.
/// Detection is anchored at the comment's first content token (after the
/// `//` / `/*` / `*` introducer and whitespace): a marker not at the very
/// start of the comment is NOT detected. This matches the all-caps
/// section-header convention and — deliberately — does not flag prose:
/// every marker is a colon-bearing labeled tag, so a bare `must` /
/// `never` / `panics` mid-sentence (or even leading one) is not an
/// invariant declaration and does not fire.
///
/// **Recorded limit:** a marker embedded mid-comment (`// see INVARIANT:`
/// further down) is not seen; the convention puts the marker at the lead.
/// The match is case-sensitive to the config's canonical spelling, so
/// `// invariant:` (lowercase) is not detected — only the all-caps form
/// the guide's vocabulary uses.
fn invariant_marker(line: &str) -> Option<String> {
    let lead = line
        .trim_start_matches(['/', '*', '!', ' ', '\t'])
        .trim_start();
    for marker in INVARIANT_MARKERS {
        if let Some(rest) = lead.strip_prefix(marker) {
            let needs_boundary = !marker.ends_with(':');
            let boundary = rest
                .chars()
                .next()
                .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
            if !needs_boundary || boundary {
                return Some((*marker).to_string());
            }
        }
    }
    None
}

impl Extractor {
    /// Walk the raw file text for invariant-marker comments and emit one
    /// [`Fact::InvariantComment`] per marker comment, in source order. See
    /// [`INVARIANT_MARKERS`] / [`invariant_marker`] for the detection
    /// rule and its recorded limits. `in_test` is the line-grain twin of
    /// the visit's `test_depth`: a comment whose line falls inside a
    /// `#[cfg(test)]` module or `#[test]` fn (collected in
    /// [`Extractor::test_ranges`]) is test context, exactly as the
    /// item-level facts already record it.
    fn scan_invariant_comments(&mut self, text: &str) {
        let mut in_block = false;
        for (idx, raw) in text.lines().enumerate() {
            let line_no = (idx + 1) as u32;
            if !self.is_comment_line(raw, &mut in_block) {
                continue;
            }
            let Some(marker) = invariant_marker(raw) else {
                continue;
            };
            let in_test = self
                .test_ranges
                .iter()
                .any(|(start, end)| line_no >= *start && line_no <= *end);
            self.facts.push(Fact::InvariantComment {
                marker,
                line: line_no,
                in_test,
            });
        }
    }

    /// A line is a comment line for the marker census: a `//` line
    /// comment, a `/*` block line (opening, interior, or closing), or a
    /// line inside an open block. A line that merely CONTAINS a block
    /// opener inside code (`x(); /* … */ y()`) reads as a comment line,
    /// but `invariant_marker` anchors at the lead, so a marker after the
    /// code is not detected — the over-count costs nothing.
    ///
    /// **Recorded limit:** block comments are tracked one transition per
    /// line (a line that opens AND closes a block is comment-shaped for
    /// its whole length); a marker that shares a line with code before it
    /// is not detected.
    fn is_comment_line(&self, raw: &str, in_block: &mut bool) -> bool {
        if *in_block {
            if raw.contains("*/") {
                *in_block = false;
            }
            return true;
        }
        let trimmed = raw.trim_start();
        if trimmed.starts_with("//") {
            return true;
        }
        if let Some(open) = raw.find("/*") {
            if !raw[open..].contains("*/") {
                *in_block = true;
            }
            return true;
        }
        false
    }
}

impl<'ast> Visit<'ast> for Extractor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.facts.push(Fact::Item {
            kind: "fn".into(),
            symbol: format!("{}::{}", self.module, node.sig.ident),
            line: line_of(&node.sig.ident),
            attrs: attr_text(&node.attrs),
            is_pub: is_pub(&node.vis),
            has_doctest: has_doc_fence(&node.attrs),
        });
        let in_test = is_test_fn(&node.attrs) || is_cfg_test(&node.attrs);
        if in_test {
            self.test_depth += 1;
            self.test_ranges
                .push((line_of(node), node.span().end().line as u32));
        }
        let deviating = self.deviating.enter_if_deviates(&node.attrs);
        // The decl fact for an `unsafe fn` sees the fn's own test and
        // deviates attrs — push after the depths account for them.
        if node.sig.unsafety.is_some() {
            self.facts.push(Fact::UnsafeUse {
                context: format!("fn {}", node.sig.ident),
                line: line_of(&node.sig.ident),
                in_test: self.test_depth > 0,
                in_deviation: self.deviating.in_deviation(),
                reason: self.deviating.reason(),
            });
        }
        syn::visit::visit_item_fn(self, node);
        if deviating {
            self.deviating.leave();
        }
        if in_test {
            self.test_depth -= 1;
        }
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let deviating = self.deviating.enter_if_deviates(&node.attrs);
        // v5: an `unsafe fn` in an impl block is an unsafe use too —
        // until v4 these were invisible to the gate.
        if node.sig.unsafety.is_some() {
            self.facts.push(Fact::UnsafeUse {
                context: format!("fn {}", node.sig.ident),
                line: line_of(&node.sig.ident),
                in_test: self.test_depth > 0,
                in_deviation: self.deviating.in_deviation(),
                reason: self.deviating.reason(),
            });
        }
        syn::visit::visit_impl_item_fn(self, node);
        if deviating {
            self.deviating.leave();
        }
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        let in_test = is_cfg_test(&node.attrs);
        if in_test {
            self.test_depth += 1;
            self.test_ranges
                .push((line_of(node), node.span().end().line as u32));
        }
        syn::visit::visit_item_mod(self, node);
        if in_test {
            self.test_depth -= 1;
        }
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let m = node.method.to_string();
        if m == "unwrap" || m == "expect" {
            self.facts.push(Fact::UnwrapUse {
                method: m,
                line: line_of(&node.method),
                in_test: self.test_depth > 0,
                in_deviation: self.deviating.in_deviation(),
                reason: self.deviating.reason(),
            });
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.facts.push(Fact::Item {
            kind: "struct".into(),
            symbol: format!("{}::{}", self.module, node.ident),
            line: line_of(&node.ident),
            attrs: attr_text(&node.attrs),
            is_pub: is_pub(&node.vis),
            has_doctest: has_doc_fence(&node.attrs),
        });
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.facts.push(Fact::Item {
            kind: "enum".into(),
            symbol: format!("{}::{}", self.module, node.ident),
            line: line_of(&node.ident),
            attrs: attr_text(&node.attrs),
            is_pub: is_pub(&node.vis),
            has_doctest: has_doc_fence(&node.attrs),
        });
        // thiserror variants: #[error("...")] on each variant, the
        // enum's own attrs travel with every variant fact (Class F).
        let enum_attrs = attr_text(&node.attrs);
        for v in &node.variants {
            for a in &v.attrs {
                if !a.path().is_ident("error") {
                    continue;
                }
                let syn::Meta::List(list) = &a.meta else {
                    continue;
                };
                // First string literal in the error(...) tokens is the
                // display template; transparent variants have none.
                let message = list
                    .tokens
                    .clone()
                    .into_iter()
                    .find_map(|t| match t {
                        proc_macro2::TokenTree::Literal(l) => {
                            let s = l.to_string();
                            s.starts_with('"').then(|| s.trim_matches('"').to_string())
                        }
                        _ => None,
                    })
                    .unwrap_or_default();
                self.facts.push(Fact::ErrorVariant {
                    enum_symbol: format!("{}::{}", self.module, node.ident),
                    variant: v.ident.to_string(),
                    message,
                    line: line_of(&v.ident),
                    enum_attrs: enum_attrs.clone(),
                });
            }
        }
        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.facts.push(Fact::Item {
            kind: "trait".into(),
            symbol: format!("{}::{}", self.module, node.ident),
            line: line_of(&node.ident),
            attrs: attr_text(&node.attrs),
            is_pub: is_pub(&node.vis),
            has_doctest: has_doc_fence(&node.attrs),
        });
        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        let rendered = node
            .tree
            .to_token_stream()
            .to_string()
            .replace(' ', "")
            .replace(",{", ", {");
        self.facts.push(Fact::Import {
            from_module: self.module.clone(),
            to_path: rendered,
            line: line_of(node),
        });
        syn::visit::visit_item_use(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(p) = node.func.as_ref() {
            let segs: Vec<String> = p
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect();
            if segs.len() >= 2 && segs.last().map(String::as_str) == Some("new") {
                self.facts.push(Fact::Ctor {
                    type_name: segs[segs.len() - 2].clone(),
                    line: line_of(node),
                });
            }
            // `env::{var,var_os,set_var,remove_var}` — the ambient-env
            // signal. Matches `std::env::var(...)` and `env::var(...)` by
            // the trailing `env::<method>` shape; carries the same
            // test/deviates scoping as `UnwrapUse`.
            if segs.len() >= 2
                && segs[segs.len() - 2] == "env"
                && matches!(
                    segs[segs.len() - 1].as_str(),
                    "var" | "var_os" | "set_var" | "remove_var"
                )
            {
                self.facts.push(Fact::EnvRead {
                    method: segs[segs.len() - 1].clone(),
                    line: line_of(node),
                    in_test: self.test_depth > 0,
                    in_deviation: self.deviating.in_deviation(),
                    reason: self.deviating.reason(),
                });
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        self.facts.push(Fact::UnsafeUse {
            context: "block".into(),
            line: line_of(node),
            in_test: self.test_depth > 0,
            in_deviation: self.deviating.in_deviation(),
            reason: self.deviating.reason(),
        });
        syn::visit::visit_expr_unsafe(self, node);
    }

    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        // A bit-mask bound is a `for`-loop signal only — `while`/`loop`
        // have no iterable. Checked for every for-loop, regardless of
        // iterable kind: a `2^n` bound only makes sense on a counted loop,
        // and `0..(1 << n)` is itself a range iterable.
        if self.test_depth > 0
            && let Some(bound) = sweep::power_of_two_bound(&node.expr)
        {
            self.facts.push(Fact::TestSweep {
                kind: "bitmask".into(),
                line: line_of(node),
                detail: bound,
            });
        }
        // Only a RANGE iterable (`0..n`) is a generated axis that counts
        // toward the swept-matrix depth; a collection/array/path iterable
        // (`[a, b]`, `Stage::ALL`, `vec.iter()`) is a DECLARED axis — its
        // cases are written as data — and does not count, no matter how
        // deeply nested. `while`/`loop` carry no iterable at all and are
        // left to the default Visit (they do not count either: see
        // [`sweep::is_range_iterable`]'s recorded limit).
        if sweep::is_range_iterable(&node.expr) {
            self.enter_loop(line_of(node));
            syn::visit::visit_expr_for_loop(self, node);
            self.range_loop_depth -= 1;
        } else {
            syn::visit::visit_expr_for_loop(self, node);
        }
    }
}

#[cfg(test)]
#[path = "lib/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "lib/deviation_tests.rs"]
mod deviation_tests;
